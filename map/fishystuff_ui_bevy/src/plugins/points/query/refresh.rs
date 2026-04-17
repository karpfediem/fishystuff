use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use fishystuff_api::models::events::{EventsQueryMode, MapBboxPx};

use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::events::{
    cluster_view_events, suggested_cluster_bucket_px, EventZoneSetResolver, EventsSnapshotState,
    LocalEventQuery, VisibleTileScope, VISIBLE_TILE_SCOPE_PX,
};
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_view::loaded_field_layer;
use crate::map::layers::{LayerRegistry, LayerRuntime, FISH_EVIDENCE_LAYER_KEY};
use crate::plugins::api::{
    FishFilterState, LayerEffectiveFilterState, MapDisplayState, PatchFilterState,
    SemanticFieldFilterState,
};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::points::EvidenceZoneFilter;

use super::super::render::view_bbox_map_px;
use super::state::PointsQuerySignature;
use super::{
    normalized_time_and_fish_filters, quantize_px, PointsState, RenderPoint, VIEWPORT_SIG_STEP_PX,
};

pub(in crate::plugins::points) fn refresh_points_from_local_snapshot(
    mut refresh: LocalSnapshotRefresh<'_, '_>,
) {
    crate::perf_scope!("events.snapshot_query_refresh");
    let fish_evidence_visible = refresh
        .layer_registry
        .id_by_key(FISH_EVIDENCE_LAYER_KEY)
        .map(|id| refresh.layer_runtime.visible(id))
        .unwrap_or(refresh.display_state.show_points);
    if !refresh.display_state.show_points
        || !fish_evidence_visible
        || refresh.view_mode.mode != ViewMode::Map2D
    {
        refresh.points.status = "points: hidden".to_string();
        refresh.points.request_sig = None;
        return;
    }

    if refresh.snapshot.loading && !refresh.snapshot.loaded {
        refresh.points.status = "points: snapshot loading".to_string();
        clear_render_points(&mut refresh.points);
        return;
    }
    if refresh.snapshot.failed && !refresh.snapshot.loaded {
        refresh.points.status = format!(
            "points: snapshot failed ({})",
            refresh
                .snapshot
                .last_error
                .as_deref()
                .unwrap_or("unknown snapshot error")
        );
        clear_render_points(&mut refresh.points);
        return;
    }
    if !refresh.snapshot.loaded {
        refresh.points.status = "points: snapshot pending".to_string();
        clear_render_points(&mut refresh.points);
        return;
    }

    let Some(viewport_bbox) = view_bbox_map_px(&refresh.windows, &refresh.camera_q) else {
        refresh.points.status = "points: missing viewport".to_string();
        clear_render_points(&mut refresh.points);
        return;
    };

    let Some((from_ts_utc, to_ts_utc, mut fish_ids)) =
        normalized_time_and_fish_filters(&refresh.patch_filter, &refresh.fish_filter)
    else {
        refresh.points.status = "points: missing range".to_string();
        clear_render_points(&mut refresh.points);
        return;
    };
    fish_ids.sort_unstable();
    fish_ids.dedup();

    let tile_scope = MapBboxPx {
        min_x: viewport_bbox.min_x,
        min_y: viewport_bbox.min_y,
        max_x: viewport_bbox.max_x,
        max_y: viewport_bbox.max_y,
    };
    let cluster_bucket_px = suggested_cluster_bucket_px(&viewport_bbox);
    let inactive_filter = EvidenceZoneFilter::default();
    let zone_filter = refresh
        .layer_filters
        .zone_membership_filter(FISH_EVIDENCE_LAYER_KEY)
        .unwrap_or(&inactive_filter);
    let zone_mask_layer = refresh
        .layer_registry
        .get_by_key(SemanticFieldFilterState::ZONE_MASK_LAYER_ID);
    let zone_lookup_url = zone_mask_layer.and_then(|layer| layer.field_url());
    let zone_mask_field =
        zone_mask_layer.and_then(|layer| loaded_field_layer(layer, refresh.exact_lookups.as_ref()));
    let mut zone_set_resolver = EventZoneSetResolver::new(zone_mask_field);
    let signature = PointsQuerySignature {
        revision: refresh.snapshot.revision.clone(),
        zone_filter_revision: if zone_filter.active {
            zone_filter.revision
        } else {
            0
        },
        zone_lookup_url: zone_lookup_url.clone(),
        zone_lookup_ready: zone_filter.active && zone_mask_field.is_some(),
        from_ts_utc,
        to_ts_utc,
        fish_ids: fish_ids.clone(),
        viewport_qmin_x: quantize_px(viewport_bbox.min_x, VIEWPORT_SIG_STEP_PX),
        viewport_qmin_y: quantize_px(viewport_bbox.min_y, VIEWPORT_SIG_STEP_PX),
        viewport_qmax_x: quantize_px(viewport_bbox.max_x, VIEWPORT_SIG_STEP_PX),
        viewport_qmax_y: quantize_px(viewport_bbox.max_y, VIEWPORT_SIG_STEP_PX),
        tile_scope_min_x: quantize_px(tile_scope.min_x, VISIBLE_TILE_SCOPE_PX),
        tile_scope_min_y: quantize_px(tile_scope.min_y, VISIBLE_TILE_SCOPE_PX),
        tile_scope_max_x: quantize_px(tile_scope.max_x, VISIBLE_TILE_SCOPE_PX),
        tile_scope_max_y: quantize_px(tile_scope.max_y, VISIBLE_TILE_SCOPE_PX),
        cluster_bucket_px,
    };

    if refresh.points.request_sig.as_ref() == Some(&signature) {
        refresh.points.status = points_status_line(&refresh.points, &refresh.snapshot);
        return;
    }

    let local_query = LocalEventQuery {
        bbox: &viewport_bbox,
        from_ts_utc,
        to_ts_utc,
        fish_ids: fish_ids.as_slice(),
        zone_rgbs: (zone_filter.active && zone_mask_field.is_none())
            .then_some(&zone_filter.zone_rgbs),
        tile_scope: Some(VisibleTileScope::from_bbox(
            &tile_scope,
            VISIBLE_TILE_SCOPE_PX,
        )),
    };
    let mut selection = refresh.snapshot.select_for_view(&local_query);
    if zone_filter.active && zone_mask_field.is_some() {
        selection.filtered_indices.retain(|idx| {
            refresh.snapshot.events.get(*idx).is_some_and(|event| {
                zone_set_resolver
                    .zone_rgbs(event)
                    .iter()
                    .any(|zone_rgb| zone_filter.zone_rgbs.contains(zone_rgb))
            })
        });
    }
    let clustered = {
        crate::perf_scope!("events.clustering");
        cluster_view_events(
            &refresh.snapshot.events,
            &selection.filtered_indices,
            cluster_bucket_px,
        )
    };
    let rendered_points: Vec<RenderPoint> = clustered
        .points
        .into_iter()
        .map(|point| RenderPoint {
            map_px_x: point.map_px_x,
            map_px_y: point.map_px_y,
            world_x: point.world_x,
            world_z: point.world_z,
            fish_id: point.fish_id,
            zone_rgb_u32: None,
            sample_count: point.sample_count,
            aggregated: point.aggregated,
        })
        .collect();

    refresh.points.request_sig = Some(signature);
    refresh.points.points = rendered_points;
    refresh.points.mode = Some(clustered.mode);
    refresh.points.bucket_px = clustered.cluster_bucket_px;
    refresh.points.sample_step = 1;
    refresh.points.total = selection.filtered_indices.len();
    refresh.points.represented_sample_count = clustered.represented_event_count;
    refresh.points.candidate_count = selection.candidate_count;
    refresh.points.rendered_point_count = clustered.rendered_point_count;
    refresh.points.rendered_cluster_count = clustered.rendered_cluster_count;
    refresh.points.spatial_bucket_px = refresh.snapshot.spatial_index.bucket_px;
    crate::perf_gauge!(
        "events.cluster_count",
        refresh.points.rendered_cluster_count
    );
    crate::perf_gauge!(
        "events.raw_point_count",
        refresh.points.rendered_point_count
    );
    refresh.points.status = points_status_line(&refresh.points, &refresh.snapshot);
    refresh.points.dirty = true;
}

#[derive(SystemParam)]
pub(in crate::plugins::points) struct LocalSnapshotRefresh<'w, 's> {
    points: ResMut<'w, PointsState>,
    patch_filter: Res<'w, PatchFilterState>,
    fish_filter: Res<'w, FishFilterState>,
    layer_filters: Res<'w, LayerEffectiveFilterState>,
    display_state: Res<'w, MapDisplayState>,
    view_mode: Res<'w, ViewModeState>,
    layer_registry: Res<'w, LayerRegistry>,
    layer_runtime: Res<'w, LayerRuntime>,
    exact_lookups: Res<'w, ExactLookupCache>,
    snapshot: Res<'w, EventsSnapshotState>,
    windows: Query<'w, 's, &'static Window>,
    camera_q: Query<'w, 's, (&'static Camera, &'static Transform), With<Map2dCamera>>,
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use fishystuff_api::models::events::EventPointCompact;
    use fishystuff_core::field::DiscreteFieldRows;

    use super::*;
    use crate::map::exact_lookup::ExactLookupCache;
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
    fn exact_lookup_zone_filter_keeps_events_with_matching_zone_mask_even_without_event_zone_ids() {
        let layer = test_zone_mask_layer();
        let mut exact_lookups = ExactLookupCache::default();
        let mut zone_rows = vec![0_u32; 9 * 9];
        for (x, y) in [
            (3_usize, 3_usize),
            (4, 3),
            (5, 3),
            (3, 4),
            (5, 4),
            (3, 5),
            (4, 5),
            (5, 5),
        ] {
            zone_rows[y * 9 + x] = 0x222222;
        }
        exact_lookups.insert_ready(
            layer.id,
            layer.field_url().expect("field url"),
            DiscreteFieldRows::from_u32_grid(9, 9, &zone_rows).expect("field"),
        );
        let zone_mask_field = loaded_field_layer(&layer, &exact_lookups);
        let mut resolver = EventZoneSetResolver::new(zone_mask_field);
        let events = [
            EventPointCompact {
                event_id: 1,
                fish_id: 10,
                ts_utc: 100,
                map_px_x: 4,
                map_px_y: 4,
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
                ts_utc: 100,
                map_px_x: 0,
                map_px_y: 0,
                length_milli: 1,
                world_x: None,
                world_z: None,
                zone_rgb_u32: Some(0x222222),
                source_kind: None,
                source_id: None,
            },
        ];
        let mut filtered_indices = vec![0, 1];
        let zone_rgbs = HashSet::from([0x222222]);

        filtered_indices.retain(|idx| {
            events.get(*idx).is_some_and(|event| {
                resolver
                    .zone_rgbs(event)
                    .iter()
                    .any(|zone_rgb| zone_rgbs.contains(zone_rgb))
            })
        });

        assert_eq!(filtered_indices, vec![0]);
    }
}

fn clear_render_points(points: &mut PointsState) {
    points.request_sig = None;
    if !points.points.is_empty() {
        points.points.clear();
        points.dirty = true;
    }
    points.total = 0;
    points.represented_sample_count = 0;
    points.mode = None;
    points.bucket_px = None;
    points.sample_step = 1;
    points.candidate_count = 0;
    points.rendered_point_count = 0;
    points.rendered_cluster_count = 0;
    points.spatial_bucket_px = 0;
}

fn points_status_line(points: &PointsState, snapshot: &EventsSnapshotState) -> String {
    let revision = snapshot.revision.as_deref().unwrap_or("-");
    let mode = match points.mode {
        Some(EventsQueryMode::Raw) => "raw",
        Some(EventsQueryMode::GridAggregate) => "grid_aggregate",
        None => "-",
    };
    let cluster_bucket = points
        .bucket_px
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    format!(
        "points: mode={} rev={} snapshot_events={} idx_bucket={} cluster_bucket={} candidates={} represented={} rendered_points={} rendered_clusters={} snapshot={}",
        mode,
        revision,
        snapshot.event_count,
        snapshot.spatial_index.bucket_px,
        cluster_bucket,
        points.candidate_count,
        points.represented_sample_count,
        points.rendered_point_count,
        points.rendered_cluster_count,
        snapshot.last_load_kind.label(),
    )
}
