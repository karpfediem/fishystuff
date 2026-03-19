use crate::plugins::api::{FishFilterState, PatchFilterState};

mod evidence;
mod refresh;
mod state;

pub use state::{EvidenceZoneFilter, PointsState, RenderPoint};

pub(super) use evidence::sync_evidence_zone_filter;
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

    use super::*;

    #[test]
    fn quantized_signature_ignores_sub_step_viewport_motion() {
        let sig_a = state::PointsQuerySignature {
            revision: Some("r1".to_string()),
            zone_filter_revision: 0,
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

        let (zones, has_zone_data, matched_events) =
            evidence::collect_evidence_zone_rgbs(&events, 120, 170, &[10]);

        assert_eq!(matched_events, 2);
        assert!(has_zone_data);
        assert_eq!(zones, HashSet::from([0x445566]));
    }

    #[test]
    fn merge_zone_terms_unions_explicit_and_fish_evidence_zones() {
        let merged = evidence::merge_zone_terms(
            &HashSet::from([0xaabbcc]),
            HashSet::from([0x112233, 0x445566]),
            true,
            2,
        );

        assert_eq!(merged, HashSet::from([0x112233, 0x445566, 0xaabbcc]));
    }
}
