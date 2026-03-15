use bevy::prelude::*;
use fishystuff_api::models::events::{EventsQueryMode, MapBboxPx};

use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::events::{
    cluster_view_events, suggested_cluster_bucket_px, EventsSnapshotState, LocalEventQuery,
    VisibleTileScope, VISIBLE_TILE_SCOPE_PX,
};
use crate::plugins::api::{FishFilterState, MapDisplayState, PatchFilterState};
use crate::plugins::camera::Map2dCamera;

use super::super::render::view_bbox_map_px;
use super::state::PointsQuerySignature;
use super::{
    normalized_time_and_fish_filters, quantize_px, PointsState, RenderPoint, VIEWPORT_SIG_STEP_PX,
};

pub(in crate::plugins::points) fn refresh_points_from_local_snapshot(
    mut points: ResMut<PointsState>,
    patch_filter: Res<PatchFilterState>,
    fish_filter: Res<FishFilterState>,
    display_state: Res<MapDisplayState>,
    view_mode: Res<ViewModeState>,
    snapshot: Res<EventsSnapshotState>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &Transform), With<Map2dCamera>>,
) {
    crate::perf_scope!("events.snapshot_query_refresh");
    if !display_state.show_points || view_mode.mode != ViewMode::Map2D {
        points.status = "points: hidden".to_string();
        points.request_sig = None;
        return;
    }

    if snapshot.loading && !snapshot.loaded {
        points.status = "points: snapshot loading".to_string();
        clear_render_points(&mut points);
        return;
    }
    if snapshot.failed && !snapshot.loaded {
        points.status = format!(
            "points: snapshot failed ({})",
            snapshot
                .last_error
                .as_deref()
                .unwrap_or("unknown snapshot error")
        );
        clear_render_points(&mut points);
        return;
    }
    if !snapshot.loaded {
        points.status = "points: snapshot pending".to_string();
        clear_render_points(&mut points);
        return;
    }

    let Some(viewport_bbox) = view_bbox_map_px(&windows, &camera_q) else {
        points.status = "points: missing viewport".to_string();
        clear_render_points(&mut points);
        return;
    };

    let Some((from_ts_utc, to_ts_utc, mut fish_ids)) =
        normalized_time_and_fish_filters(&patch_filter, &fish_filter)
    else {
        points.status = "points: missing range".to_string();
        clear_render_points(&mut points);
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
    let signature = PointsQuerySignature {
        revision: snapshot.revision.clone(),
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

    if points.request_sig.as_ref() == Some(&signature) {
        points.status = points_status_line(&points, &snapshot);
        return;
    }

    let local_query = LocalEventQuery {
        bbox: &viewport_bbox,
        from_ts_utc,
        to_ts_utc,
        fish_ids: fish_ids.as_slice(),
        tile_scope: Some(VisibleTileScope::from_bbox(
            &tile_scope,
            VISIBLE_TILE_SCOPE_PX,
        )),
    };
    let selection = snapshot.select_for_view(&local_query);
    let clustered = {
        crate::perf_scope!("events.clustering");
        cluster_view_events(
            &snapshot.events,
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

    points.request_sig = Some(signature);
    points.points = rendered_points;
    points.mode = Some(clustered.mode);
    points.bucket_px = clustered.cluster_bucket_px;
    points.sample_step = 1;
    points.total = selection.filtered_indices.len();
    points.represented_sample_count = clustered.represented_event_count;
    points.candidate_count = selection.candidate_count;
    points.rendered_point_count = clustered.rendered_point_count;
    points.rendered_cluster_count = clustered.rendered_cluster_count;
    points.spatial_bucket_px = snapshot.spatial_index.bucket_px;
    crate::perf_gauge!("events.cluster_count", points.rendered_cluster_count);
    crate::perf_gauge!("events.raw_point_count", points.rendered_point_count);
    points.status = points_status_line(&points, &snapshot);
    points.dirty = true;
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
