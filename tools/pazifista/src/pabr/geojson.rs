use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_json::{Map, Value};

use super::{
    FieldExportSummary, GeoJsonExportSummary, PabrMap, RegionGroupMapping, RowSegment,
    INDEX_SENTINEL,
};
use crate::pabr::util::{color_rgb_for_value, mode_value, sample_source_row};
use fishystuff_core::field::{DiscreteFieldRows, FieldRowSpan};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rectangle {
    start_x: u32,
    end_x: u32,
    start_y: u32,
    end_y: u32,
}

#[derive(Debug, Clone, Serialize)]
struct GeoJsonFeatureCollection {
    #[serde(rename = "type")]
    collection_type: &'static str,
    features: Vec<GeoJsonFeature>,
}

#[derive(Debug, Clone, Serialize)]
struct GeoJsonFeature {
    #[serde(rename = "type")]
    feature_type: &'static str,
    properties: Map<String, Value>,
    geometry: GeoJsonGeometry,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "coordinates")]
enum GeoJsonGeometry {
    Polygon(Vec<Vec<[f64; 2]>>),
    MultiPolygon(Vec<Vec<Vec<[f64; 2]>>>),
}

impl PabrMap {
    pub fn export_regions_geojson(
        &self,
        output_path: &Path,
        row_shift: u32,
    ) -> Result<GeoJsonExportSummary> {
        let rectangles =
            self.build_rectangles(row_shift, |region_id| Some(u32::from(region_id)))?;
        self.write_regions_geojson(output_path, rectangles)
    }

    pub fn export_region_groups_geojson(
        &self,
        output_path: &Path,
        row_shift: u32,
        mapping: &RegionGroupMapping,
    ) -> Result<GeoJsonExportSummary> {
        let rectangles = self.build_rectangles(row_shift, |region_id| {
            mapping.region_group_for_region(region_id).map(u32::from)
        })?;
        self.write_region_groups_geojson(output_path, rectangles, mapping)
    }

    pub fn export_regions_field(
        &self,
        output_path: &Path,
        row_shift: u32,
    ) -> Result<FieldExportSummary> {
        let field = self.build_field_rows(row_shift, |region_id| Some(u32::from(region_id)))?;
        write_field(output_path, &field)
    }

    pub fn export_region_groups_field(
        &self,
        output_path: &Path,
        row_shift: u32,
        mapping: &RegionGroupMapping,
    ) -> Result<FieldExportSummary> {
        let field = self.build_field_rows(row_shift, |region_id| {
            mapping.region_group_for_region(region_id).map(u32::from)
        })?;
        write_field(output_path, &field)
    }

    pub(crate) fn decoded_row_segments<F>(
        &self,
        source_row: usize,
        row_shift: u32,
        mapper: &F,
    ) -> Result<Vec<RowSegment>>
    where
        F: Fn(u16) -> Option<u32>,
    {
        let row = self
            .bkd
            .rows
            .get(source_row)
            .with_context(|| format!("source row {} is out of bounds", source_row))?;
        if row.is_empty() {
            return Ok(Vec::new());
        }

        let band_count = self.band_count()?;
        let native_width = self.rid.native_width;
        let row_offset =
            ((source_row as u64 * u64::from(row_shift)) % u64::from(native_width)) as u32;

        let mut band_positions = vec![0usize; band_count];
        let mut folded_values = Vec::with_capacity(band_count);
        let mut segments = Vec::new();
        let mut current_value: Option<u32> = None;
        let mut run_start_x = 0u32;

        for local_x in 0..native_width {
            folded_values.clear();

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
                let region_id = self.region_id_for_dictionary_index(dictionary_index)?;
                if let Some(value) = mapper(region_id) {
                    folded_values.push(value);
                }
            }

            let next_value = if folded_values.is_empty() {
                None
            } else {
                Some(mode_value(&folded_values))
            };

            if next_value != current_value {
                if let Some(value) = current_value {
                    segments.push(RowSegment {
                        start_x: run_start_x,
                        end_x: local_x,
                        value,
                    });
                }
                current_value = next_value;
                run_start_x = local_x;
            }
        }

        if let Some(value) = current_value {
            segments.push(RowSegment {
                start_x: run_start_x,
                end_x: native_width,
                value,
            });
        }

        Ok(segments)
    }

    fn write_regions_geojson(
        &self,
        output_path: &Path,
        rectangles: BTreeMap<u32, Vec<Rectangle>>,
    ) -> Result<GeoJsonExportSummary> {
        let mut features = Vec::with_capacity(rectangles.len());
        let mut rectangle_count = 0usize;

        for (region_id, feature_rectangles) in rectangles {
            rectangle_count += feature_rectangles.len();
            let mut properties = Map::new();
            properties.insert("id".to_string(), Value::from(region_id));
            properties.insert("r".to_string(), Value::from(region_id));
            properties.insert(
                "c".to_string(),
                Value::Array(
                    color_rgb_for_value(region_id)
                        .into_iter()
                        .map(Value::from)
                        .collect(),
                ),
            );

            features.push(GeoJsonFeature {
                feature_type: "Feature",
                properties,
                geometry: rectangles_to_geometry(&feature_rectangles),
            });
        }

        write_geojson(output_path, &features)?;
        Ok(GeoJsonExportSummary {
            output_path: output_path.to_path_buf(),
            feature_count: features.len(),
            rectangle_count,
        })
    }

    fn write_region_groups_geojson(
        &self,
        output_path: &Path,
        rectangles: BTreeMap<u32, Vec<Rectangle>>,
        mapping: &RegionGroupMapping,
    ) -> Result<GeoJsonExportSummary> {
        let mut features = Vec::with_capacity(rectangles.len());
        let mut rectangle_count = 0usize;

        for (group_id, feature_rectangles) in rectangles {
            rectangle_count += feature_rectangles.len();
            let group_id_u16 =
                u16::try_from(group_id).context("region-group id exceeds u16 during export")?;
            let mut properties = Map::new();
            properties.insert("id".to_string(), Value::from(group_id));
            properties.insert("rg".to_string(), Value::from(group_id));
            properties.insert(
                "c".to_string(),
                Value::Array(
                    color_rgb_for_value(group_id)
                        .into_iter()
                        .map(Value::from)
                        .collect(),
                ),
            );
            properties.insert(
                "rs".to_string(),
                Value::Array(
                    mapping
                        .region_ids_for_group(group_id_u16)
                        .iter()
                        .copied()
                        .map(|region_id| Value::from(u32::from(region_id)))
                        .collect(),
                ),
            );

            features.push(GeoJsonFeature {
                feature_type: "Feature",
                properties,
                geometry: rectangles_to_geometry(&feature_rectangles),
            });
        }

        write_geojson(output_path, &features)?;
        Ok(GeoJsonExportSummary {
            output_path: output_path.to_path_buf(),
            feature_count: features.len(),
            rectangle_count,
        })
    }

    fn build_rectangles<F>(
        &self,
        row_shift: u32,
        mapper: F,
    ) -> Result<BTreeMap<u32, Vec<Rectangle>>>
    where
        F: Fn(u16) -> Option<u32>,
    {
        if self.rid.native_width == 0 || self.rid.native_height == 0 {
            bail!("PABR export requires non-zero native dimensions");
        }
        if self.bkd.rows.is_empty() {
            bail!("PABR export requires at least one BKD row");
        }

        let mut rectangles_by_value: BTreeMap<u32, Vec<Rectangle>> = BTreeMap::new();
        let mut active_segments: BTreeMap<RowSegment, u32> = BTreeMap::new();
        let mut cached_source_row = usize::MAX;
        let mut cached_segments = Vec::new();

        for output_y in 0..self.rid.native_height {
            let source_row =
                sample_source_row(output_y, self.rid.native_height, self.bkd.rows.len());
            if source_row != cached_source_row {
                cached_segments = self.decoded_row_segments(source_row, row_shift, &mapper)?;
                cached_source_row = source_row;
            }

            let mut next_active = BTreeMap::new();
            for &segment in &cached_segments {
                let start_y = active_segments.remove(&segment).unwrap_or(output_y);
                next_active.insert(segment, start_y);
            }

            for (segment, start_y) in active_segments {
                rectangles_by_value
                    .entry(segment.value)
                    .or_default()
                    .push(Rectangle {
                        start_x: segment.start_x,
                        end_x: segment.end_x,
                        start_y,
                        end_y: output_y,
                    });
            }
            active_segments = next_active;
        }

        for (segment, start_y) in active_segments {
            rectangles_by_value
                .entry(segment.value)
                .or_default()
                .push(Rectangle {
                    start_x: segment.start_x,
                    end_x: segment.end_x,
                    start_y,
                    end_y: self.rid.native_height,
                });
        }

        Ok(rectangles_by_value)
    }

    fn build_field_rows<F>(&self, row_shift: u32, mapper: F) -> Result<DiscreteFieldRows>
    where
        F: Fn(u16) -> Option<u32>,
    {
        if self.rid.native_width == 0 || self.rid.native_height == 0 {
            bail!("PABR export requires non-zero native dimensions");
        }
        if self.bkd.rows.is_empty() {
            bail!("PABR export requires at least one BKD row");
        }

        let mut rows = Vec::with_capacity(self.rid.native_height as usize);
        let mut cached_source_row = usize::MAX;
        let mut cached_segments = Vec::new();

        for output_y in 0..self.rid.native_height {
            let source_row =
                sample_source_row(output_y, self.rid.native_height, self.bkd.rows.len());
            if source_row != cached_source_row {
                cached_segments = self.decoded_row_segments(source_row, row_shift, &mapper)?;
                cached_source_row = source_row;
            }

            rows.push(
                cached_segments
                    .iter()
                    .map(|segment| FieldRowSpan {
                        start_x: segment.start_x,
                        end_x: segment.end_x,
                        id: segment.value,
                    })
                    .collect::<Vec<_>>(),
            );
        }

        DiscreteFieldRows::from_row_spans(self.rid.native_width, self.rid.native_height, rows)
    }
}

fn rectangles_to_geometry(rectangles: &[Rectangle]) -> GeoJsonGeometry {
    let polygons: Vec<Vec<Vec<[f64; 2]>>> = rectangles
        .iter()
        .map(|rectangle| {
            vec![vec![
                [rectangle.start_x as f64, rectangle.start_y as f64],
                [rectangle.end_x as f64, rectangle.start_y as f64],
                [rectangle.end_x as f64, rectangle.end_y as f64],
                [rectangle.start_x as f64, rectangle.end_y as f64],
                [rectangle.start_x as f64, rectangle.start_y as f64],
            ]]
        })
        .collect();

    if polygons.len() == 1 {
        GeoJsonGeometry::Polygon(polygons.into_iter().next().expect("single polygon"))
    } else {
        GeoJsonGeometry::MultiPolygon(polygons)
    }
}

fn write_geojson(output_path: &Path, features: &[GeoJsonFeature]) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create output directory {}", parent.display())
            })?;
        }
    }

    let file = File::create(output_path)
        .with_context(|| format!("failed to create {}", output_path.display()))?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(
        writer,
        &GeoJsonFeatureCollection {
            collection_type: "FeatureCollection",
            features: features.to_vec(),
        },
    )
    .with_context(|| format!("failed to write {}", output_path.display()))
}

fn write_field(output_path: &Path, field: &DiscreteFieldRows) -> Result<FieldExportSummary> {
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create output directory {}", parent.display())
            })?;
        }
    }

    let bytes = field.to_bytes();
    fs::write(output_path, &bytes)
        .with_context(|| format!("failed to write {}", output_path.display()))?;

    Ok(FieldExportSummary {
        output_path: output_path.to_path_buf(),
        width: field.width(),
        height: field.height(),
        segment_count: field.segment_count(),
        byte_len: bytes.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::{rectangles_to_geometry, GeoJsonGeometry, PabrMap, Rectangle};
    use crate::pabr::{BkdFile, Breakpoint, RidFile};
    use fishystuff_core::field::DiscreteFieldRows;

    #[test]
    fn build_rectangles_merges_matching_runs_vertically() {
        let map = PabrMap {
            rid_path: "test.rid".into(),
            bkd_path: "test.bkd".into(),
            rid: RidFile {
                region_ids: vec![4, 9],
                native_width: 4,
                native_height: 4,
                trailer_prefix_len: 0,
            },
            bkd: BkdFile {
                rows: vec![
                    vec![
                        Breakpoint {
                            source_x: 0,
                            dictionary_index: 0,
                        },
                        Breakpoint {
                            source_x: 2,
                            dictionary_index: 1,
                        },
                    ],
                    vec![
                        Breakpoint {
                            source_x: 0,
                            dictionary_index: 0,
                        },
                        Breakpoint {
                            source_x: 2,
                            dictionary_index: 1,
                        },
                    ],
                    vec![Breakpoint {
                        source_x: 0,
                        dictionary_index: 1,
                    }],
                    vec![Breakpoint {
                        source_x: 0,
                        dictionary_index: 1,
                    }],
                ],
                trailer_words: [0, 0, 0],
                max_source_x: 3,
            },
        };

        let rectangles = map
            .build_rectangles(0, |region_id| Some(u32::from(region_id)))
            .expect("rectangles");

        assert_eq!(
            rectangles.get(&4),
            Some(&vec![Rectangle {
                start_x: 0,
                end_x: 2,
                start_y: 0,
                end_y: 2,
            }])
        );
        assert_eq!(
            rectangles.get(&9),
            Some(&vec![
                Rectangle {
                    start_x: 2,
                    end_x: 4,
                    start_y: 0,
                    end_y: 2,
                },
                Rectangle {
                    start_x: 0,
                    end_x: 4,
                    start_y: 2,
                    end_y: 4,
                },
            ])
        );
    }

    #[test]
    fn build_field_rows_preserves_background_gaps() {
        let map = PabrMap {
            rid_path: "test.rid".into(),
            bkd_path: "test.bkd".into(),
            rid: RidFile {
                region_ids: vec![4],
                native_width: 4,
                native_height: 2,
                trailer_prefix_len: 0,
            },
            bkd: BkdFile {
                rows: vec![vec![
                    Breakpoint {
                        source_x: 1,
                        dictionary_index: 0,
                    },
                    Breakpoint {
                        source_x: 3,
                        dictionary_index: u16::MAX,
                    },
                ]],
                trailer_words: [0, 0, 0],
                max_source_x: 3,
            },
        };

        let field = map
            .build_field_rows(0, |region_id| Some(u32::from(region_id)))
            .expect("field");

        let expected = DiscreteFieldRows::from_row_spans(
            4,
            2,
            vec![
                vec![fishystuff_core::field::FieldRowSpan {
                    start_x: 0,
                    end_x: 3,
                    id: 4,
                }],
                vec![fishystuff_core::field::FieldRowSpan {
                    start_x: 0,
                    end_x: 3,
                    id: 4,
                }],
            ],
        )
        .expect("expected field");

        assert_eq!(field, expected);
    }

    #[test]
    fn rectangles_to_geometry_uses_multipolygon_for_multiple_rectangles() {
        let geometry = rectangles_to_geometry(&[
            Rectangle {
                start_x: 0,
                end_x: 2,
                start_y: 0,
                end_y: 1,
            },
            Rectangle {
                start_x: 3,
                end_x: 4,
                start_y: 0,
                end_y: 2,
            },
        ]);

        match geometry {
            GeoJsonGeometry::MultiPolygon(polygons) => assert_eq!(polygons.len(), 2),
            GeoJsonGeometry::Polygon(_) => panic!("expected multipolygon"),
        }
    }
}
