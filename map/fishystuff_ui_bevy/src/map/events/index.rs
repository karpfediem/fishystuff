use std::collections::HashMap;
use std::collections::HashSet;

use fishystuff_api::models::events::{EventPointCompact, MapBboxPx};

pub const SPATIAL_BUCKET_PX: i32 = 128;
pub const VISIBLE_TILE_SCOPE_PX: i32 = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct BucketKey {
    bx: i32,
    by: i32,
}

#[derive(Debug, Clone)]
pub struct SpatialIndex {
    pub bucket_px: i32,
    buckets: HashMap<BucketKey, Vec<usize>>,
}

impl SpatialIndex {
    pub fn new(bucket_px: i32) -> Self {
        Self {
            bucket_px: bucket_px.max(1),
            buckets: HashMap::new(),
        }
    }

    pub fn rebuild(&mut self, events: &[EventPointCompact]) {
        self.buckets.clear();
        for (idx, event) in events.iter().enumerate() {
            let key = bucket_key(event.map_px_x, event.map_px_y, self.bucket_px);
            self.buckets.entry(key).or_default().push(idx);
        }
    }

    pub fn query_bbox(&self, bbox: &MapBboxPx, events: &[EventPointCompact]) -> Vec<usize> {
        let min_x = bbox.min_x.min(bbox.max_x);
        let max_x = bbox.min_x.max(bbox.max_x);
        let min_y = bbox.min_y.min(bbox.max_y);
        let max_y = bbox.min_y.max(bbox.max_y);
        let min_bx = min_x.div_euclid(self.bucket_px);
        let max_bx = max_x.div_euclid(self.bucket_px);
        let min_by = min_y.div_euclid(self.bucket_px);
        let max_by = max_y.div_euclid(self.bucket_px);

        let mut out = Vec::new();
        for by in min_by..=max_by {
            for bx in min_bx..=max_bx {
                let Some(indices) = self.buckets.get(&BucketKey { bx, by }) else {
                    continue;
                };
                for idx in indices {
                    let Some(event) = events.get(*idx) else {
                        continue;
                    };
                    if event.map_px_x < min_x
                        || event.map_px_x > max_x
                        || event.map_px_y < min_y
                        || event.map_px_y > max_y
                    {
                        continue;
                    }
                    out.push(*idx);
                }
            }
        }
        out
    }
}

fn bucket_key(map_px_x: i32, map_px_y: i32, bucket_px: i32) -> BucketKey {
    BucketKey {
        bx: map_px_x.div_euclid(bucket_px),
        by: map_px_y.div_euclid(bucket_px),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VisibleTileScope {
    min_tx: i32,
    max_tx: i32,
    min_ty: i32,
    max_ty: i32,
    tile_px: i32,
}

impl VisibleTileScope {
    pub fn from_bbox(bbox: &MapBboxPx, tile_px: i32) -> Self {
        let tile_px = tile_px.max(1);
        let min_x = bbox.min_x.min(bbox.max_x);
        let max_x = bbox.min_x.max(bbox.max_x);
        let min_y = bbox.min_y.min(bbox.max_y);
        let max_y = bbox.min_y.max(bbox.max_y);
        Self {
            min_tx: min_x.div_euclid(tile_px),
            max_tx: max_x.div_euclid(tile_px),
            min_ty: min_y.div_euclid(tile_px),
            max_ty: max_y.div_euclid(tile_px),
            tile_px,
        }
    }

    pub fn contains(self, map_px_x: i32, map_px_y: i32) -> bool {
        let tx = map_px_x.div_euclid(self.tile_px);
        let ty = map_px_y.div_euclid(self.tile_px);
        tx >= self.min_tx && tx <= self.max_tx && ty >= self.min_ty && ty <= self.max_ty
    }
}

#[derive(Debug, Clone)]
pub struct LocalEventQuery<'a> {
    pub bbox: &'a MapBboxPx,
    pub from_ts_utc: Option<i64>,
    pub to_ts_utc: Option<i64>,
    pub fish_ids: &'a [i32],
    pub zone_rgbs: Option<&'a HashSet<u32>>,
    pub tile_scope: Option<VisibleTileScope>,
}

#[derive(Debug, Clone, Default)]
pub struct ViewSelection {
    pub candidate_count: usize,
    pub filtered_indices: Vec<usize>,
}

#[cfg(test)]
mod tests {
    use super::SpatialIndex;
    use fishystuff_api::models::events::{EventPointCompact, MapBboxPx};

    fn sample_events() -> Vec<EventPointCompact> {
        vec![
            EventPointCompact {
                event_id: 1,
                fish_id: 101,
                ts_utc: 100,
                map_px_x: 100,
                map_px_y: 100,
                length_milli: 1000,
                world_x: Some(1000),
                world_z: Some(2000),
                zone_rgb_u32: None,
                source_kind: None,
                source_id: None,
            },
            EventPointCompact {
                event_id: 2,
                fish_id: 202,
                ts_utc: 200,
                map_px_x: 130,
                map_px_y: 110,
                length_milli: 1200,
                world_x: Some(1300),
                world_z: Some(2200),
                zone_rgb_u32: None,
                source_kind: None,
                source_id: None,
            },
            EventPointCompact {
                event_id: 3,
                fish_id: 101,
                ts_utc: 300,
                map_px_x: 3200,
                map_px_y: 4200,
                length_milli: 1500,
                world_x: None,
                world_z: None,
                zone_rgb_u32: None,
                source_kind: None,
                source_id: None,
            },
        ]
    }

    #[test]
    fn spatial_index_query_bbox_returns_expected_candidates() {
        let events = sample_events();
        let mut index = SpatialIndex::new(128);
        index.rebuild(&events);
        let bbox = MapBboxPx {
            min_x: 90,
            min_y: 90,
            max_x: 160,
            max_y: 130,
        };

        let out = index.query_bbox(&bbox, &events);
        assert_eq!(out, vec![0, 1]);
    }
}
