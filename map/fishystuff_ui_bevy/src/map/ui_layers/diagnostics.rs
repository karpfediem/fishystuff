use super::*;
use bevy::ecs::system::SystemParam;

pub(super) fn sync_layer_debug(
    state: LayerDiagnosticsState<'_, '_>,
    mut debug_q: Query<(&mut Text, &mut Visibility), With<LayerDebugText>>,
) {
    if !state.stats.is_changed()
        && !state.cache.is_changed()
        && !state.layer_registry.is_changed()
        && !state.layer_settings.is_changed()
        && !state.view_mode.is_changed()
        && !state.terrain_diag.is_changed()
        && !state.snapshot.is_changed()
        && !state.points.is_changed()
        && !state.point_icons.is_changed()
        && !state.controls.is_changed()
        && !state.debug.is_changed()
    {
        return;
    }
    for (mut text, mut visibility) in &mut debug_q {
        if !state.debug.enabled {
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility = Visibility::Visible;
        let view = match (state.stats.view_min, state.stats.view_max) {
            (Some((x0, y0)), Some((x1, y1))) => {
                format!("[{x0:.0},{y0:.0}]..[{x1:.0},{y1:.0}]")
            }
            _ => "-".to_string(),
        };
        let cursor_world = state
            .stats
            .cursor_world
            .map(|(x, z)| format!("{x:.0},{z:.0}"))
            .unwrap_or_else(|| "-".to_string());
        let cursor = state
            .stats
            .cursor_map
            .map(|(x, y)| format!("{x:.0},{y:.0}"))
            .unwrap_or_else(|| "-".to_string());

        let mut layer_lines = String::new();
        for layer in state.layer_registry.ordered() {
            let Some(layer_state) = state.layer_settings.get(layer.id) else {
                continue;
            };
            if !layer_state.visible {
                continue;
            }
            if layer.kind == LayerKind::VectorGeoJson {
                layer_lines.push_str(&format!(
                    "\n{}: vec={} p{:>3.0}% bytes={} f{}/{} poly{} mp{} h{} v{} t{} b{:.1}ms l{:.2}ms cache={} h{} m{} e{} rev={}",
                    layer.name.as_str(),
                    vector_status_label(layer_state.vector_status),
                    layer_state.vector_progress * 100.0,
                    layer_state.vector_fetched_bytes,
                    layer_state.vector_features_processed,
                    layer_state.vector_feature_count,
                    layer_state.vector_polygon_count,
                    layer_state.vector_multipolygon_count,
                    layer_state.vector_hole_ring_count,
                    layer_state.vector_vertex_count,
                    layer_state.vector_triangle_count,
                    layer_state.vector_build_ms,
                    layer_state.vector_last_frame_build_ms,
                    if layer_state.vector_cache_last_hit { "hit" } else { "miss" },
                    layer_state.vector_cache_hits,
                    layer_state.vector_cache_misses,
                    layer_state.vector_cache_entries,
                    layer
                        .vector_source
                        .as_ref()
                        .map(vector_revision_label)
                        .unwrap_or_else(|| "-".to_string()),
                ));
            } else {
                let base_lod = state
                    .layer_settings
                    .get(layer.id)
                    .and_then(|layer_state| layer_state.current_base_lod)
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let detail_lod = state
                    .layer_settings
                    .get(layer.id)
                    .and_then(|layer_state| layer_state.current_detail_lod)
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let resident_by_level =
                    format_level_counts(state.stats.resident_by_level.get(&layer.id));
                let protected_by_level =
                    format_level_counts(state.stats.protected_by_level.get(&layer.id));
                let warm_by_level = format_level_counts(state.stats.warm_by_level.get(&layer.id));
                let fallback_by_level =
                    format_level_counts(state.stats.fallback_visible_by_level.get(&layer.id));
                let hits_by_level =
                    format_level_counts(state.stats.cache_hits_by_level.get(&layer.id));
                let misses_by_level =
                    format_level_counts(state.stats.cache_misses_by_level.get(&layer.id));
                let evictions_by_level =
                    format_level_counts(state.stats.cache_evictions_by_level.get(&layer.id));
                let blank_visible = state
                    .stats
                    .blank_visible_by_layer
                    .get(&layer.id)
                    .copied()
                    .unwrap_or(0);
                layer_lines.push_str(&format!(
                    "\n{}: b{} d{} v{} r{} q{} i{} blank{} res{{{}}} prot{{{}}} warm{{{}}} fall{{{}}} hit{{{}}} miss{{{}}} evict{{{}}} m={:?}",
                    layer.name.as_str(),
                    base_lod,
                    detail_lod,
                    layer_state.visible_tile_count,
                    layer_state.resident_tile_count,
                    layer_state.pending_count,
                    layer_state.inflight_count,
                    blank_visible,
                    resident_by_level,
                    protected_by_level,
                    warm_by_level,
                    fallback_by_level,
                    hits_by_level,
                    misses_by_level,
                    evictions_by_level,
                    layer_state.manifest_status,
                ));
            }
        }

        let terrain_visible_levels = if state.terrain_diag.visible_chunks_by_level.is_empty() {
            "-".to_string()
        } else {
            state
                .terrain_diag
                .visible_chunks_by_level
                .iter()
                .map(|(level, count)| format!("l{}:{}", level, count))
                .collect::<Vec<_>>()
                .join(",")
        };
        let terrain_resident_levels = if state.terrain_diag.resident_chunks_by_level.is_empty() {
            "-".to_string()
        } else {
            state
                .terrain_diag
                .resident_chunks_by_level
                .iter()
                .map(|(level, count)| format!("l{}:{}", level, count))
                .collect::<Vec<_>>()
                .join(",")
        };

        let next = format!(
            "{}\nmode={:?} terrain(ready={} rev={:?} manifest={} fail={} chunk={} grid={} max_l={} bbox={:.1}..{:.1} drape_show={} chunks={}/{}/{} vis{{{}}} res{{{}}} fallback={} cache(h/m/e)={}/{}/{} drape={} build={:.2}ms cam={:.1}/{:.1}/{:.1} d={:.0})\nevents_snapshot: rev={} events_loaded={} idx_bucket={} candidates={} rendered_points={} rendered_clusters={} snapshot_source={} meta_req={} snapshot_req={}\npoint_icons: requested={} loading={} loaded={} failed={} missing_catalog={} visible={} visible_ids={:?} requested_sample={:?} failed_sample={:?}\nreq/cache: req={} cov(q/s)={}/{} det(q/s)={}/{} sup={} hits={} evict={} cache={} evict_mode={}\ncoverage: fallback_visible={} blank_visible={} motion={} pan={:.3} zoom_out={:.3}\nview(world)={} cursor(world)={} cursor(map)={}\nstream: visible={} inflight={} queued={}{}",
            state.points.status,
            state.view_mode.mode,
            if state.terrain_diag.terrain_ready {
                "ready"
            } else {
                "pending"
            },
            state.terrain_diag.terrain_revision,
            state.terrain_diag.manifest_ready,
            state.terrain_diag.manifest_failed,
            state.terrain_diag.chunk_map_px_runtime,
            state.terrain_diag.grid_size_runtime,
            state.terrain_diag.max_level_runtime,
            state.terrain_diag.bbox_y_min,
            state.terrain_diag.bbox_y_max,
            if state.terrain_diag.show_drape {
                "1"
            } else {
                "0"
            },
            state.terrain_diag.chunks_requested,
            state.terrain_diag.chunks_building,
            state.terrain_diag.chunks_ready,
            terrain_visible_levels,
            terrain_resident_levels,
            state.terrain_diag.fallback_chunks,
            state.terrain_diag.cache_hits,
            state.terrain_diag.cache_misses,
            state.terrain_diag.cache_evictions,
            state.terrain_diag.drape_patch_count,
            state.terrain_diag.avg_build_ms,
            state.terrain_diag.camera_pivot.x,
            state.terrain_diag.camera_pivot.y,
            state.terrain_diag.camera_pivot.z,
            state.terrain_diag.camera_distance,
            state.snapshot.revision.as_deref().unwrap_or("-"),
            state.snapshot.event_count,
            state.points.spatial_bucket_px,
            state.points.candidate_count,
            state.points.rendered_point_count,
            state.points.rendered_cluster_count,
            state.snapshot.last_load_kind.label(),
            state.snapshot.meta_requests_started,
            state.snapshot.snapshot_requests_started,
            state.point_icons.requested_count(),
            state.point_icons.loading_count(),
            state.point_icons.loaded_count(),
            state.point_icons.failed_count(),
            state.point_icons.missing_catalog_count(),
            state.point_icons.visible_icon_count,
            state.point_icons.visible_sample(),
            state.point_icons.requested_sample(),
            state.point_icons.failed_sample(),
            state.stats.requested_tiles,
            state.stats.coverage_requests_queued,
            state.stats.coverage_requests_started,
            state.stats.detail_requests_queued,
            state.stats.detail_requests_started,
            state.stats.requests_suppressed_motion,
            state.stats.cache_hits,
            state.stats.cache_evictions,
            state.cache.len(),
            if state.controls.disable_eviction {
                "off"
            } else {
                "on"
            },
            state.stats.fallback_visible_tiles,
            state.stats.blank_visible_tiles,
            if state.stats.camera_unstable {
                "unstable"
            } else {
                "stable"
            },
            state.stats.camera_pan_fraction,
            state.stats.camera_zoom_out_ratio,
            view,
            cursor_world,
            cursor,
            state.stats.visible_tiles,
            state.stats.inflight,
            state.stats.queue_len,
            layer_lines,
        );
        if text.0 != next {
            text.0 = next;
        }
    }
}

#[derive(SystemParam)]
pub(super) struct LayerDiagnosticsState<'w, 's> {
    stats: Res<'w, TileStats>,
    cache: Res<'w, RasterTileCache>,
    layer_registry: Res<'w, LayerRegistry>,
    layer_settings: Res<'w, LayerSettings>,
    view_mode: Res<'w, ViewModeState>,
    terrain_diag: Res<'w, crate::map::terrain::runtime::TerrainDiagnostics>,
    snapshot: Res<'w, EventsSnapshotState>,
    points: Res<'w, PointsState>,
    point_icons: Res<'w, PointIconCache>,
    controls: Res<'w, TileDebugControls>,
    debug: Res<'w, LayerDebugSettings>,
    _marker: std::marker::PhantomData<&'s ()>,
}

fn format_level_counts(levels: Option<&BTreeMap<i32, u32>>) -> String {
    let Some(levels) = levels else {
        return "-".to_string();
    };
    if levels.is_empty() {
        return "-".to_string();
    }
    let mut parts = Vec::with_capacity(levels.len());
    for (level, count) in levels {
        parts.push(format!("z{}:{}", level, count));
    }
    parts.join(",")
}

fn vector_revision_label(source: &VectorSourceSpec) -> String {
    let revision = source.revision.trim();
    if revision.is_empty() {
        format!("url:{}", source.url)
    } else {
        revision.to_string()
    }
}

fn vector_status_label(status: crate::map::layers::LayerVectorStatus) -> &'static str {
    use crate::map::layers::LayerVectorStatus;
    match status {
        LayerVectorStatus::Inactive => "Inactive",
        LayerVectorStatus::NotRequested => "Not requested",
        LayerVectorStatus::Fetching => "Fetching",
        LayerVectorStatus::Parsing => "Parsing",
        LayerVectorStatus::Building => "Building",
        LayerVectorStatus::Ready => "Ready",
        LayerVectorStatus::Failed => "Failed",
    }
}
