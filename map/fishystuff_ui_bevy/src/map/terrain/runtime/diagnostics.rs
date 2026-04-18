use super::*;

pub(super) fn sync_terrain_diagnostics(
    mode: Res<ViewModeState>,
    config: Res<Terrain3dConfig>,
    view: Res<Terrain3dViewState>,
    runtime: Res<TerrainRuntime>,
    mut diagnostics: ResMut<TerrainDiagnostics>,
) {
    let (requested, building, ready) = runtime.chunk_counts();
    let manifest_bbox = runtime
        .manifest
        .as_ref()
        .map(|loaded| (loaded.manifest.bbox_y_min, loaded.manifest.bbox_y_max));
    diagnostics.enabled = mode.mode == ViewMode::Terrain3D;
    diagnostics.terrain_ready = runtime.manifest_ready();
    diagnostics.terrain_revision = runtime
        .manifest
        .as_ref()
        .map(|loaded| loaded.manifest.revision.clone());
    diagnostics.manifest_ready = runtime.manifest.is_some();
    diagnostics.manifest_failed = runtime.manifest_failed;
    diagnostics.chunk_map_px_runtime = runtime
        .manifest
        .as_ref()
        .map(|loaded| loaded.manifest.chunk_map_px)
        .unwrap_or(config.chunk_map_px);
    diagnostics.grid_size_runtime = runtime
        .manifest
        .as_ref()
        .map(|loaded| loaded.manifest.grid_size)
        .unwrap_or(config.verts_per_chunk_edge as u16);
    diagnostics.max_level_runtime = runtime
        .manifest
        .as_ref()
        .map(|loaded| loaded.manifest.max_level)
        .unwrap_or(0);
    diagnostics.bbox_y_min = manifest_bbox.map(|bbox| bbox.0).unwrap_or(0.0);
    diagnostics.bbox_y_max = manifest_bbox.map(|bbox| bbox.1).unwrap_or(0.0);
    diagnostics.show_drape = config.show_drape;
    diagnostics.chunks_requested = requested;
    diagnostics.chunks_building = building;
    diagnostics.chunks_ready = ready;
    diagnostics.drape_patch_count = runtime.chunk_drape_entries.len() + runtime.drape_entries.len();
    diagnostics.drape_missing_textures = runtime.drape_missing_textures;
    diagnostics.drape_min_z = runtime.drape_min_z;
    diagnostics.drape_max_z = runtime.drape_max_z;
    diagnostics.avg_build_ms = runtime.avg_build_ms;
    diagnostics.camera_pivot = view.pivot_world;
    diagnostics.camera_yaw = view.yaw;
    diagnostics.camera_pitch = view.pitch;
    diagnostics.camera_distance = view.distance;
    diagnostics.visible_chunks_by_level = runtime.visible_chunks_by_level.clone();
    diagnostics.resident_chunks_by_level = runtime.resident_chunks_by_level.clone();
    diagnostics.cache_hits = runtime.cache_hits;
    diagnostics.cache_misses = runtime.cache_misses;
    diagnostics.cache_evictions = runtime.cache_evictions;
    diagnostics.fallback_chunks = runtime.fallback_chunks;
    crate::perf_last!(
        "terrain.runtime.ready",
        if diagnostics.terrain_ready { 1.0 } else { 0.0 }
    );
    crate::perf_last!("terrain.runtime.chunks_requested", requested as f64);
    crate::perf_last!("terrain.runtime.chunks_ready", ready as f64);
    crate::perf_last!("terrain.runtime.cache_hits", runtime.cache_hits as f64);
    crate::perf_last!("terrain.runtime.cache_misses", runtime.cache_misses as f64);
    crate::perf_last!("terrain.runtime.avg_build_ms", runtime.avg_build_ms as f64);
}
