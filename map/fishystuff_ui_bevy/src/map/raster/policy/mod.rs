use std::collections::{BTreeMap, HashMap, HashSet};

use bevy::prelude::Resource;

use crate::map::layers::LayerId;
use crate::map::spaces::WorldPoint;

use super::TileKey;

mod bounds;
mod requests;
mod residency;

pub(crate) use bounds::{
    compute_cache_budget, compute_desired_layer_tiles, desired_change_is_minor, lod_signature,
    update_camera_motion_state, DesiredTileComputation,
};
pub(crate) use requests::{
    build_layer_requests, log_tile_stats, start_tile_requests, tile_should_render, BuildResult,
    LayerRequestBuild, StartTileRequests,
};
pub(crate) use residency::{
    apply_layer_residency_plan, build_layer_residency_plan, eviction_priority_score,
    incr_level_count, merge_level_counts, sum_level_counts, tile_residency_class,
};

pub(crate) const REQUEST_REFRESH_INTERVAL_FRAMES: u64 = 45;

#[derive(Resource, Default)]
pub(crate) struct LayerViewState {
    pub(crate) per_layer: HashMap<LayerId, DesiredLayerTiles>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DesiredLayerTiles {
    pub(crate) base: Option<TileBounds>,
    pub(crate) detail: Option<TileBounds>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TileBounds {
    pub(crate) min_tx: i32,
    pub(crate) max_tx: i32,
    pub(crate) min_ty: i32,
    pub(crate) max_ty: i32,
    pub(crate) z: i32,
    pub(crate) map_version: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResidencyClass {
    Protected,
    Warm,
    Evictable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Level0Rect {
    min_x: i64,
    max_x: i64,
    min_y: i64,
    max_y: i64,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct LayerResidencyPlan {
    render_visible: HashSet<TileKey>,
    protected: HashSet<TileKey>,
    warm: HashSet<TileKey>,
    fallback_visible: HashSet<TileKey>,
    ancestor_requests: HashSet<TileKey>,
    blank_visible_count: u32,
    desired_spans: Vec<Level0Rect>,
    max_detail_requests_while_moving: usize,
    motion_suppresses_refine: bool,
}

#[derive(Resource, Default)]
pub(crate) struct TileResidencyState {
    pub(crate) frame: u64,
    pub(crate) render_visible: HashSet<TileKey>,
    pub(crate) protected: HashSet<TileKey>,
    pub(crate) warm: HashSet<TileKey>,
    pub(crate) fallback_visible: HashSet<TileKey>,
    pub(crate) ancestor_requests: HashSet<TileKey>,
    pub(crate) desired_spans_by_layer: HashMap<LayerId, Vec<Level0Rect>>,
    pub(crate) protected_by_layer_level: HashMap<LayerId, BTreeMap<i32, u32>>,
    pub(crate) warm_by_layer_level: HashMap<LayerId, BTreeMap<i32, u32>>,
    pub(crate) fallback_by_layer_level: HashMap<LayerId, BTreeMap<i32, u32>>,
    pub(crate) blank_visible_by_layer: HashMap<LayerId, u32>,
    pub(crate) max_detail_requests_while_moving_by_layer: HashMap<LayerId, usize>,
    pub(crate) motion_suppresses_refine_by_layer: HashMap<LayerId, bool>,
}

impl TileResidencyState {
    pub(crate) fn begin_frame(&mut self, frame: u64) {
        self.frame = frame;
        self.render_visible.clear();
        self.protected.clear();
        self.warm.clear();
        self.fallback_visible.clear();
        self.ancestor_requests.clear();
        self.desired_spans_by_layer.clear();
        self.protected_by_layer_level.clear();
        self.warm_by_layer_level.clear();
        self.fallback_by_layer_level.clear();
        self.blank_visible_by_layer.clear();
        self.max_detail_requests_while_moving_by_layer.clear();
        self.motion_suppresses_refine_by_layer.clear();
    }
}

#[derive(Resource, Debug, Clone, Copy, Default)]
pub(crate) struct TileFrameClock {
    pub(crate) frame: u64,
}

#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct CameraMotionState {
    pub(crate) prev_center: Option<WorldPoint>,
    pub(crate) prev_diag: Option<f64>,
    pub(crate) unstable_frames_left: u32,
    pub(crate) unstable: bool,
    pub(crate) pan_fraction: f64,
    pub(crate) zoom_out_ratio: f64,
}

impl Default for CameraMotionState {
    fn default() -> Self {
        Self {
            prev_center: None,
            prev_diag: None,
            unstable_frames_left: 0,
            unstable: false,
            pan_fraction: 0.0,
            zoom_out_ratio: 0.0,
        }
    }
}

pub(super) fn merge_level_count_maps(dst: &mut BTreeMap<i32, u32>, src: &BTreeMap<i32, u32>) {
    for (level, value) in src {
        *dst.entry(*level).or_default() += *value;
    }
}

#[cfg(test)]
mod tests {
    use super::super::cache::{RasterTileCache, RasterTileEntry, TileState};
    use super::super::manifest::{LevelInfo, LoadedTileset};
    use super::*;
    use crate::map::layers::{LayerKind, PickMode};
    use crate::map::spaces::layer_transform::LayerTransform;
    use bevy::prelude::Handle;

    fn full_level(z: i32, width: u32, height: u32) -> LevelInfo {
        let bits = width as usize * height as usize;
        let mut occupancy = vec![0_u8; bits.div_ceil(8)];
        for bit in 0..bits {
            occupancy[bit >> 3] |= 1 << (bit & 7);
        }
        LevelInfo {
            z,
            min_x: 0,
            min_y: 0,
            max_x: width as i32 - 1,
            max_y: height as i32 - 1,
            width,
            height,
            tile_count: bits,
            occupancy,
        }
    }

    fn full_tileset(max_level: i32) -> LoadedTileset {
        let mut levels = Vec::new();
        for z in 0..=max_level {
            let width = 1_u32 << (max_level - z);
            levels.push(full_level(z, width, width));
        }
        LoadedTileset {
            tile_px: 512,
            max_level: max_level as u8,
            levels,
        }
    }

    fn test_layer(max_level: u8) -> crate::map::layers::LayerSpec {
        crate::map::layers::LayerSpec {
            id: LayerId::from_raw(0),
            key: "test".to_string(),
            name: "Test".to_string(),
            visible_default: true,
            opacity_default: 1.0,
            z_base: 0.0,
            kind: LayerKind::TiledRaster,
            tileset_url: "/tileset.json".to_string(),
            tile_url_template: "/tiles/{z}/{x}_{y}.png".to_string(),
            tileset_version: "v1".to_string(),
            field_source: None,
            field_metadata_source: None,
            vector_source: None,
            transform: LayerTransform::IdentityMapSpace,
            tile_px: 512,
            max_level,
            y_flip: false,
            lod_policy: crate::map::layers::LodPolicy {
                target_tiles: 64,
                hysteresis_hi: 80.0,
                hysteresis_lo: 40.0,
                margin_tiles: 0,
                enable_refine: true,
                refine_debounce_ms: 0,
                max_detail_tiles: 128,
                max_resident_tiles: 256,
                pinned_coarse_levels: 2,
                coarse_pin_min_level: None,
                warm_margin_tiles: 1,
                protected_margin_tiles: 0,
                detail_eviction_weight: 4.0,
                max_detail_requests_while_camera_moving: 1,
                motion_suppresses_refine: true,
            },
            request_weight: 1.0,
            pick_mode: PickMode::None,
            display_order: 0,
        }
    }

    fn key(z: i32, tx: i32, ty: i32) -> TileKey {
        TileKey {
            layer: LayerId::from_raw(0),
            map_version: 1,
            z,
            tx,
            ty,
        }
    }

    fn ready_entry(last_used: u64) -> RasterTileEntry {
        RasterTileEntry {
            handle: Handle::default(),
            entity: None,
            material: None,
            state: TileState::Ready,
            visible: false,
            alpha: 1.0,
            depth: 0.0,
            last_used,
            exact_quad: false,
            sprite_size: None,
            pixel_data: None,
            zone_rgbs: Vec::new(),
            zone_lookup_rows: None,
            filter_active: false,
            filter_revision: 0,
            pixel_filtered: false,
            hover_highlight_zone: None,
            clip_mask_layer: None,
            clip_mask_revision: 0,
            clip_mask_applied: false,
            linger_until_frame: 0,
        }
    }

    #[test]
    fn coarse_pinning_marks_root_as_protected() {
        let layer = test_layer(3);
        let tileset = full_tileset(3);
        let mut cache = RasterTileCache::default();
        cache.entries.insert(key(3, 0, 0), ready_entry(10));
        cache.entries.insert(key(0, 0, 0), ready_entry(20));
        let desired = DesiredLayerTiles {
            base: Some(TileBounds {
                min_tx: 0,
                max_tx: 0,
                min_ty: 0,
                max_ty: 0,
                z: 0,
                map_version: 1,
            }),
            detail: None,
        };
        let plan = build_layer_residency_plan(&layer, &tileset, desired, 1, &cache, false);
        assert!(plan.protected.contains(&key(3, 0, 0)));
    }

    #[test]
    fn finer_tiles_get_higher_eviction_score_than_coarse_tiles() {
        let layer = test_layer(3);
        let fine_key = key(0, 0, 0);
        let coarse_key = key(3, 0, 0);
        let entry = ready_entry(100);
        let residency = TileResidencyState::default();
        let fine_score = eviction_priority_score(1_000, fine_key, &entry, &residency, Some(&layer));
        let coarse_score =
            eviction_priority_score(1_000, coarse_key, &entry, &residency, Some(&layer));
        assert!(fine_score > coarse_score);
    }

    #[test]
    fn fallback_ancestor_is_protected_when_child_missing() {
        let layer = test_layer(2);
        let tileset = full_tileset(2);
        let mut cache = RasterTileCache::default();
        cache.entries.insert(key(2, 0, 0), ready_entry(1));
        let desired = DesiredLayerTiles {
            base: Some(TileBounds {
                min_tx: 1,
                max_tx: 1,
                min_ty: 1,
                max_ty: 1,
                z: 0,
                map_version: 1,
            }),
            detail: None,
        };
        let plan = build_layer_residency_plan(&layer, &tileset, desired, 1, &cache, false);
        assert!(plan.fallback_visible.contains(&key(2, 0, 0)));
        assert!(plan.protected.contains(&key(2, 0, 0)));
        assert_eq!(plan.blank_visible_count, 0);
    }

    #[test]
    fn warm_ring_keeps_nearby_tiles_warm_on_small_move() {
        let mut layer = test_layer(2);
        layer.lod_policy.warm_margin_tiles = 1;
        let tileset = full_tileset(2);
        let mut cache = RasterTileCache::default();
        cache.entries.insert(key(1, 1, 0), ready_entry(10));
        let desired = DesiredLayerTiles {
            base: Some(TileBounds {
                min_tx: 0,
                max_tx: 0,
                min_ty: 0,
                max_ty: 0,
                z: 1,
                map_version: 1,
            }),
            detail: None,
        };
        let plan = build_layer_residency_plan(&layer, &tileset, desired, 1, &cache, false);
        assert!(plan.warm.contains(&key(1, 1, 0)));
    }

    #[test]
    fn fast_zoom_out_suppresses_detail_requests() {
        let layer = test_layer(2);
        let tileset = full_tileset(2);
        let cache = RasterTileCache::default();
        let desired = DesiredLayerTiles {
            base: Some(TileBounds {
                min_tx: 0,
                max_tx: 1,
                min_ty: 0,
                max_ty: 1,
                z: 1,
                map_version: 1,
            }),
            detail: Some(TileBounds {
                min_tx: 0,
                max_tx: 3,
                min_ty: 0,
                max_ty: 3,
                z: 0,
                map_version: 1,
            }),
        };
        let mut residency = TileResidencyState::default();
        residency.blank_visible_by_layer.insert(layer.id, 4);
        residency
            .max_detail_requests_while_moving_by_layer
            .insert(layer.id, 1);
        let result = build_layer_requests(LayerRequestBuild {
            layer: &layer,
            tileset: &tileset,
            desired,
            map_version: Some("v1"),
            cache: &cache,
            map_version_id: 1,
            camera_unstable: true,
            residency: &residency,
        });
        assert_eq!(result.detail_queued, 0);
        assert!(result
            .requests
            .iter()
            .all(|request| request.kind != crate::map::streaming::RequestKind::DetailRefine));
    }

    #[test]
    fn no_gap_if_any_ancestor_loaded() {
        let layer = test_layer(2);
        let tileset = full_tileset(2);
        let mut cache = RasterTileCache::default();
        cache.entries.insert(key(2, 0, 0), ready_entry(1));
        let desired = DesiredLayerTiles {
            base: None,
            detail: Some(TileBounds {
                min_tx: 2,
                max_tx: 2,
                min_ty: 2,
                max_ty: 2,
                z: 0,
                map_version: 1,
            }),
        };
        let plan = build_layer_residency_plan(&layer, &tileset, desired, 1, &cache, false);
        assert_eq!(plan.blank_visible_count, 0);
        assert!(plan.render_visible.contains(&key(2, 0, 0)));
    }
}
