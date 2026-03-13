use std::collections::{BTreeMap, HashMap};

use fishystuff_api::models::events::{EventPointCompact, EventsQueryMode, MapBboxPx};

const RAW_RENDER_THRESHOLD: usize = 2_000;

#[derive(Debug, Clone)]
pub struct DerivedRenderPoint {
    pub map_px_x: i32,
    pub map_px_y: i32,
    pub world_x: Option<i32>,
    pub world_z: Option<i32>,
    pub fish_id: Option<i32>,
    pub sample_count: u32,
    pub aggregated: bool,
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
    cluster_bucket_px: i32,
) -> ClusterOutput {
    if filtered_indices.len() <= RAW_RENDER_THRESHOLD {
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

#[derive(Debug, Default, Clone)]
struct ClusterAcc {
    count: u32,
    sum_map_x: i64,
    sum_map_y: i64,
    sum_world_x: i64,
    sum_world_z: i64,
    world_count: u32,
    fish_counts: HashMap<i32, u32>,
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

#[cfg(test)]
mod tests {
    use super::cluster_view_events;
    use fishystuff_api::models::events::{EventPointCompact, EventsQueryMode};

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
                source_kind: None,
                source_id: None,
            });
        }
        let indices: Vec<usize> = (0..events.len()).collect();
        let a = cluster_view_events(&events, &indices, 64);
        let b = cluster_view_events(&events, &indices, 64);

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
}
