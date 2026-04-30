use std::collections::{BTreeMap, HashMap};

use fishystuff_api::models::events::{EventPointCompact, EventsQueryMode, MapBboxPx};

use crate::plugins::api::PointSampleSummary;

const RAW_RENDER_THRESHOLD: usize = 2_000;
const FULL_DETAIL_VIEWPORT_MAX_SPAN_PX: i32 = 256;

#[derive(Debug, Clone)]
pub struct DerivedRenderPoint {
    pub map_px_x: i32,
    pub map_px_y: i32,
    pub world_x: Option<i32>,
    pub world_z: Option<i32>,
    pub fish_id: Option<i32>,
    pub sample_count: u32,
    pub aggregated: bool,
    pub point_samples: Vec<PointSampleSummary>,
}

#[derive(Debug, Clone)]
pub struct ClusterOutput {
    pub mode: EventsQueryMode,
    pub cluster_bucket_px: Option<i32>,
    pub represented_event_count: usize,
    pub rendered_point_count: usize,
    pub rendered_cluster_count: usize,
    pub points: Vec<DerivedRenderPoint>,
}

pub fn suggested_cluster_bucket_px(bbox: &MapBboxPx) -> i32 {
    let width = (bbox.max_x - bbox.min_x).abs() + 1;
    let height = (bbox.max_y - bbox.min_y).abs() + 1;
    let span = width.max(height);
    if span >= 7000 {
        256
    } else if span >= 4000 {
        160
    } else if span >= 2200 {
        96
    } else if span >= 1200 {
        64
    } else {
        32
    }
}

pub fn cluster_view_events(
    events: &[EventPointCompact],
    filtered_indices: &[usize],
    viewport_bbox: &MapBboxPx,
    cluster_bucket_px: i32,
) -> ClusterOutput {
    if should_render_raw_detail(filtered_indices.len(), viewport_bbox) {
        let mut points = Vec::with_capacity(filtered_indices.len());
        for idx in filtered_indices {
            let Some(event) = events.get(*idx) else {
                continue;
            };
            points.push(DerivedRenderPoint {
                map_px_x: event.map_px_x,
                map_px_y: event.map_px_y,
                world_x: event.world_x,
                world_z: event.world_z,
                fish_id: Some(event.fish_id),
                sample_count: 1,
                aggregated: false,
                point_samples: vec![point_sample_summary_for_event(event)],
            });
        }
        return ClusterOutput {
            mode: EventsQueryMode::Raw,
            cluster_bucket_px: None,
            represented_event_count: filtered_indices.len(),
            rendered_point_count: points.len(),
            rendered_cluster_count: 0,
            points,
        };
    }

    let cluster_bucket_px = cluster_bucket_px.max(1);
    let mut bins: BTreeMap<(i32, i32), ClusterAcc> = BTreeMap::new();
    for idx in filtered_indices {
        let Some(event) = events.get(*idx) else {
            continue;
        };
        let key = (
            event.map_px_x.div_euclid(cluster_bucket_px),
            event.map_px_y.div_euclid(cluster_bucket_px),
        );
        bins.entry(key).or_default().push(event);
    }

    let mut points = Vec::with_capacity(bins.len());
    for acc in bins.into_values() {
        points.push(acc.to_render_point());
    }

    ClusterOutput {
        mode: EventsQueryMode::GridAggregate,
        cluster_bucket_px: Some(cluster_bucket_px),
        represented_event_count: filtered_indices.len(),
        rendered_point_count: 0,
        rendered_cluster_count: points.len(),
        points,
    }
}

fn should_render_raw_detail(filtered_count: usize, viewport_bbox: &MapBboxPx) -> bool {
    filtered_count <= RAW_RENDER_THRESHOLD
        || viewport_span_px(viewport_bbox) <= FULL_DETAIL_VIEWPORT_MAX_SPAN_PX
}

fn viewport_span_px(bbox: &MapBboxPx) -> i32 {
    let width = (bbox.max_x - bbox.min_x).abs() + 1;
    let height = (bbox.max_y - bbox.min_y).abs() + 1;
    width.max(height)
}

#[derive(Debug, Default, Clone)]
struct ClusterAcc {
    count: u32,
    sum_map_x: i64,
    sum_map_y: i64,
    sum_world_x: i64,
    sum_world_z: i64,
    world_count: u32,
    fish_counts: HashMap<i32, u32>,
    point_samples: BTreeMap<PointSampleSummaryKey, PointSampleAccumulator>,
}

impl ClusterAcc {
    fn push(&mut self, event: &EventPointCompact) {
        self.count = self.count.saturating_add(1);
        self.sum_map_x += i64::from(event.map_px_x);
        self.sum_map_y += i64::from(event.map_px_y);
        if let (Some(world_x), Some(world_z)) = (event.world_x, event.world_z) {
            self.sum_world_x += i64::from(world_x);
            self.sum_world_z += i64::from(world_z);
            self.world_count = self.world_count.saturating_add(1);
        }
        *self.fish_counts.entry(event.fish_id).or_insert(0) += 1;
        self.point_samples
            .entry(PointSampleSummaryKey::from_event(event))
            .or_insert_with(|| PointSampleAccumulator {
                fish_id: event.fish_id,
                sample_count: 0,
                last_ts_utc: event.ts_utc,
                zone_rgbs: normalized_zone_rgbs(event),
                full_zone_rgbs: normalized_full_zone_rgbs(event),
            })
            .push(event);
    }

    fn to_render_point(&self) -> DerivedRenderPoint {
        let count_i64 = i64::from(self.count.max(1));
        let top_fish_id = self.top_fish_id();
        DerivedRenderPoint {
            map_px_x: (self.sum_map_x as f64 / count_i64 as f64).round() as i32,
            map_px_y: (self.sum_map_y as f64 / count_i64 as f64).round() as i32,
            world_x: if self.world_count > 0 {
                Some(
                    (self.sum_world_x as f64 / f64::from(self.world_count))
                        .round()
                        .clamp(i32::MIN as f64, i32::MAX as f64) as i32,
                )
            } else {
                None
            },
            world_z: if self.world_count > 0 {
                Some(
                    (self.sum_world_z as f64 / f64::from(self.world_count))
                        .round()
                        .clamp(i32::MIN as f64, i32::MAX as f64) as i32,
                )
            } else {
                None
            },
            fish_id: top_fish_id,
            sample_count: self.count.max(1),
            aggregated: true,
            point_samples: sorted_point_samples(&self.point_samples),
        }
    }

    fn top_fish_id(&self) -> Option<i32> {
        let mut best: Option<(i32, u32)> = None;
        for (&fish_id, &count) in &self.fish_counts {
            match best {
                None => best = Some((fish_id, count)),
                Some((best_fish, best_count)) => {
                    if count > best_count || (count == best_count && fish_id < best_fish) {
                        best = Some((fish_id, count));
                    }
                }
            }
        }
        best.map(|(fish_id, _)| fish_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PointSampleSummaryKey {
    fish_id: i32,
    zone_rgbs: Vec<u32>,
    full_zone_rgbs: Vec<u32>,
}

impl PointSampleSummaryKey {
    fn from_event(event: &EventPointCompact) -> Self {
        Self {
            fish_id: event.fish_id,
            zone_rgbs: normalized_zone_rgbs(event),
            full_zone_rgbs: normalized_full_zone_rgbs(event),
        }
    }
}

#[derive(Debug, Clone)]
struct PointSampleAccumulator {
    fish_id: i32,
    sample_count: u32,
    last_ts_utc: i64,
    zone_rgbs: Vec<u32>,
    full_zone_rgbs: Vec<u32>,
}

impl PointSampleAccumulator {
    fn push(&mut self, event: &EventPointCompact) {
        self.sample_count = self.sample_count.saturating_add(1);
        self.last_ts_utc = self.last_ts_utc.max(event.ts_utc);
    }

    fn to_summary(&self) -> PointSampleSummary {
        PointSampleSummary {
            fish_id: self.fish_id,
            sample_count: self.sample_count.max(1),
            last_ts_utc: self.last_ts_utc,
            zone_rgbs: self.zone_rgbs.clone(),
            full_zone_rgbs: self.full_zone_rgbs.clone(),
        }
    }
}

fn point_sample_summary_for_event(event: &EventPointCompact) -> PointSampleSummary {
    PointSampleSummary {
        fish_id: event.fish_id,
        sample_count: 1,
        last_ts_utc: event.ts_utc,
        zone_rgbs: normalized_zone_rgbs(event),
        full_zone_rgbs: normalized_full_zone_rgbs(event),
    }
}

fn sorted_point_samples(
    samples: &BTreeMap<PointSampleSummaryKey, PointSampleAccumulator>,
) -> Vec<PointSampleSummary> {
    let mut summaries = samples
        .values()
        .map(PointSampleAccumulator::to_summary)
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| {
        right
            .sample_count
            .cmp(&left.sample_count)
            .then_with(|| right.last_ts_utc.cmp(&left.last_ts_utc))
            .then_with(|| left.fish_id.cmp(&right.fish_id))
            .then_with(|| left.zone_rgbs.cmp(&right.zone_rgbs))
            .then_with(|| left.full_zone_rgbs.cmp(&right.full_zone_rgbs))
    });
    summaries
}

fn normalized_zone_rgbs(event: &EventPointCompact) -> Vec<u32> {
    if !event.zone_rgbs.is_empty() {
        return sorted_deduped(event.zone_rgbs.clone());
    }
    event.zone_rgb_u32.into_iter().collect()
}

fn normalized_full_zone_rgbs(event: &EventPointCompact) -> Vec<u32> {
    if !event.full_zone_rgbs.is_empty() {
        return sorted_deduped(event.full_zone_rgbs.clone());
    }
    if event.zone_rgbs.is_empty() {
        return event.zone_rgb_u32.into_iter().collect();
    }
    Vec::new()
}

fn sorted_deduped(mut values: Vec<u32>) -> Vec<u32> {
    values.sort_unstable();
    values.dedup();
    values
}

#[cfg(test)]
mod tests {
    use super::cluster_view_events;
    use fishystuff_api::models::events::{EventPointCompact, EventsQueryMode, MapBboxPx};

    #[test]
    fn clustering_is_deterministic_for_same_inputs() {
        let mut events = Vec::new();
        for idx in 0..6000 {
            events.push(EventPointCompact {
                event_id: idx as i64,
                fish_id: if idx % 2 == 0 { 10 } else { 20 },
                ts_utc: 1_700_000_000 + idx as i64,
                map_px_x: 1000 + (idx % 40) as i32,
                map_px_y: 2000 + (idx % 40) as i32,
                length_milli: 1000,
                world_x: None,
                world_z: None,
                zone_rgb_u32: None,
                zone_rgbs: Vec::new(),
                full_zone_rgbs: Vec::new(),
                source_kind: None,
                source_id: None,
            });
        }
        let indices: Vec<usize> = (0..events.len()).collect();
        let viewport_bbox = MapBboxPx {
            min_x: 0,
            min_y: 0,
            max_x: 2_000,
            max_y: 2_000,
        };
        let a = cluster_view_events(&events, &indices, &viewport_bbox, 64);
        let b = cluster_view_events(&events, &indices, &viewport_bbox, 64);

        assert_eq!(a.mode, EventsQueryMode::GridAggregate);
        assert_eq!(a.rendered_cluster_count, b.rendered_cluster_count);
        assert_eq!(a.points.len(), b.points.len());
        for (left, right) in a.points.iter().zip(b.points.iter()) {
            assert_eq!(left.map_px_x, right.map_px_x);
            assert_eq!(left.map_px_y, right.map_px_y);
            assert_eq!(left.sample_count, right.sample_count);
            assert_eq!(left.fish_id, right.fish_id);
        }
    }

    #[test]
    fn aggregated_clusters_keep_a_representative_fish_id() {
        let mut events = Vec::new();
        for idx in 0..2501 {
            events.push(EventPointCompact {
                event_id: idx as i64,
                fish_id: if idx < 2000 { 10 } else { 20 },
                ts_utc: 1_700_000_000 + idx as i64,
                map_px_x: 1000 + (idx % 8) as i32,
                map_px_y: 2000 + (idx % 8) as i32,
                length_milli: 1000,
                world_x: Some(10_000 + idx as i32),
                world_z: Some(20_000 + idx as i32),
                zone_rgb_u32: None,
                zone_rgbs: Vec::new(),
                full_zone_rgbs: Vec::new(),
                source_kind: None,
                source_id: None,
            });
        }

        let indices: Vec<usize> = (0..events.len()).collect();
        let viewport_bbox = MapBboxPx {
            min_x: 0,
            min_y: 0,
            max_x: 2_000,
            max_y: 2_000,
        };
        let clustered = cluster_view_events(&events, &indices, &viewport_bbox, 64);

        assert_eq!(clustered.mode, EventsQueryMode::GridAggregate);
        assert_eq!(clustered.points.len(), 1);
        assert_eq!(clustered.points[0].fish_id, Some(10));
        assert!(clustered.points[0].aggregated);
        assert_eq!(clustered.points[0].sample_count, 2501);
    }

    #[test]
    fn aggregated_clusters_keep_sorted_point_sample_summaries() {
        let mut events = Vec::new();
        for idx in 0..2501 {
            let (fish_id, zone_rgbs, full_zone_rgbs) = if idx < 1200 {
                (10, vec![0x39e58d], vec![0x39e58d])
            } else if idx < 2000 {
                (20, vec![0x123456, 0x654321], Vec::new())
            } else {
                (30, vec![0x0abcde], vec![0x0abcde])
            };
            events.push(EventPointCompact {
                event_id: idx as i64,
                fish_id,
                ts_utc: 1_700_000_000 + idx as i64,
                map_px_x: 1000 + (idx % 8) as i32,
                map_px_y: 2000 + (idx % 8) as i32,
                length_milli: 1000,
                world_x: None,
                world_z: None,
                zone_rgb_u32: None,
                zone_rgbs,
                full_zone_rgbs,
                source_kind: None,
                source_id: None,
            });
        }

        let indices: Vec<usize> = (0..events.len()).collect();
        let viewport_bbox = MapBboxPx {
            min_x: 0,
            min_y: 0,
            max_x: 2_000,
            max_y: 2_000,
        };
        let clustered = cluster_view_events(&events, &indices, &viewport_bbox, 64);
        let summaries = &clustered.points[0].point_samples;

        assert_eq!(clustered.mode, EventsQueryMode::GridAggregate);
        assert_eq!(clustered.points.len(), 1);
        assert_eq!(summaries.len(), 3);
        assert_eq!(summaries[0].fish_id, 10);
        assert_eq!(summaries[0].sample_count, 1200);
        assert_eq!(summaries[0].full_zone_rgbs, vec![0x39e58d]);
        assert_eq!(summaries[1].fish_id, 20);
        assert_eq!(summaries[1].sample_count, 800);
        assert_eq!(summaries[1].zone_rgbs, vec![0x123456, 0x654321]);
        assert!(summaries[1].full_zone_rgbs.is_empty());
        assert_eq!(summaries[2].fish_id, 30);
        assert_eq!(summaries[2].sample_count, 501);
    }

    #[test]
    fn full_detail_view_keeps_raw_points_even_above_threshold() {
        let mut events = Vec::new();
        for idx in 0..2501 {
            events.push(EventPointCompact {
                event_id: idx as i64,
                fish_id: if idx % 2 == 0 { 10 } else { 20 },
                ts_utc: 1_700_000_000 + idx as i64,
                map_px_x: 1000 + (idx % 16) as i32,
                map_px_y: 2000 + (idx % 16) as i32,
                length_milli: 1000,
                world_x: None,
                world_z: None,
                zone_rgb_u32: None,
                zone_rgbs: Vec::new(),
                full_zone_rgbs: Vec::new(),
                source_kind: None,
                source_id: None,
            });
        }

        let indices: Vec<usize> = (0..events.len()).collect();
        let viewport_bbox = MapBboxPx {
            min_x: 1000,
            min_y: 2000,
            max_x: 1100,
            max_y: 2100,
        };
        let clustered = cluster_view_events(&events, &indices, &viewport_bbox, 32);

        assert_eq!(clustered.mode, EventsQueryMode::Raw);
        assert_eq!(clustered.cluster_bucket_px, None);
        assert_eq!(clustered.rendered_cluster_count, 0);
        assert_eq!(clustered.rendered_point_count, indices.len());
        assert!(clustered.points.iter().all(|point| !point.aggregated));
    }
}
