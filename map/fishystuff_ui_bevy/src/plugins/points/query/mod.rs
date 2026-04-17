use crate::plugins::api::{FishFilterState, PatchFilterState};

mod evidence;
mod refresh;
mod state;

pub use state::{EvidenceZoneFilter, PointsState, RenderPoint};

pub(crate) use evidence::{sync_evidence_zone_filter, sync_layer_effective_filters};
pub(super) use refresh::refresh_points_from_local_snapshot;

const VIEWPORT_SIG_STEP_PX: i32 = 32;

fn normalized_time_and_fish_filters(
    patch_filter: &PatchFilterState,
    fish_filter: &FishFilterState,
) -> Option<(i64, i64, Vec<i32>)> {
    let from_ts_utc = patch_filter.from_ts?;
    let to_ts_utc = patch_filter.to_ts?;
    if from_ts_utc >= to_ts_utc {
        return None;
    }
    let mut fish_ids = fish_filter.selected_fish_ids.clone();
    fish_ids.sort_unstable();
    fish_ids.dedup();
    Some((from_ts_utc, to_ts_utc, fish_ids))
}

fn quantize_px(value: i32, step: i32) -> i32 {
    value.div_euclid(step.max(1))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use fishystuff_api::models::events::EventPointCompact;
    use fishystuff_core::field::DiscreteFieldRows;

    use super::*;
    use crate::map::events::EventZoneSetResolver;
    use crate::map::exact_lookup::ExactLookupCache;
    use crate::map::field_view::loaded_field_layer;
    use crate::map::layers::{LayerId, LayerKind, LayerSpec, LodPolicy, PickMode};
    use crate::map::spaces::layer_transform::LayerTransform;

    fn test_zone_mask_layer() -> LayerSpec {
        LayerSpec {
            id: LayerId::from_raw(7),
            key: "zone_mask".to_string(),
            name: "Zone Mask".to_string(),
            visible_default: true,
            opacity_default: 1.0,
            z_base: 0.0,
            kind: LayerKind::TiledRaster,
            tileset_url: "/images/tiles/zone_mask_visual/v1/tileset.json".to_string(),
            tile_url_template: "/images/tiles/zone_mask_visual/v1/{z}/{x}_{y}.png".to_string(),
            tileset_version: "v1".to_string(),
            vector_source: None,
            waypoint_source: None,
            transform: LayerTransform::IdentityMapSpace,
            tile_px: 2048,
            max_level: 0,
            y_flip: false,
            field_source: None,
            field_metadata_source: None,
            filter_bindings: Vec::new(),
            lod_policy: LodPolicy {
                target_tiles: 64,
                hysteresis_hi: 80.0,
                hysteresis_lo: 40.0,
                margin_tiles: 0,
                enable_refine: false,
                refine_debounce_ms: 0,
                max_detail_tiles: 128,
                max_resident_tiles: 256,
                pinned_coarse_levels: 0,
                coarse_pin_min_level: None,
                warm_margin_tiles: 1,
                protected_margin_tiles: 0,
                detail_eviction_weight: 4.0,
                max_detail_requests_while_camera_moving: 1,
                motion_suppresses_refine: true,
            },
            request_weight: 1.0,
            pick_mode: PickMode::ExactTilePixel,
            display_order: 0,
        }
    }

    #[test]
    fn quantized_signature_ignores_sub_step_viewport_motion() {
        let sig_a = state::PointsQuerySignature {
            revision: Some("r1".to_string()),
            zone_filter_revision: 0,
            zone_lookup_url: None,
            zone_lookup_ready: false,
            from_ts_utc: 10,
            to_ts_utc: 20,
            fish_ids: vec![100],
            viewport_qmin_x: quantize_px(100, VIEWPORT_SIG_STEP_PX),
            viewport_qmin_y: quantize_px(100, VIEWPORT_SIG_STEP_PX),
            viewport_qmax_x: quantize_px(500, VIEWPORT_SIG_STEP_PX),
            viewport_qmax_y: quantize_px(500, VIEWPORT_SIG_STEP_PX),
            tile_scope_min_x: quantize_px(100, crate::map::events::VISIBLE_TILE_SCOPE_PX),
            tile_scope_min_y: quantize_px(100, crate::map::events::VISIBLE_TILE_SCOPE_PX),
            tile_scope_max_x: quantize_px(500, crate::map::events::VISIBLE_TILE_SCOPE_PX),
            tile_scope_max_y: quantize_px(500, crate::map::events::VISIBLE_TILE_SCOPE_PX),
            cluster_bucket_px: 64,
        };
        let sig_b = state::PointsQuerySignature {
            revision: Some("r1".to_string()),
            zone_filter_revision: 0,
            zone_lookup_url: None,
            zone_lookup_ready: false,
            from_ts_utc: 10,
            to_ts_utc: 20,
            fish_ids: vec![100],
            viewport_qmin_x: quantize_px(111, VIEWPORT_SIG_STEP_PX),
            viewport_qmin_y: quantize_px(119, VIEWPORT_SIG_STEP_PX),
            viewport_qmax_x: quantize_px(510, VIEWPORT_SIG_STEP_PX),
            viewport_qmax_y: quantize_px(510, VIEWPORT_SIG_STEP_PX),
            tile_scope_min_x: quantize_px(111, crate::map::events::VISIBLE_TILE_SCOPE_PX),
            tile_scope_min_y: quantize_px(119, crate::map::events::VISIBLE_TILE_SCOPE_PX),
            tile_scope_max_x: quantize_px(510, crate::map::events::VISIBLE_TILE_SCOPE_PX),
            tile_scope_max_y: quantize_px(510, crate::map::events::VISIBLE_TILE_SCOPE_PX),
            cluster_bucket_px: 64,
        };
        assert_eq!(sig_a, sig_b);
    }

    #[test]
    fn normalized_time_and_fish_filters_sorts_and_dedups_ids() {
        let mut patch_filter = PatchFilterState::default();
        patch_filter.from_ts = Some(100);
        patch_filter.to_ts = Some(200);
        let mut fish_filter = FishFilterState::default();
        fish_filter.selected_fish_ids = vec![20, 10, 20];
        let (_, _, ids) =
            normalized_time_and_fish_filters(&patch_filter, &fish_filter).expect("filters");
        assert_eq!(ids, vec![10, 20]);
    }

    #[test]
    fn collect_evidence_zone_rgbs_filters_by_time_and_fish() {
        let events = vec![
            EventPointCompact {
                event_id: 1,
                fish_id: 10,
                ts_utc: 100,
                map_px_x: 10,
                map_px_y: 20,
                length_milli: 1,
                world_x: None,
                world_z: None,
                zone_rgb_u32: Some(0x112233),
                source_kind: None,
                source_id: None,
            },
            EventPointCompact {
                event_id: 2,
                fish_id: 10,
                ts_utc: 150,
                map_px_x: 10,
                map_px_y: 20,
                length_milli: 1,
                world_x: None,
                world_z: None,
                zone_rgb_u32: Some(0x445566),
                source_kind: None,
                source_id: None,
            },
            EventPointCompact {
                event_id: 3,
                fish_id: 20,
                ts_utc: 150,
                map_px_x: 10,
                map_px_y: 20,
                length_milli: 1,
                world_x: None,
                world_z: None,
                zone_rgb_u32: Some(0x778899),
                source_kind: None,
                source_id: None,
            },
            EventPointCompact {
                event_id: 4,
                fish_id: 10,
                ts_utc: 160,
                map_px_x: 10,
                map_px_y: 20,
                length_milli: 1,
                world_x: None,
                world_z: None,
                zone_rgb_u32: None,
                source_kind: None,
                source_id: None,
            },
        ];

        let mut resolver = EventZoneSetResolver::new(None);
        let (zones, has_zone_data, matched_events) =
            evidence::collect_evidence_zone_rgbs(&events, 120, 170, &[10], &mut resolver);

        assert_eq!(matched_events, 2);
        assert!(has_zone_data);
        assert_eq!(zones, HashSet::from([0x445566]));
    }

    #[test]
    fn collect_evidence_zone_rgbs_prefers_zone_mask_exact_lookup() {
        let layer = test_zone_mask_layer();
        let mut exact_lookups = ExactLookupCache::default();
        exact_lookups.insert_ready(
            layer.id,
            layer.field_url().expect("field url"),
            DiscreteFieldRows::from_u32_grid(
                5,
                5,
                &[
                    0, 0, 0x654321, 0x123456, 0x123456, 0, 0, 0x654321, 0x123456, 0x123456, 0, 0,
                    0x654321, 0x123456, 0x123456, 0, 0, 0x654321, 0x123456, 0x123456, 0, 0,
                    0x654321, 0x123456, 0x123456,
                ],
            )
            .expect("field"),
        );
        let zone_mask_field = loaded_field_layer(&layer, &exact_lookups);
        let mut resolver = EventZoneSetResolver::new(zone_mask_field);
        let events = vec![
            EventPointCompact {
                event_id: 1,
                fish_id: 10,
                ts_utc: 150,
                map_px_x: 2,
                map_px_y: 2,
                length_milli: 1,
                world_x: None,
                world_z: None,
                zone_rgb_u32: None,
                source_kind: None,
                source_id: None,
            },
            EventPointCompact {
                event_id: 2,
                fish_id: 10,
                ts_utc: 151,
                map_px_x: 0,
                map_px_y: 0,
                length_milli: 1,
                world_x: None,
                world_z: None,
                zone_rgb_u32: Some(0xaabbcc),
                source_kind: None,
                source_id: None,
            },
        ];

        let (zones, has_zone_data, matched_events) =
            evidence::collect_evidence_zone_rgbs(&events, 120, 170, &[10], &mut resolver);

        assert_eq!(matched_events, 2);
        assert!(has_zone_data);
        assert_eq!(zones, HashSet::from([0x123456, 0x654321]));
    }

    #[test]
    fn collect_evidence_zone_rgbs_falls_back_to_event_zone_when_zone_mask_is_unavailable() {
        let events = vec![EventPointCompact {
            event_id: 1,
            fish_id: 10,
            ts_utc: 150,
            map_px_x: 1,
            map_px_y: 0,
            length_milli: 1,
            world_x: None,
            world_z: None,
            zone_rgb_u32: Some(0x112233),
            source_kind: None,
            source_id: None,
        }];

        let mut resolver = EventZoneSetResolver::new(None);
        let (zones, has_zone_data, matched_events) =
            evidence::collect_evidence_zone_rgbs(&events, 120, 170, &[10], &mut resolver);

        assert_eq!(matched_events, 1);
        assert!(has_zone_data);
        assert_eq!(zones, HashSet::from([0x112233]));
    }
}
