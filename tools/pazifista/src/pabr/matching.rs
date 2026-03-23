use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{PabrMap, RegionMatchSummary, RowSegment};
use crate::pabr::util::sample_source_row;

type Interval = (u32, u32);

#[derive(Debug, Deserialize)]
struct CurrentFeatureCollection {
    features: Vec<CurrentFeature>,
}

#[derive(Debug, Deserialize)]
struct CurrentFeature {
    properties: Map<String, Value>,
    geometry: CurrentGeometry,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "coordinates")]
enum CurrentGeometry {
    Polygon(Vec<Vec<[f64; 2]>>),
    MultiPolygon(Vec<Vec<Vec<[f64; 2]>>>),
}

#[derive(Debug, Serialize)]
struct RegionMatchReport {
    native_width: u32,
    native_height: u32,
    row_shift: u32,
    top: usize,
    pabr_region_count: usize,
    current_region_count: usize,
    overlap_pair_count: usize,
    pabr_only_region_ids: Vec<u32>,
    current_only_region_ids: Vec<u32>,
    pabr_regions: Vec<RegionMatchRecord>,
    current_regions: Vec<RegionMatchRecord>,
    pabr_only_candidates: Vec<RegionMatchRecord>,
    current_only_candidates: Vec<RegionMatchRecord>,
    mutual_best_matches: Vec<MutualBestMatch>,
    mutual_best_id_changes: Vec<MutualBestMatch>,
}

#[derive(Debug, Serialize)]
struct RegionMatchRecord {
    region_id: u32,
    label: Option<String>,
    area: u64,
    top_matches: Vec<MatchCandidate>,
}

#[derive(Debug, Clone, Serialize)]
struct MatchCandidate {
    region_id: u32,
    label: Option<String>,
    intersection_area: u64,
    source_coverage: f64,
    target_coverage: f64,
    iou: f64,
}

#[derive(Debug, Clone, Serialize)]
struct MutualBestMatch {
    pabr_region_id: u32,
    pabr_label: Option<String>,
    current_region_id: u32,
    current_label: Option<String>,
    intersection_area: u64,
    pabr_coverage: f64,
    current_coverage: f64,
    iou: f64,
}

impl PabrMap {
    pub fn native_region_areas(&self, row_shift: u32) -> Result<BTreeMap<u32, u64>> {
        let (_, areas) = self.build_native_region_rows(row_shift)?;
        Ok(areas)
    }

    pub fn match_regions_geojson(
        &self,
        current_regions_path: &Path,
        output_path: &Path,
        row_shift: u32,
        top: usize,
    ) -> Result<RegionMatchSummary> {
        if top == 0 {
            bail!("--top must be at least 1");
        }

        let current_layer = load_current_feature_collection(current_regions_path)?;
        let (pabr_rows, pabr_areas) = self.build_native_region_rows(row_shift)?;
        let (current_rows, current_region_ids, current_areas, current_labels) =
            rasterize_current_regions(
                &current_layer.features,
                self.rid.native_width,
                self.rid.native_height,
            )?;
        let overlap_areas = accumulate_overlap_areas(&pabr_rows, &current_rows)?;

        let pabr_region_ids: BTreeSet<u32> = pabr_areas.keys().copied().collect();
        let pabr_only_region_ids: Vec<u32> = pabr_region_ids
            .difference(&current_region_ids)
            .copied()
            .collect();
        let current_only_region_ids: Vec<u32> = current_region_ids
            .difference(&pabr_region_ids)
            .copied()
            .collect();

        let pabr_labels: BTreeMap<u32, String> = current_labels
            .iter()
            .filter(|(region_id, _)| pabr_region_ids.contains(region_id))
            .map(|(region_id, label)| (*region_id, label.clone()))
            .collect();

        let matches_by_pabr = build_adjacency(&overlap_areas, false);
        let matches_by_current = build_adjacency(&overlap_areas, true);

        let pabr_regions = build_region_match_records(
            &pabr_region_ids,
            &pabr_areas,
            &current_areas,
            &pabr_labels,
            &current_labels,
            &matches_by_pabr,
            top,
        );
        let current_regions = build_region_match_records(
            &current_region_ids,
            &current_areas,
            &pabr_areas,
            &current_labels,
            &pabr_labels,
            &matches_by_current,
            top,
        );
        let pabr_only_candidates =
            filter_region_match_records(&pabr_regions, &pabr_only_region_ids);
        let current_only_candidates =
            filter_region_match_records(&current_regions, &current_only_region_ids);
        let mutual_best_matches = build_mutual_best_matches(
            &pabr_regions,
            &current_regions,
            &pabr_labels,
            &current_labels,
        );
        let mutual_best_id_changes = mutual_best_matches
            .iter()
            .filter(|candidate| candidate.pabr_region_id != candidate.current_region_id)
            .cloned()
            .collect();

        let report = RegionMatchReport {
            native_width: self.rid.native_width,
            native_height: self.rid.native_height,
            row_shift,
            top,
            pabr_region_count: pabr_regions.len(),
            current_region_count: current_regions.len(),
            overlap_pair_count: overlap_areas.len(),
            pabr_only_region_ids,
            current_only_region_ids,
            pabr_regions,
            current_regions,
            pabr_only_candidates,
            current_only_candidates,
            mutual_best_matches,
            mutual_best_id_changes,
        };

        write_match_report(output_path, &report)?;
        Ok(RegionMatchSummary {
            output_path: output_path.to_path_buf(),
            pabr_region_count: report.pabr_region_count,
            current_region_count: report.current_region_count,
            overlap_pair_count: report.overlap_pair_count,
            pabr_only_count: report.pabr_only_region_ids.len(),
            current_only_count: report.current_only_region_ids.len(),
            mutual_best_match_count: report.mutual_best_matches.len(),
        })
    }

    fn build_native_region_rows(
        &self,
        row_shift: u32,
    ) -> Result<(Vec<Vec<RowSegment>>, BTreeMap<u32, u64>)> {
        if self.rid.native_width == 0 || self.rid.native_height == 0 {
            bail!("PABR matching requires non-zero native dimensions");
        }
        if self.bkd.rows.is_empty() {
            bail!("PABR matching requires at least one BKD row");
        }

        let mut rows = vec![Vec::new(); self.rid.native_height as usize];
        let mut areas = BTreeMap::new();
        let mut cached_source_row = usize::MAX;
        let mut cached_segments = Vec::new();

        for output_y in 0..self.rid.native_height {
            let source_row =
                sample_source_row(output_y, self.rid.native_height, self.bkd.rows.len());
            if source_row != cached_source_row {
                cached_segments =
                    self.decoded_row_segments(source_row, row_shift, &|region_id| {
                        Some(u32::from(region_id))
                    })?;
                cached_source_row = source_row;
            }

            let row = &mut rows[output_y as usize];
            row.reserve(cached_segments.len());
            for segment in &cached_segments {
                let width = u64::from(segment.end_x - segment.start_x);
                *areas.entry(segment.value).or_insert(0) += width;
                row.push(*segment);
            }
        }

        Ok((rows, areas))
    }
}

fn load_current_feature_collection(path: &Path) -> Result<CurrentFeatureCollection> {
    let file = File::open(path)
        .with_context(|| format!("failed to open current regions GeoJSON {}", path.display()))?;
    serde_json::from_reader(file)
        .with_context(|| format!("failed to parse GeoJSON {}", path.display()))
}

fn rasterize_current_regions(
    features: &[CurrentFeature],
    width: u32,
    height: u32,
) -> Result<(
    Vec<Vec<RowSegment>>,
    BTreeSet<u32>,
    BTreeMap<u32, u64>,
    BTreeMap<u32, String>,
)> {
    let mut rows = vec![Vec::new(); height as usize];
    let mut region_ids = BTreeSet::new();
    let mut areas = BTreeMap::new();
    let mut labels = BTreeMap::new();

    for feature in features {
        let region_id = extract_u32_property(&feature.properties, "r")
            .context("current regions feature is missing numeric property `r`")?;
        region_ids.insert(region_id);
        if let Some(label) = feature
            .properties
            .get("on")
            .and_then(Value::as_str)
            .map(str::to_owned)
        {
            labels.entry(region_id).or_insert(label);
        }

        let rasterized = rasterize_geometry(&feature.geometry, width, height)
            .with_context(|| format!("failed to rasterize current region {}", region_id))?;
        for (row_y, intervals) in rasterized {
            let row = &mut rows[row_y as usize];
            for (start_x, end_x) in intervals {
                let width = u64::from(end_x - start_x);
                *areas.entry(region_id).or_insert(0) += width;
                row.push(RowSegment {
                    start_x,
                    end_x,
                    value: region_id,
                });
            }
        }
    }

    for row in &mut rows {
        row.sort_by_key(|segment| (segment.start_x, segment.end_x, segment.value));
        merge_same_value_segments(row);
    }

    Ok((rows, region_ids, areas, labels))
}

fn extract_u32_property(properties: &Map<String, Value>, key: &str) -> Result<u32> {
    let value = properties
        .get(key)
        .with_context(|| format!("missing property `{}`", key))?;
    if let Some(number) = value.as_u64() {
        return u32::try_from(number).with_context(|| format!("property `{}` exceeds u32", key));
    }
    if let Some(number) = value.as_i64() {
        return u32::try_from(number)
            .with_context(|| format!("property `{}` is negative or exceeds u32", key));
    }

    bail!("property `{}` is not numeric", key)
}

fn rasterize_geometry(
    geometry: &CurrentGeometry,
    width: u32,
    height: u32,
) -> Result<BTreeMap<u32, Vec<Interval>>> {
    let mut rows = BTreeMap::new();

    match geometry {
        CurrentGeometry::Polygon(rings) => {
            merge_row_interval_maps(&mut rows, rasterize_polygon(rings, width, height)?);
        }
        CurrentGeometry::MultiPolygon(polygons) => {
            for rings in polygons {
                merge_row_interval_maps(&mut rows, rasterize_polygon(rings, width, height)?);
            }
        }
    }

    Ok(rows)
}

fn rasterize_polygon(
    rings: &[Vec<[f64; 2]>],
    width: u32,
    height: u32,
) -> Result<BTreeMap<u32, Vec<Interval>>> {
    if rings.is_empty() {
        return Ok(BTreeMap::new());
    }

    let mut rows = rasterize_ring(&rings[0], width, height)?;
    for hole in &rings[1..] {
        let hole_rows = rasterize_ring(hole, width, height)?;
        for (row_y, hole_intervals) in hole_rows {
            if let Some(base_intervals) = rows.get_mut(&row_y) {
                *base_intervals = subtract_intervals(base_intervals, &hole_intervals);
                if base_intervals.is_empty() {
                    rows.remove(&row_y);
                }
            }
        }
    }

    Ok(rows)
}

fn rasterize_ring(
    ring: &[[f64; 2]],
    width: u32,
    height: u32,
) -> Result<BTreeMap<u32, Vec<Interval>>> {
    if ring.len() < 3 {
        bail!("GeoJSON ring must contain at least three vertices");
    }

    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for &[x, y] in ring {
        if !x.is_finite() || !y.is_finite() {
            bail!("GeoJSON ring contains non-finite coordinates");
        }
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    let row_start = ((min_y - 0.5).floor() as i64).clamp(0, i64::from(height));
    let row_end = ((max_y - 0.5).ceil() as i64).clamp(0, i64::from(height));
    let mut rows = BTreeMap::new();

    for row_y in row_start..row_end {
        let scan_y = row_y as f64 + 0.5;
        let mut intersections = Vec::new();

        for index in 0..ring.len() {
            let [x1, y1] = ring[index];
            let [x2, y2] = ring[(index + 1) % ring.len()];
            if y1 == y2 {
                continue;
            }

            let intersects = (y1 <= scan_y && scan_y < y2) || (y2 <= scan_y && scan_y < y1);
            if !intersects {
                continue;
            }

            let t = (scan_y - y1) / (y2 - y1);
            intersections.push(x1 + t * (x2 - x1));
        }

        intersections.sort_by(f64::total_cmp);
        if intersections.len() % 2 != 0 {
            bail!("GeoJSON ring produced an odd number of scanline intersections");
        }

        let mut intervals = Vec::new();
        for pair in intersections.chunks_exact(2) {
            if let Some(interval) = span_to_interval(pair[0], pair[1], width) {
                intervals.push(interval);
            }
        }

        merge_intervals(&mut intervals);
        if !intervals.is_empty() {
            rows.insert(row_y as u32, intervals);
        }
    }

    Ok(rows)
}

fn span_to_interval(left: f64, right: f64, width: u32) -> Option<Interval> {
    if !left.is_finite() || !right.is_finite() || right <= left {
        return None;
    }

    let start_x = ((left - 0.5).ceil() as i64).clamp(0, i64::from(width)) as u32;
    let end_x = ((right - 0.5).ceil() as i64).clamp(0, i64::from(width)) as u32;
    (start_x < end_x).then_some((start_x, end_x))
}

fn merge_row_interval_maps(
    destination: &mut BTreeMap<u32, Vec<Interval>>,
    source: BTreeMap<u32, Vec<Interval>>,
) {
    for (row_y, mut intervals) in source {
        let row = destination.entry(row_y).or_default();
        row.append(&mut intervals);
        merge_intervals(row);
    }
}

fn merge_intervals(intervals: &mut Vec<Interval>) {
    if intervals.len() < 2 {
        return;
    }

    intervals.sort_unstable_by_key(|&(start_x, end_x)| (start_x, end_x));
    let mut merged = Vec::with_capacity(intervals.len());
    let mut current = intervals[0];

    for &(start_x, end_x) in &intervals[1..] {
        if start_x <= current.1 {
            current.1 = current.1.max(end_x);
        } else {
            merged.push(current);
            current = (start_x, end_x);
        }
    }
    merged.push(current);
    *intervals = merged;
}

fn subtract_intervals(base: &[Interval], holes: &[Interval]) -> Vec<Interval> {
    if base.is_empty() || holes.is_empty() {
        return base.to_vec();
    }

    let mut result = Vec::new();
    let mut hole_index = 0usize;

    for &(start_x, end_x) in base {
        let mut cursor = start_x;
        while hole_index < holes.len() && holes[hole_index].1 <= cursor {
            hole_index += 1;
        }

        let mut scan_index = hole_index;
        while scan_index < holes.len() {
            let (hole_start, hole_end) = holes[scan_index];
            if hole_start >= end_x {
                break;
            }
            if hole_start > cursor {
                result.push((cursor, hole_start.min(end_x)));
            }
            cursor = cursor.max(hole_end.min(end_x));
            if cursor >= end_x {
                break;
            }
            scan_index += 1;
        }

        if cursor < end_x {
            result.push((cursor, end_x));
        }
    }

    result.retain(|&(start_x, end_x)| start_x < end_x);
    result
}

fn merge_same_value_segments(row: &mut Vec<RowSegment>) {
    if row.len() < 2 {
        return;
    }

    let mut merged = Vec::with_capacity(row.len());
    let mut current = row[0];

    for &segment in &row[1..] {
        if segment.value == current.value && segment.start_x <= current.end_x {
            current.end_x = current.end_x.max(segment.end_x);
        } else {
            merged.push(current);
            current = segment;
        }
    }
    merged.push(current);
    *row = merged;
}

fn accumulate_overlap_areas(
    pabr_rows: &[Vec<RowSegment>],
    current_rows: &[Vec<RowSegment>],
) -> Result<BTreeMap<(u32, u32), u64>> {
    if pabr_rows.len() != current_rows.len() {
        bail!(
            "row-count mismatch while matching: PABR has {}, current layer has {}",
            pabr_rows.len(),
            current_rows.len()
        );
    }

    let mut overlaps = BTreeMap::new();
    for (pabr_row, current_row) in pabr_rows.iter().zip(current_rows) {
        let mut pabr_index = 0usize;
        let mut current_index = 0usize;

        while pabr_index < pabr_row.len() && current_index < current_row.len() {
            let pabr_segment = pabr_row[pabr_index];
            let current_segment = current_row[current_index];
            let overlap_start = pabr_segment.start_x.max(current_segment.start_x);
            let overlap_end = pabr_segment.end_x.min(current_segment.end_x);
            if overlap_start < overlap_end {
                *overlaps
                    .entry((pabr_segment.value, current_segment.value))
                    .or_insert(0) += u64::from(overlap_end - overlap_start);
            }

            match pabr_segment.end_x.cmp(&current_segment.end_x) {
                Ordering::Less => pabr_index += 1,
                Ordering::Greater => current_index += 1,
                Ordering::Equal => {
                    pabr_index += 1;
                    current_index += 1;
                }
            }
        }
    }

    Ok(overlaps)
}

fn build_adjacency(
    overlap_areas: &BTreeMap<(u32, u32), u64>,
    invert: bool,
) -> BTreeMap<u32, Vec<(u32, u64)>> {
    let mut adjacency: BTreeMap<u32, Vec<(u32, u64)>> = BTreeMap::new();
    for (&(pabr_region_id, current_region_id), &intersection_area) in overlap_areas {
        let (source_region_id, target_region_id) = if invert {
            (current_region_id, pabr_region_id)
        } else {
            (pabr_region_id, current_region_id)
        };
        adjacency
            .entry(source_region_id)
            .or_default()
            .push((target_region_id, intersection_area));
    }
    adjacency
}

fn build_region_match_records(
    source_region_ids: &BTreeSet<u32>,
    source_areas: &BTreeMap<u32, u64>,
    target_areas: &BTreeMap<u32, u64>,
    source_labels: &BTreeMap<u32, String>,
    target_labels: &BTreeMap<u32, String>,
    adjacency: &BTreeMap<u32, Vec<(u32, u64)>>,
    top: usize,
) -> Vec<RegionMatchRecord> {
    source_region_ids
        .iter()
        .map(|&region_id| {
            let area = *source_areas.get(&region_id).unwrap_or(&0);
            let mut candidates = adjacency.get(&region_id).cloned().unwrap_or_default();
            candidates
                .sort_by(|left, right| compare_candidate_rank(*left, *right, area, target_areas));
            let top_matches = candidates
                .into_iter()
                .take(top)
                .filter_map(|(candidate_region_id, intersection_area)| {
                    let candidate_area = *target_areas.get(&candidate_region_id)?;
                    Some(MatchCandidate {
                        region_id: candidate_region_id,
                        label: target_labels.get(&candidate_region_id).cloned(),
                        intersection_area,
                        source_coverage: ratio(intersection_area, area),
                        target_coverage: ratio(intersection_area, candidate_area),
                        iou: ratio(intersection_area, area + candidate_area - intersection_area),
                    })
                })
                .collect();

            RegionMatchRecord {
                region_id,
                label: source_labels.get(&region_id).cloned(),
                area,
                top_matches,
            }
        })
        .collect()
}

fn compare_candidate_rank(
    left: (u32, u64),
    right: (u32, u64),
    source_area: u64,
    target_areas: &BTreeMap<u32, u64>,
) -> Ordering {
    right
        .1
        .cmp(&left.1)
        .then_with(|| {
            let left_target_area = *target_areas.get(&left.0).unwrap_or(&1);
            let right_target_area = *target_areas.get(&right.0).unwrap_or(&1);
            compare_fraction_desc(left.1, left_target_area, right.1, right_target_area)
        })
        .then_with(|| {
            let left_target_area = *target_areas.get(&left.0).unwrap_or(&1);
            let right_target_area = *target_areas.get(&right.0).unwrap_or(&1);
            let left_union = source_area + left_target_area - left.1;
            let right_union = source_area + right_target_area - right.1;
            compare_fraction_desc(left.1, left_union, right.1, right_union)
        })
        .then_with(|| left.0.cmp(&right.0))
}

fn compare_fraction_desc(
    left_numerator: u64,
    left_denominator: u64,
    right_numerator: u64,
    right_denominator: u64,
) -> Ordering {
    (u128::from(right_numerator) * u128::from(left_denominator))
        .cmp(&(u128::from(left_numerator) * u128::from(right_denominator)))
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn filter_region_match_records(
    records: &[RegionMatchRecord],
    region_ids: &[u32],
) -> Vec<RegionMatchRecord> {
    let region_ids: BTreeSet<u32> = region_ids.iter().copied().collect();
    records
        .iter()
        .filter(|record| region_ids.contains(&record.region_id))
        .map(|record| RegionMatchRecord {
            region_id: record.region_id,
            label: record.label.clone(),
            area: record.area,
            top_matches: record.top_matches.clone(),
        })
        .collect()
}

fn build_mutual_best_matches(
    pabr_regions: &[RegionMatchRecord],
    current_regions: &[RegionMatchRecord],
    pabr_labels: &BTreeMap<u32, String>,
    current_labels: &BTreeMap<u32, String>,
) -> Vec<MutualBestMatch> {
    let current_best_by_region: BTreeMap<u32, &MatchCandidate> = current_regions
        .iter()
        .filter_map(|record| {
            record
                .top_matches
                .first()
                .map(|candidate| (record.region_id, candidate))
        })
        .collect();

    let mut mutual = Vec::new();
    for record in pabr_regions {
        let Some(best_current) = record.top_matches.first() else {
            continue;
        };
        let Some(best_back) = current_best_by_region.get(&best_current.region_id) else {
            continue;
        };
        if best_back.region_id != record.region_id {
            continue;
        }

        mutual.push(MutualBestMatch {
            pabr_region_id: record.region_id,
            pabr_label: pabr_labels.get(&record.region_id).cloned(),
            current_region_id: best_current.region_id,
            current_label: current_labels.get(&best_current.region_id).cloned(),
            intersection_area: best_current.intersection_area,
            pabr_coverage: best_current.source_coverage,
            current_coverage: best_current.target_coverage,
            iou: best_current.iou,
        });
    }

    mutual
}

fn write_match_report(output_path: &Path, report: &RegionMatchReport) -> Result<()> {
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
    serde_json::to_writer_pretty(writer, report)
        .with_context(|| format!("failed to write {}", output_path.display()))
}
