use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use fishystuff_api::models::events::EventPointCompact;
use fishystuff_api::models::events::{EventsQueryMode, MapBboxPx};

use crate::bridge::contract::FishyMapSearchExpressionNode;
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::events::{
    cluster_view_events, suggested_cluster_bucket_px, EventsSnapshotState, ViewSelection,
    VisibleTileScope, VISIBLE_TILE_SCOPE_PX,
};
use crate::map::layers::{LayerRegistry, LayerRuntime, FISH_EVIDENCE_LAYER_KEY};
use crate::map::search_filters::{
    effective_search_expression, project_expression_for_zone_membership, search_expression_key,
    LayerSearchEvaluator,
};
use crate::plugins::api::{
    CommunityFishZoneSupportIndex, FishCatalog, FishFilterState, LayerEffectiveFilterState,
    LayerFilterBindingOverrideState, MapDisplayState, PatchFilterState, SearchExpressionState,
    SemanticFieldFilterState, ZoneMembershipFilter,
};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::points::query::evidence::zone_membership_binding_support;

use super::super::render::view_bbox_map_px;
use super::state::PointsQuerySignature;
use super::{PointRenderState, PointsState, RenderPoint};

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
        clear_render_points(&mut refresh.points, &mut refresh.render_state);
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
        clear_render_points(&mut refresh.points, &mut refresh.render_state);
        return;
    }
    if !refresh.snapshot.loaded {
        refresh.points.status = "points: snapshot pending".to_string();
        clear_render_points(&mut refresh.points, &mut refresh.render_state);
        return;
    }
    let Some(viewport_bbox) = view_bbox_map_px(&refresh.windows, &refresh.camera_q) else {
        refresh.points.status = "points: missing viewport".to_string();
        clear_render_points(&mut refresh.points, &mut refresh.render_state);
        return;
    };

    let Some((from_ts_utc, to_ts_utc)) = normalized_time_bounds(&refresh.patch_filter) else {
        refresh.points.status = "points: missing range".to_string();
        clear_render_points(&mut refresh.points, &mut refresh.render_state);
        return;
    };
    let expression = effective_search_expression(
        &crate::bridge::contract::FishyMapInputState {
            filters: crate::bridge::contract::FishyMapFiltersState {
                search_expression: refresh.search_expression.expression.clone(),
                ..Default::default()
            },
            ui: crate::bridge::contract::FishyMapUiState {
                shared_fish_state: refresh.search_expression.shared_fish_state.clone(),
                ..Default::default()
            },
            ..Default::default()
        },
        &refresh.fish_filter.selected_fish_ids,
        &refresh.semantic_filter.selected_field_ids_by_layer,
    );
    let fish_layer_expression = refresh
        .layer_registry
        .get_by_key(FISH_EVIDENCE_LAYER_KEY)
        .and_then(|layer| {
            project_expression_for_zone_membership(
                &expression,
                zone_membership_binding_support(layer, &refresh.binding_overrides),
            )
        });
    let mut signature_fish_ids = refresh.fish_filter.selected_fish_ids.clone();
    signature_fish_ids.sort_unstable();
    signature_fish_ids.dedup();

    let cluster_bucket_px = suggested_cluster_bucket_px(&viewport_bbox);
    let query_bbox = stable_point_query_bbox(&viewport_bbox, cluster_bucket_px);
    let inactive_filter = ZoneMembershipFilter::default();
    let zone_filter = refresh
        .layer_filters
        .zone_membership_filter(FISH_EVIDENCE_LAYER_KEY)
        .unwrap_or(&inactive_filter);
    let signature = PointsQuerySignature {
        revision: refresh.snapshot.revision.clone(),
        zone_filter_revision: if zone_filter.active {
            zone_filter.revision
        } else {
            0
        },
        zone_lookup_url: None,
        zone_lookup_ready: false,
        from_ts_utc,
        to_ts_utc,
        fish_ids: signature_fish_ids,
        search_expression_key: fish_layer_expression
            .as_ref()
            .map(search_expression_key)
            .unwrap_or_default(),
        viewport_min_x: query_bbox.min_x,
        viewport_min_y: query_bbox.min_y,
        viewport_max_x: query_bbox.max_x,
        viewport_max_y: query_bbox.max_y,
        tile_scope_min_x: query_bbox.min_x,
        tile_scope_min_y: query_bbox.min_y,
        tile_scope_max_x: query_bbox.max_x,
        tile_scope_max_y: query_bbox.max_y,
        cluster_bucket_px,
    };

    if refresh.points.request_sig.as_ref() == Some(&signature) {
        refresh.points.status = points_status_line(&refresh.points, &refresh.snapshot);
        return;
    }

    let visible_scope = VisibleTileScope::from_bbox(&query_bbox, VISIBLE_TILE_SCOPE_PX);
    let selection = select_snapshot_indices_for_point_layer(
        &refresh.snapshot,
        &query_bbox,
        visible_scope,
        from_ts_utc,
        to_ts_utc,
        fish_layer_expression.as_ref(),
        &refresh.fish_catalog,
        &refresh.search_expression,
    );
    let clustered = {
        crate::perf_scope!("events.clustering");
        cluster_view_events(
            &refresh.snapshot.events,
            &selection.filtered_indices,
            &viewport_bbox,
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
            event_indices: point.event_indices,
            point_samples: point.point_samples,
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
    refresh.render_state.dirty = true;
}

#[derive(SystemParam)]
pub(in crate::plugins::points) struct LocalSnapshotRefresh<'w, 's> {
    points: ResMut<'w, PointsState>,
    render_state: ResMut<'w, PointRenderState>,
    patch_filter: Res<'w, PatchFilterState>,
    fish_filter: Res<'w, FishFilterState>,
    semantic_filter: Res<'w, SemanticFieldFilterState>,
    search_expression: Res<'w, SearchExpressionState>,
    binding_overrides: Res<'w, LayerFilterBindingOverrideState>,
    fish_catalog: Res<'w, FishCatalog>,
    layer_filters: Res<'w, LayerEffectiveFilterState>,
    display_state: Res<'w, MapDisplayState>,
    view_mode: Res<'w, ViewModeState>,
    layer_registry: Res<'w, LayerRegistry>,
    layer_runtime: Res<'w, LayerRuntime>,
    snapshot: Res<'w, EventsSnapshotState>,
    windows: Query<'w, 's, &'static Window>,
    camera_q: Query<'w, 's, (&'static Camera, &'static Transform), With<Map2dCamera>>,
}

fn normalized_time_bounds(patch_filter: &PatchFilterState) -> Option<(Option<i64>, Option<i64>)> {
    if patch_filter
        .from_ts
        .zip(patch_filter.to_ts)
        .is_some_and(|(from_ts_utc, to_ts_utc)| from_ts_utc >= to_ts_utc)
    {
        return None;
    }
    Some((patch_filter.from_ts, patch_filter.to_ts))
}

fn stable_point_query_bbox(viewport_bbox: &MapBboxPx, cluster_bucket_px: i32) -> MapBboxPx {
    let (min_x, max_x) = normalized_axis(viewport_bbox.min_x, viewport_bbox.max_x);
    let (min_y, max_y) = normalized_axis(viewport_bbox.min_y, viewport_bbox.max_y);
    let width = (max_x - min_x + 1).max(1);
    let height = (max_y - min_y + 1).max(1);
    let snap_px = stable_query_snap_px(width.max(height), cluster_bucket_px);
    let expanded_min_x = min_x.saturating_sub(width);
    let expanded_max_x = max_x.saturating_add(width);
    let expanded_min_y = min_y.saturating_sub(height);
    let expanded_max_y = max_y.saturating_add(height);
    MapBboxPx {
        min_x: clamp_i64_to_i32(expanded_min_x.div_euclid(snap_px) * snap_px),
        min_y: clamp_i64_to_i32(expanded_min_y.div_euclid(snap_px) * snap_px),
        max_x: clamp_i64_to_i32((expanded_max_x.div_euclid(snap_px) + 1) * snap_px - 1),
        max_y: clamp_i64_to_i32((expanded_max_y.div_euclid(snap_px) + 1) * snap_px - 1),
    }
}

fn stable_query_snap_px(viewport_span_px: i64, cluster_bucket_px: i32) -> i64 {
    let bucket_px = i64::from(cluster_bucket_px.max(1));
    let multiplier = (viewport_span_px / (bucket_px * 2)).max(4);
    bucket_px * multiplier
}

fn normalized_axis(a: i32, b: i32) -> (i64, i64) {
    let a = i64::from(a);
    let b = i64::from(b);
    (a.min(b), a.max(b))
}

fn clamp_i64_to_i32(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

fn select_snapshot_indices_for_point_layer(
    snapshot: &EventsSnapshotState,
    query_bbox: &MapBboxPx,
    visible_scope: VisibleTileScope,
    from_ts_utc: Option<i64>,
    to_ts_utc: Option<i64>,
    expression: Option<&FishyMapSearchExpressionNode>,
    fish_catalog: &FishCatalog,
    search_expression: &SearchExpressionState,
) -> ViewSelection {
    let candidate_indices = {
        crate::perf_scope!("events.spatial_index_query");
        snapshot
            .spatial_index
            .query_bbox(query_bbox, &snapshot.events)
    };
    let empty_community = CommunityFishZoneSupportIndex::default();
    let mut evaluator = LayerSearchEvaluator::new(
        fish_catalog,
        &empty_community,
        snapshot,
        from_ts_utc,
        to_ts_utc,
        &search_expression.shared_fish_state.caught_ids,
        &search_expression.shared_fish_state.favourite_ids,
    );
    let mut filtered_indices = Vec::with_capacity(candidate_indices.len());
    for idx in &candidate_indices {
        let Some(event) = snapshot.events.get(*idx) else {
            continue;
        };
        if !event_matches_point_layer_filters(
            event,
            visible_scope,
            from_ts_utc,
            to_ts_utc,
            expression,
            &mut evaluator,
        ) {
            continue;
        }
        filtered_indices.push(*idx);
    }
    ViewSelection {
        candidate_count: candidate_indices.len(),
        filtered_indices,
    }
}

fn event_matches_point_layer_filters(
    event: &EventPointCompact,
    visible_scope: VisibleTileScope,
    from_ts_utc: Option<i64>,
    to_ts_utc: Option<i64>,
    expression: Option<&FishyMapSearchExpressionNode>,
    evaluator: &mut LayerSearchEvaluator<'_>,
) -> bool {
    if from_ts_utc.is_some_and(|from_ts_utc| event.ts_utc < from_ts_utc)
        || to_ts_utc.is_some_and(|to_ts_utc| event.ts_utc >= to_ts_utc)
    {
        return false;
    }
    if !visible_scope.contains(event.map_px_x, event.map_px_y) {
        return false;
    }
    expression.is_none_or(|expression| evaluator.event_matches_expression(event, expression))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use fishystuff_api::models::events::EventPointCompact;

    use super::*;
    use crate::bridge::contract::{
        FishyMapSearchExpressionNode, FishyMapSearchExpressionOperator, FishyMapSearchTerm,
    };
    use crate::map::events::SpatialIndex;
    use crate::plugins::api::{FishCatalog, FishEntry, SearchExpressionState};

    #[test]
    fn stable_point_query_bbox_is_stable_for_small_pans() {
        let first = stable_point_query_bbox(
            &MapBboxPx {
                min_x: 10_000,
                min_y: 20_000,
                max_x: 17_999,
                max_y: 27_999,
            },
            256,
        );
        let small_pan = stable_point_query_bbox(
            &MapBboxPx {
                min_x: 10_120,
                min_y: 20_140,
                max_x: 18_119,
                max_y: 28_139,
            },
            256,
        );

        assert_eq!(
            (
                small_pan.min_x,
                small_pan.min_y,
                small_pan.max_x,
                small_pan.max_y
            ),
            (first.min_x, first.min_y, first.max_x, first.max_y)
        );
    }

    #[test]
    fn stable_point_query_bbox_includes_whole_edge_cluster_buckets() {
        let bbox = stable_point_query_bbox(
            &MapBboxPx {
                min_x: 250,
                min_y: -40,
                max_x: 320,
                max_y: 40,
            },
            256,
        );

        assert!(bbox.min_x <= 0);
        assert!(bbox.max_x >= 511);
        assert_eq!(bbox.min_x.rem_euclid(256), 0);
        assert_eq!((bbox.max_x + 1).rem_euclid(256), 0);
    }

    #[test]
    fn precomputed_zone_filter_keeps_events_with_matching_zone_support() {
        let mut resolver = crate::map::events::EventZoneSetResolver::new();
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
                zone_rgbs: vec![0x222222],
                full_zone_rgbs: vec![0x222222],
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
                zone_rgbs: Vec::new(),
                full_zone_rgbs: Vec::new(),
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

        assert_eq!(filtered_indices, vec![0, 1]);
    }

    #[test]
    fn select_snapshot_indices_for_point_layer_keeps_mixed_fish_or_zone_matches() {
        let events = vec![
            EventPointCompact {
                event_id: 1,
                fish_id: 240,
                ts_utc: 100,
                map_px_x: 4,
                map_px_y: 4,
                length_milli: 1,
                world_x: None,
                world_z: None,
                zone_rgb_u32: Some(0x111111),
                zone_rgbs: vec![0x111111],
                full_zone_rgbs: vec![0x111111],
                source_kind: None,
                source_id: None,
            },
            EventPointCompact {
                event_id: 2,
                fish_id: 777,
                ts_utc: 100,
                map_px_x: 8,
                map_px_y: 8,
                length_milli: 1,
                world_x: None,
                world_z: None,
                zone_rgb_u32: Some(0xabcdef),
                zone_rgbs: vec![0xabcdef],
                full_zone_rgbs: vec![0xabcdef],
                source_kind: None,
                source_id: None,
            },
            EventPointCompact {
                event_id: 3,
                fish_id: 777,
                ts_utc: 100,
                map_px_x: 12,
                map_px_y: 12,
                length_milli: 1,
                world_x: None,
                world_z: None,
                zone_rgb_u32: Some(0x222222),
                zone_rgbs: vec![0x222222],
                full_zone_rgbs: vec![0x222222],
                source_kind: None,
                source_id: None,
            },
        ];
        let mut spatial_index = SpatialIndex::new(crate::map::events::SPATIAL_BUCKET_PX);
        spatial_index.rebuild(&events);
        let snapshot = EventsSnapshotState {
            loaded: true,
            events,
            spatial_index,
            ..EventsSnapshotState::default()
        };
        let mut fish_catalog = FishCatalog::default();
        fish_catalog.replace(vec![
            FishEntry {
                id: 240,
                item_id: 820240,
                encyclopedia_key: Some(240),
                encyclopedia_id: Some(9240),
                name: "Blobfish".to_string(),
                name_lower: "blobfish".to_string(),
                grade: Some("Rare".to_string()),
                is_prize: false,
            },
            FishEntry {
                id: 777,
                item_id: 820777,
                encyclopedia_key: Some(777),
                encyclopedia_id: Some(9777),
                name: "Other".to_string(),
                name_lower: "other".to_string(),
                grade: Some("General".to_string()),
                is_prize: false,
            },
        ]);
        let expression = FishyMapSearchExpressionNode::Group {
            operator: FishyMapSearchExpressionOperator::Or,
            children: vec![
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::Fish { fish_id: 240 },
                    negated: false,
                },
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::Zone { zone_rgb: 0xabcdef },
                    negated: false,
                },
            ],
            negated: false,
        };

        let selection = select_snapshot_indices_for_point_layer(
            &snapshot,
            &MapBboxPx {
                min_x: 0,
                min_y: 0,
                max_x: 16,
                max_y: 16,
            },
            VisibleTileScope::from_bbox(
                &MapBboxPx {
                    min_x: 0,
                    min_y: 0,
                    max_x: 16,
                    max_y: 16,
                },
                VISIBLE_TILE_SCOPE_PX,
            ),
            None,
            None,
            Some(&expression),
            &fish_catalog,
            &SearchExpressionState::default(),
        );

        assert_eq!(selection.candidate_count, 3);
        assert_eq!(selection.filtered_indices, vec![0, 1]);
    }
}

fn clear_render_points(points: &mut PointsState, render_state: &mut PointRenderState) {
    points.request_sig = None;
    if !points.points.is_empty() {
        points.points.clear();
        render_state.dirty = true;
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
