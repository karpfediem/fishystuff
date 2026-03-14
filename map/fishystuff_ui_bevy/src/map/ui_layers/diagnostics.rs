use super::*;

pub(super) fn sync_layer_debug(
    stats: Res<TileStats>,
    cache: Res<RasterTileCache>,
    layer_registry: Res<LayerRegistry>,
    layer_settings: Res<LayerSettings>,
    view_mode: Res<ViewModeState>,
    terrain_diag: Res<crate::map::terrain::runtime::TerrainDiagnostics>,
    snapshot: Res<EventsSnapshotState>,
    points: Res<PointsState>,
    point_icons: Res<PointIconCache>,
    controls: Res<TileDebugControls>,
    debug: Res<LayerDebugSettings>,
    mut debug_q: Query<(&mut Text, &mut Visibility), With<LayerDebugText>>,
) {
    if !stats.is_changed()
        && !cache.is_changed()
        && !layer_registry.is_changed()
        && !layer_settings.is_changed()
        && !view_mode.is_changed()
        && !terrain_diag.is_changed()
        && !snapshot.is_changed()
        && !points.is_changed()
        && !point_icons.is_changed()
        && !controls.is_changed()
        && !debug.is_changed()
    {
        return;
    }
    for (mut text, mut visibility) in &mut debug_q {
        if !debug.enabled {
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility = Visibility::Visible;
        let view = match (stats.view_min, stats.view_max) {
            (Some((x0, y0)), Some((x1, y1))) => {
                format!("[{x0:.0},{y0:.0}]..[{x1:.0},{y1:.0}]")
            }
            _ => "-".to_string(),
        };
        let cursor_world = stats
            .cursor_world
            .map(|(x, z)| format!("{x:.0},{z:.0}"))
            .unwrap_or_else(|| "-".to_string());
        let cursor = stats
            .cursor_map
            .map(|(x, y)| format!("{x:.0},{y:.0}"))
            .unwrap_or_else(|| "-".to_string());

        let mut layer_lines = String::new();
        for layer in layer_registry.ordered() {
            let Some(state) = layer_settings.get(layer.id) else {
                continue;
            };
            if !state.visible {
                continue;
            }
            if layer.kind == LayerKind::VectorGeoJson {
                layer_lines.push_str(&format!(
                    "\n{}: vec={} p{:>3.0}% bytes={} f{}/{} poly{} mp{} h{} v{} t{} b{:.1}ms l{:.2}ms cache={} h{} m{} e{} rev={}",
                    layer.name.as_str(),
                    vector_status_label(state.vector_status),
                    state.vector_progress * 100.0,
                    state.vector_fetched_bytes,
                    state.vector_features_processed,
                    state.vector_feature_count,
                    state.vector_polygon_count,
                    state.vector_multipolygon_count,
                    state.vector_hole_ring_count,
                    state.vector_vertex_count,
                    state.vector_triangle_count,
                    state.vector_build_ms,
                    state.vector_last_frame_build_ms,
                    if state.vector_cache_last_hit { "hit" } else { "miss" },
                    state.vector_cache_hits,
                    state.vector_cache_misses,
                    state.vector_cache_entries,
                    layer
                        .vector_source
                        .as_ref()
                        .map(vector_revision_label)
                        .unwrap_or_else(|| "-".to_string()),
                ));
            } else {
                let base_lod = state
                    .current_base_lod
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let detail_lod = state
                    .current_detail_lod
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let resident_by_level = format_level_counts(stats.resident_by_level.get(&layer.id));
                let protected_by_level =
                    format_level_counts(stats.protected_by_level.get(&layer.id));
                let warm_by_level = format_level_counts(stats.warm_by_level.get(&layer.id));
                let fallback_by_level =
                    format_level_counts(stats.fallback_visible_by_level.get(&layer.id));
                let hits_by_level = format_level_counts(stats.cache_hits_by_level.get(&layer.id));
                let misses_by_level =
                    format_level_counts(stats.cache_misses_by_level.get(&layer.id));
                let evictions_by_level =
                    format_level_counts(stats.cache_evictions_by_level.get(&layer.id));
                let blank_visible = stats
                    .blank_visible_by_layer
                    .get(&layer.id)
                    .copied()
                    .unwrap_or(0);
                layer_lines.push_str(&format!(
                    "\n{}: b{} d{} v{} r{} q{} i{} blank{} res{{{}}} prot{{{}}} warm{{{}}} fall{{{}}} hit{{{}}} miss{{{}}} evict{{{}}} m={:?}",
                    layer.name.as_str(),
                    base_lod,
                    detail_lod,
                    state.visible_tile_count,
                    state.resident_tile_count,
                    state.pending_count,
                    state.inflight_count,
                    blank_visible,
                    resident_by_level,
                    protected_by_level,
                    warm_by_level,
                    fallback_by_level,
                    hits_by_level,
                    misses_by_level,
                    evictions_by_level,
                    state.manifest_status,
                ));
            }
        }

        let terrain_visible_levels = if terrain_diag.visible_chunks_by_level.is_empty() {
            "-".to_string()
        } else {
            terrain_diag
                .visible_chunks_by_level
                .iter()
                .map(|(level, count)| format!("l{}:{}", level, count))
                .collect::<Vec<_>>()
                .join(",")
        };
        let terrain_resident_levels = if terrain_diag.resident_chunks_by_level.is_empty() {
            "-".to_string()
        } else {
            terrain_diag
                .resident_chunks_by_level
                .iter()
                .map(|(level, count)| format!("l{}:{}", level, count))
                .collect::<Vec<_>>()
                .join(",")
        };

        let next = format!(
            "{}\nmode={:?} terrain(ready={} rev={:?} manifest={} fail={} chunk={} grid={} max_l={} bbox={:.1}..{:.1} drape_show={} chunks={}/{}/{} vis{{{}}} res{{{}}} fallback={} cache(h/m/e)={}/{}/{} drape={} build={:.2}ms cam={:.1}/{:.1}/{:.1} d={:.0})\nevents_snapshot: rev={} events_loaded={} idx_bucket={} candidates={} rendered_points={} rendered_clusters={} snapshot_source={} meta_req={} snapshot_req={}\npoint_icons: requested={} loading={} loaded={} failed={} missing_catalog={} visible={} visible_ids={:?} requested_sample={:?} failed_sample={:?}\nreq/cache: req={} cov(q/s)={}/{} det(q/s)={}/{} sup={} hits={} evict={} cache={} evict_mode={}\ncoverage: fallback_visible={} blank_visible={} motion={} pan={:.3} zoom_out={:.3}\nview(world)={} cursor(world)={} cursor(map)={}\nstream: visible={} inflight={} queued={}{}",
            points.status,
            view_mode.mode,
            if terrain_diag.terrain_ready {
                "ready"
            } else {
                "pending"
            },
            terrain_diag.terrain_revision,
            terrain_diag.manifest_ready,
            terrain_diag.manifest_failed,
            terrain_diag.chunk_map_px_runtime,
            terrain_diag.grid_size_runtime,
            terrain_diag.max_level_runtime,
            terrain_diag.bbox_y_min,
            terrain_diag.bbox_y_max,
            if terrain_diag.show_drape { "1" } else { "0" },
            terrain_diag.chunks_requested,
            terrain_diag.chunks_building,
            terrain_diag.chunks_ready,
            terrain_visible_levels,
            terrain_resident_levels,
            terrain_diag.fallback_chunks,
            terrain_diag.cache_hits,
            terrain_diag.cache_misses,
            terrain_diag.cache_evictions,
            terrain_diag.drape_patch_count,
            terrain_diag.avg_build_ms,
            terrain_diag.camera_pivot.x,
            terrain_diag.camera_pivot.y,
            terrain_diag.camera_pivot.z,
            terrain_diag.camera_distance,
            snapshot.revision.as_deref().unwrap_or("-"),
            snapshot.event_count,
            points.spatial_bucket_px,
            points.candidate_count,
            points.rendered_point_count,
            points.rendered_cluster_count,
            snapshot.last_load_kind.label(),
            snapshot.meta_requests_started,
            snapshot.snapshot_requests_started,
            point_icons.requested_count(),
            point_icons.loading_count(),
            point_icons.loaded_count(),
            point_icons.failed_count(),
            point_icons.missing_catalog_count(),
            point_icons.visible_icon_count,
            point_icons.visible_sample(),
            point_icons.requested_sample(),
            point_icons.failed_sample(),
            stats.requested_tiles,
            stats.coverage_requests_queued,
            stats.coverage_requests_started,
            stats.detail_requests_queued,
            stats.detail_requests_started,
            stats.requests_suppressed_motion,
            stats.cache_hits,
            stats.cache_evictions,
            cache.len(),
            if controls.disable_eviction {
                "off"
            } else {
                "on"
            },
            stats.fallback_visible_tiles,
            stats.blank_visible_tiles,
            if stats.camera_unstable { "unstable" } else { "stable" },
            stats.camera_pan_fraction,
            stats.camera_zoom_out_ratio,
            view,
            cursor_world,
            cursor,
            stats.visible_tiles,
            stats.inflight,
            stats.queue_len,
            layer_lines,
        );
        if text.0 != next {
            text.0 = next;
        }
    }
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
