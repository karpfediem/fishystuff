use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{bail, Context, Result};

use super::{OutputDimensions, PabrMap, RenderSummary, BACKGROUND_BGR, INDEX_SENTINEL};
use crate::pabr::util::{
    color_bgr_for_region_id, mode_region_id, sample_local_x, sample_source_row,
};

impl PabrMap {
    pub fn resolve_output_dimensions(
        &self,
        width: Option<u32>,
        height: Option<u32>,
        scale: Option<f32>,
    ) -> Result<OutputDimensions> {
        if let Some(scale) = scale {
            if width.is_some() || height.is_some() {
                bail!("--scale cannot be combined with --width or --height");
            }
            if !scale.is_finite() || scale <= 0.0 {
                bail!("--scale must be a finite positive number");
            }

            return Ok(OutputDimensions {
                width: scaled_dimension(self.rid.native_width, scale),
                height: scaled_dimension(self.rid.native_height, scale),
            });
        }

        match (width, height) {
            (Some(width), Some(height)) => Ok(OutputDimensions { width, height }),
            (Some(width), None) => Ok(OutputDimensions {
                width,
                height: preserve_aspect(width, self.rid.native_width, self.rid.native_height),
            }),
            (None, Some(height)) => Ok(OutputDimensions {
                width: preserve_aspect(height, self.rid.native_height, self.rid.native_width),
                height,
            }),
            (None, None) => Ok(OutputDimensions {
                width: self.rid.native_width,
                height: self.rid.native_height,
            }),
        }
    }

    pub fn render_bmp(
        &self,
        output_path: &Path,
        dimensions: OutputDimensions,
        row_shift: u32,
    ) -> Result<RenderSummary> {
        if dimensions.width == 0 || dimensions.height == 0 {
            bail!("output dimensions must be greater than zero");
        }

        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create output directory {}", parent.display())
                })?;
            }
        }

        let file = File::create(output_path)
            .with_context(|| format!("failed to create {}", output_path.display()))?;
        let mut writer = BufWriter::new(file);

        let row_padding = (4 - (dimensions.width * 3) % 4) % 4;
        let row_stride = dimensions.width as usize * 3 + row_padding as usize;
        let image_size = u64::from(row_stride as u32) * u64::from(dimensions.height);
        let file_size = 54u64 + image_size;

        if file_size > u64::from(u32::MAX) {
            bail!(
                "BMP output would be too large for the format header: {} bytes",
                file_size
            );
        }

        write_bmp_header(&mut writer, dimensions, image_size as u32, file_size as u32)?;

        let mut cached_source_row = usize::MAX;
        let mut cached_pixels = vec![0u8; row_stride];

        for output_y in (0..dimensions.height).rev() {
            let source_row = sample_source_row(output_y, dimensions.height, self.bkd.rows.len());
            if source_row != cached_source_row {
                self.render_scanline_row(
                    source_row,
                    dimensions.width,
                    row_shift,
                    &mut cached_pixels,
                )?;
                cached_source_row = source_row;
            }
            writer
                .write_all(&cached_pixels)
                .with_context(|| format!("failed writing {}", output_path.display()))?;
        }

        writer
            .flush()
            .with_context(|| format!("failed flushing {}", output_path.display()))?;

        Ok(RenderSummary {
            output_path: output_path.to_path_buf(),
            dimensions,
        })
    }

    fn render_scanline_row(
        &self,
        source_row: usize,
        output_width: u32,
        row_shift: u32,
        row_buffer: &mut [u8],
    ) -> Result<()> {
        let row = self
            .bkd
            .rows
            .get(source_row)
            .with_context(|| format!("source row {} is out of bounds", source_row))?;
        let band_count = self.band_count()?;
        let native_width = self.rid.native_width;
        let row_offset =
            ((source_row as u64 * u64::from(row_shift)) % u64::from(native_width)) as u32;

        let pixel_bytes = output_width as usize * 3;
        row_buffer.fill(0);

        if row.is_empty() {
            return Ok(());
        }

        let mut band_positions = vec![0usize; band_count];
        let mut folded_region_ids = Vec::with_capacity(band_count);
        for output_x in 0..output_width as usize {
            let local_x = sample_local_x(output_x, output_width, native_width);
            folded_region_ids.clear();

            for (band, band_position) in band_positions.iter_mut().enumerate() {
                let global_x = local_x + row_offset + band as u32 * native_width;
                if global_x > u32::from(u16::MAX) {
                    continue;
                }

                while *band_position + 1 < row.len()
                    && u32::from(row[*band_position + 1].source_x) <= global_x
                {
                    *band_position += 1;
                }

                let dictionary_index = row[*band_position].dictionary_index;
                if dictionary_index == INDEX_SENTINEL {
                    continue;
                }
                folded_region_ids.push(self.region_id_for_dictionary_index(dictionary_index)?);
            }

            let color = if folded_region_ids.is_empty() {
                BACKGROUND_BGR
            } else {
                color_bgr_for_region_id(mode_region_id(&folded_region_ids))
            };
            let pixel_offset = output_x * 3;
            row_buffer[pixel_offset..pixel_offset + 3].copy_from_slice(&color);
        }

        row_buffer[pixel_bytes..].fill(0);
        Ok(())
    }
}

fn write_bmp_header<W: Write>(
    writer: &mut W,
    dimensions: OutputDimensions,
    image_size: u32,
    file_size: u32,
) -> Result<()> {
    writer.write_all(b"BM")?;
    writer.write_all(&file_size.to_le_bytes())?;
    writer.write_all(&[0, 0, 0, 0])?;
    writer.write_all(&54u32.to_le_bytes())?;
    writer.write_all(&40u32.to_le_bytes())?;
    writer.write_all(&(dimensions.width as i32).to_le_bytes())?;
    writer.write_all(&(dimensions.height as i32).to_le_bytes())?;
    writer.write_all(&1u16.to_le_bytes())?;
    writer.write_all(&24u16.to_le_bytes())?;
    writer.write_all(&0u32.to_le_bytes())?;
    writer.write_all(&image_size.to_le_bytes())?;
    writer.write_all(&2835i32.to_le_bytes())?;
    writer.write_all(&2835i32.to_le_bytes())?;
    writer.write_all(&0u32.to_le_bytes())?;
    writer.write_all(&0u32.to_le_bytes())?;
    Ok(())
}

fn preserve_aspect(target: u32, from: u32, to: u32) -> u32 {
    let scaled = (u64::from(target) * u64::from(to) + u64::from(from) / 2) / u64::from(from);
    scaled.max(1) as u32
}

fn scaled_dimension(native: u32, scale: f32) -> u32 {
    ((native as f64 * f64::from(scale)).round() as u32).max(1)
}

#[cfg(test)]
mod tests {
    use super::{OutputDimensions, PabrMap};
    use crate::pabr::{BkdFile, RidFile};

    fn build_test_map() -> PabrMap {
        PabrMap {
            rid_path: "test.rid".into(),
            bkd_path: "test.bkd".into(),
            rid: RidFile {
                region_ids: vec![4, 17],
                native_width: 11560,
                native_height: 10540,
                trailer_prefix_len: 4,
            },
            bkd: BkdFile {
                rows: vec![
                    vec![
                        crate::pabr::Breakpoint {
                            source_x: 0,
                            dictionary_index: 0,
                        },
                        crate::pabr::Breakpoint {
                            source_x: 32768,
                            dictionary_index: 1,
                        },
                    ],
                    vec![crate::pabr::Breakpoint {
                        source_x: 0,
                        dictionary_index: 1,
                    }],
                ],
                trailer_words: [0, 24, 0],
                max_source_x: 32768,
            },
        }
    }

    #[test]
    fn output_dimensions_preserve_native_aspect() {
        let map = build_test_map();

        assert_eq!(
            map.resolve_output_dimensions(Some(2048), None, None)
                .expect("width-only sizing should work"),
            OutputDimensions {
                width: 2048,
                height: 1867,
            }
        );
        assert_eq!(
            map.resolve_output_dimensions(None, Some(1860), None)
                .expect("height-only sizing should work"),
            OutputDimensions {
                width: 2040,
                height: 1860,
            }
        );
        assert_eq!(
            map.resolve_output_dimensions(None, None, Some(0.5))
                .expect("scaled sizing should work"),
            OutputDimensions {
                width: 5780,
                height: 5270,
            }
        );
    }

    #[test]
    fn explicit_dimensions_are_returned_directly() {
        let dimensions = OutputDimensions {
            width: 128,
            height: 64,
        };
        assert_eq!(dimensions.width, 128);
        assert_eq!(dimensions.height, 64);
    }
}
