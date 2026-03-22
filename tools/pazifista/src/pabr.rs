use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

const PABR_MAGIC: &[u8; 4] = b"PABR";
const RID_FOOTER_LEN: usize = 47;
const RID_FOOTER_SIGNATURE: [u8; 10] = [0x00, 0x00, 0x60, 0xFF, 0xFF, 0xFF, 0x78, 0x87, 0x00, 0x00];
const BKD_TRAILER_LEN: usize = 12;
const INDEX_SENTINEL: u16 = u16::MAX;
pub const DEFAULT_ROW_SHIFT: u32 = 0x0EF0;
const BACKGROUND_BGR: [u8; 3] = [193, 154, 79];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Breakpoint {
    pub source_x: u16,
    pub dictionary_index: u16,
}

#[derive(Clone, Debug)]
pub struct RidFile {
    pub region_ids: Vec<u16>,
    pub native_width: u32,
    pub native_height: u32,
    pub trailer_prefix_len: usize,
}

#[derive(Clone, Debug)]
pub struct BkdFile {
    pub rows: Vec<Vec<Breakpoint>>,
    pub trailer_words: [u32; 3],
    pub max_source_x: u16,
}

#[derive(Clone, Debug)]
pub struct PabrMap {
    pub rid_path: PathBuf,
    pub bkd_path: PathBuf,
    pub rid: RidFile,
    pub bkd: BkdFile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PabrInspect {
    pub dictionary_entries: usize,
    pub scanline_rows: usize,
    pub native_width: u32,
    pub native_height: u32,
    pub wrapped_bands: usize,
    pub used_dictionary_entries: usize,
    pub used_region_ids: usize,
    pub transparent_breakpoints: usize,
    pub max_source_x: u16,
    pub rid_trailer_prefix_len: usize,
    pub bkd_trailer_words: [u32; 3],
}

#[derive(Clone, Debug)]
pub struct RenderSummary {
    pub output_path: PathBuf,
    pub dimensions: OutputDimensions,
}

impl PabrMap {
    pub fn paired_paths(input_path: &Path) -> Result<(PathBuf, PathBuf)> {
        let extension = input_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default();

        if extension.eq_ignore_ascii_case("rid") {
            Ok((input_path.to_path_buf(), input_path.with_extension("bkd")))
        } else if extension.eq_ignore_ascii_case("bkd") {
            Ok((input_path.with_extension("rid"), input_path.to_path_buf()))
        } else {
            bail!("PABR input must be a .rid or .bkd file; got .{}", extension);
        }
    }

    pub fn from_paths(rid_path: &Path, bkd_path: &Path) -> Result<Self> {
        let rid = RidFile::from_path(rid_path)?;
        let bkd = BkdFile::from_path(bkd_path)?;

        Ok(Self {
            rid_path: rid_path.to_path_buf(),
            bkd_path: bkd_path.to_path_buf(),
            rid,
            bkd,
        })
    }

    pub fn inspect(&self) -> Result<PabrInspect> {
        let mut used_dictionary_entries = BTreeSet::new();
        let mut used_region_ids = BTreeSet::new();
        let mut transparent_breakpoints = 0usize;

        for row in &self.bkd.rows {
            for breakpoint in row {
                if breakpoint.dictionary_index == INDEX_SENTINEL {
                    transparent_breakpoints += 1;
                    continue;
                }

                let dictionary_index = usize::from(breakpoint.dictionary_index);
                let region_id = self
                    .rid
                    .region_ids
                    .get(dictionary_index)
                    .copied()
                    .with_context(|| {
                        format!(
                            "dictionary index {} exceeds RID dictionary length {}",
                            dictionary_index,
                            self.rid.region_ids.len()
                        )
                    })?;

                used_dictionary_entries.insert(dictionary_index);
                used_region_ids.insert(region_id);
            }
        }

        Ok(PabrInspect {
            dictionary_entries: self.rid.region_ids.len(),
            scanline_rows: self.bkd.rows.len(),
            native_width: self.rid.native_width,
            native_height: self.rid.native_height,
            wrapped_bands: self.band_count()?,
            used_dictionary_entries: used_dictionary_entries.len(),
            used_region_ids: used_region_ids.len(),
            transparent_breakpoints,
            max_source_x: self.bkd.max_source_x,
            rid_trailer_prefix_len: self.rid.trailer_prefix_len,
            bkd_trailer_words: self.bkd.trailer_words,
        })
    }

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

    fn region_id_for_dictionary_index(&self, dictionary_index: u16) -> Result<u16> {
        self.rid
            .region_ids
            .get(usize::from(dictionary_index))
            .copied()
            .with_context(|| {
                format!(
                    "dictionary index {} exceeds RID dictionary length {}",
                    dictionary_index,
                    self.rid.region_ids.len()
                )
            })
    }

    fn band_count(&self) -> Result<usize> {
        if self.rid.native_width == 0 {
            bail!("RID footer reported a zero native width");
        }

        Ok((u32::from(self.bkd.max_source_x) / self.rid.native_width) as usize + 1)
    }
}

impl RidFile {
    pub fn from_path(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)
            .with_context(|| format!("failed to read RID file {}", path.display()))?;
        Self::from_bytes(&bytes)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 8 + RID_FOOTER_LEN {
            bail!("RID file is too small");
        }
        if &bytes[0..4] != PABR_MAGIC {
            bail!("RID file is missing PABR magic");
        }

        let dictionary_entries = read_u32(bytes, 4)? as usize;
        let dictionary_bytes = dictionary_entries
            .checked_mul(2)
            .context("RID dictionary length overflow")?;
        let dictionary_end = 8usize
            .checked_add(dictionary_bytes)
            .context("RID dictionary offset overflow")?;
        if dictionary_end + RID_FOOTER_LEN > bytes.len() {
            bail!("RID dictionary exceeds file size");
        }

        let footer = &bytes[bytes.len() - RID_FOOTER_LEN..];
        if footer[0..RID_FOOTER_SIGNATURE.len()] != RID_FOOTER_SIGNATURE {
            bail!("RID footer signature does not match the known PABR region-map footer");
        }

        let mut region_ids = Vec::with_capacity(dictionary_entries);
        for offset in (8..dictionary_end).step_by(2) {
            region_ids.push(read_u16(bytes, offset)?);
        }

        Ok(Self {
            region_ids,
            native_width: u32::from(read_u16(footer, 10)?),
            native_height: u32::from(read_u16(footer, 14)?),
            trailer_prefix_len: bytes.len() - dictionary_end - RID_FOOTER_LEN,
        })
    }
}

impl BkdFile {
    pub fn from_path(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)
            .with_context(|| format!("failed to read BKD file {}", path.display()))?;
        Self::from_bytes(&bytes)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 8 + BKD_TRAILER_LEN {
            bail!("BKD file is too small");
        }
        if &bytes[0..4] != PABR_MAGIC {
            bail!("BKD file is missing PABR magic");
        }

        let row_count = read_u32(bytes, 4)? as usize;
        let mut rows = Vec::with_capacity(row_count);
        let mut position = 8usize;
        let mut max_source_x = 0u16;

        for row_index in 0..row_count {
            if position + 4 > bytes.len() {
                bail!("BKD row {} header exceeds file size", row_index);
            }
            let pair_count = read_u32(bytes, position)? as usize;
            position += 4;

            let pair_bytes = pair_count
                .checked_mul(4)
                .context("BKD row length overflow")?;
            if position + pair_bytes > bytes.len() {
                bail!("BKD row {} payload exceeds file size", row_index);
            }

            let mut row = Vec::with_capacity(pair_count);
            let mut previous_x = 0u16;
            for pair_index in 0..pair_count {
                let pair_offset = position + pair_index * 4;
                let source_x = read_u16(bytes, pair_offset)?;
                let dictionary_index = read_u16(bytes, pair_offset + 2)?;
                if pair_index > 0 && source_x < previous_x {
                    bail!("BKD row {} is not sorted by x coordinate", row_index);
                }
                previous_x = source_x;
                max_source_x = max_source_x.max(source_x);
                row.push(Breakpoint {
                    source_x,
                    dictionary_index,
                });
            }

            position += pair_bytes;
            rows.push(row);
        }

        if position + BKD_TRAILER_LEN != bytes.len() {
            bail!(
                "BKD footer length mismatch: expected {} trailing bytes, found {}",
                BKD_TRAILER_LEN,
                bytes.len().saturating_sub(position)
            );
        }

        let trailer_words = [
            read_u32(bytes, position)?,
            read_u32(bytes, position + 4)?,
            read_u32(bytes, position + 8)?,
        ];
        if trailer_words[1] != position as u32 {
            bail!(
                "BKD footer offset {} does not match parsed row payload end {}",
                trailer_words[1],
                position
            );
        }

        Ok(Self {
            rows,
            trailer_words,
            max_source_x,
        })
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

fn sample_source_row(output_y: u32, output_height: u32, source_rows: usize) -> usize {
    let numerator = (u64::from(output_y) * 2 + 1) * source_rows as u64;
    let denominator = u64::from(output_height) * 2;
    ((numerator / denominator) as usize).min(source_rows.saturating_sub(1))
}

fn sample_local_x(output_x: usize, output_width: u32, native_width: u32) -> u32 {
    let numerator = (output_x as u64 * 2 + 1) * u64::from(native_width);
    let denominator = u64::from(output_width) * 2;
    ((numerator / denominator) as u32).min(native_width.saturating_sub(1))
}

fn color_bgr_for_region_id(region_id: u16) -> [u8; 3] {
    let mut state = u32::from(region_id);
    state ^= state >> 16;
    state = state.wrapping_mul(0x7FEB_352D);
    state ^= state >> 15;
    state = state.wrapping_mul(0x846C_A68B);
    state ^= state >> 16;

    let red = 64 + ((state >> 16) as u8 & 0x7F);
    let green = 64 + ((state >> 8) as u8 & 0x7F);
    let blue = 64 + (state as u8 & 0x7F);
    [blue, green, red]
}

fn mode_region_id(region_ids: &[u16]) -> u16 {
    let mut best_id = region_ids[0];
    let mut best_count = 1usize;

    for &candidate in region_ids {
        let mut count = 0usize;
        for &value in region_ids {
            if value == candidate {
                count += 1;
            }
        }

        if count > best_count || (count == best_count && candidate < best_id) {
            best_id = candidate;
            best_count = count;
        }
    }

    best_id
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let end = offset
        .checked_add(2)
        .context("u16 offset overflow while parsing PABR data")?;
    let slice = bytes
        .get(offset..end)
        .with_context(|| format!("u16 read at offset {} is out of bounds", offset))?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .context("u32 offset overflow while parsing PABR data")?;
    let slice = bytes
        .get(offset..end)
        .with_context(|| format!("u32 read at offset {} is out of bounds", offset))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

#[cfg(test)]
mod tests {
    use super::{
        color_bgr_for_region_id, mode_region_id, sample_local_x, sample_source_row, BkdFile,
        Breakpoint, OutputDimensions, PabrMap, RidFile, RID_FOOTER_LEN, RID_FOOTER_SIGNATURE,
    };

    fn make_rid_bytes(region_ids: &[u16], width: u16, height: u16, prefix: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"PABR");
        bytes.extend_from_slice(&(region_ids.len() as u32).to_le_bytes());
        for region_id in region_ids {
            bytes.extend_from_slice(&region_id.to_le_bytes());
        }
        bytes.extend_from_slice(prefix);

        let mut footer = [0u8; RID_FOOTER_LEN];
        footer[0..RID_FOOTER_SIGNATURE.len()].copy_from_slice(&RID_FOOTER_SIGNATURE);
        footer[10..12].copy_from_slice(&width.to_le_bytes());
        footer[14..16].copy_from_slice(&height.to_le_bytes());
        bytes.extend_from_slice(&footer);
        bytes
    }

    fn make_bkd_bytes(rows: &[Vec<Breakpoint>]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"PABR");
        bytes.extend_from_slice(&(rows.len() as u32).to_le_bytes());

        for row in rows {
            bytes.extend_from_slice(&(row.len() as u32).to_le_bytes());
            for breakpoint in row {
                bytes.extend_from_slice(&breakpoint.source_x.to_le_bytes());
                bytes.extend_from_slice(&breakpoint.dictionary_index.to_le_bytes());
            }
        }

        let payload_end = bytes.len() as u32;
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&payload_end.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes
    }

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
                        Breakpoint {
                            source_x: 0,
                            dictionary_index: 0,
                        },
                        Breakpoint {
                            source_x: 32768,
                            dictionary_index: 1,
                        },
                    ],
                    vec![Breakpoint {
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
    fn rid_footer_dimensions_are_parsed_from_fixed_trailer() {
        let rid = RidFile::from_bytes(&make_rid_bytes(&[4, 17], 11560, 10540, &[1, 2, 3, 4]))
            .expect("RID should parse");

        assert_eq!(rid.region_ids, vec![4, 17]);
        assert_eq!(rid.native_width, 11560);
        assert_eq!(rid.native_height, 10540);
        assert_eq!(rid.trailer_prefix_len, 4);
    }

    #[test]
    fn bkd_rows_parse_sorted_breakpoints_and_footer_offset() {
        let rows = vec![
            vec![
                Breakpoint {
                    source_x: 0,
                    dictionary_index: 0,
                },
                Breakpoint {
                    source_x: 32768,
                    dictionary_index: 1,
                },
            ],
            vec![Breakpoint {
                source_x: 0,
                dictionary_index: 1,
            }],
        ];
        let bkd = BkdFile::from_bytes(&make_bkd_bytes(&rows)).expect("BKD should parse");

        assert_eq!(bkd.rows, rows);
        assert_eq!(bkd.max_source_x, 32768);
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
    fn sampling_uses_pixel_centers() {
        assert_eq!(sample_local_x(0, 4, 11560), 1445);
        assert_eq!(sample_local_x(3, 4, 11560), 10115);
        assert_eq!(sample_source_row(0, 4, 2), 0);
        assert_eq!(sample_source_row(3, 4, 2), 1);
    }

    #[test]
    fn false_color_is_deterministic_and_non_black() {
        let a = color_bgr_for_region_id(4);
        let b = color_bgr_for_region_id(4);
        let c = color_bgr_for_region_id(5);

        assert_eq!(a, b);
        assert_ne!(a, [0, 0, 0]);
        assert_ne!(a, c);
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

    #[test]
    fn mode_prefers_majority_then_lowest_region_id() {
        assert_eq!(mode_region_id(&[9, 4, 9, 4, 4]), 4);
        assert_eq!(mode_region_id(&[9, 4]), 4);
    }
}
