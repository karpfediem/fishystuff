use super::super::snapshot::effective_filters;
use super::super::*;
use serde_json::json;

pub(in crate::bridge::host) fn emit_diagnostic_event(
    bridge: Res<BrowserBridgeState>,
    bootstrap: Res<ApiBootstrapState>,
    patch_filter: Res<PatchFilterState>,
    fish_filter: Res<FishFilterState>,
    semantic_filter: Res<SemanticFieldFilterState>,
    points: Res<PointsState>,
    point_icons: Res<PointIconCache>,
    selection: Res<SelectionState>,
    view_mode: Res<ViewModeState>,
    terrain_diag: Res<TerrainDiagnostics>,
    layer_registry: Res<LayerRegistry>,
    layer_runtime: Res<LayerRuntime>,
) {
    crate::perf_scope!("bridge.emit.diagnostic");
    if !bridge.input.ui.diagnostics_open {
        return;
    }
    if !bridge.is_changed()
        && !bootstrap.is_changed()
        && !patch_filter.is_changed()
        && !fish_filter.is_changed()
        && !semantic_filter.is_changed()
        && !points.is_changed()
        && !point_icons.is_changed()
        && !selection.is_changed()
        && !view_mode.is_changed()
        && !terrain_diag.is_changed()
        && !layer_registry.is_changed()
        && !layer_runtime.is_changed()
    {
        return;
    }

    let filters = effective_filters(
        &bridge.input,
        &patch_filter,
        &fish_filter,
        &semantic_filter,
        &layer_registry,
        &layer_runtime,
    );
    let payload = json!({
        "ready": bootstrap.meta.is_some() && !layer_registry.ordered().is_empty(),
        "viewMode": match view_mode.mode {
            ViewMode::Map2D => "2d",
            ViewMode::Terrain3D => "3d",
        },
        "metaStatus": bootstrap.meta_status,
        "layersStatus": bootstrap.layers_status,
        "zonesStatus": bootstrap.zones_status,
        "pointsStatus": points.status,
        "pointIcons": {
            "requested": point_icons.requested_count(),
            "loading": point_icons.loading_count(),
            "loaded": point_icons.loaded_count(),
            "failed": point_icons.failed_count(),
            "missingCatalog": point_icons.missing_catalog_count(),
            "missingCatalogSample": point_icons.missing_catalog_sample(),
            "visible": point_icons.visible_icon_count,
            "visibleSample": point_icons.visible_sample(),
            "requestedSample": point_icons.requested_sample(),
            "failedSample": point_icons.failed_sample(),
        },
        "zoneStatsStatus": selection.zone_stats_status,
        "selectedPatch": patch_filter.selected_patch,
        "patchId": filters.patch_id,
        "fromPatchId": filters.from_patch_id,
        "toPatchId": filters.to_patch_id,
        "visibleLayers": layer_registry
            .ordered()
            .iter()
            .filter(|layer| layer_runtime.visible(layer.id))
            .map(|layer| layer.key.clone())
            .collect::<Vec<_>>(),
        "terrain": {
            "ready": terrain_diag.terrain_ready,
            "revision": terrain_diag.terrain_revision,
            "chunksRequested": terrain_diag.chunks_requested,
            "chunksReady": terrain_diag.chunks_ready,
            "cacheHits": terrain_diag.cache_hits,
            "cacheMisses": terrain_diag.cache_misses,
        }
    });
    let event = FishyMapOutputEvent::Diagnostic {
        version: 1,
        payload: payload.clone(),
    };

    let serialized = match serde_json::to_string(&event) {
        Ok(value) => value,
        Err(_) => return,
    };
    LAST_DIAGNOSTIC_PAYLOAD.with(|last_payload| {
        let mut last_payload = last_payload.borrow_mut();
        if last_payload.as_deref() == Some(serialized.as_str()) {
            return;
        }
        crate::perf_counter_add!("bridge.emit.diagnostic.count", 1);
        CURRENT_SNAPSHOT.with(|snapshot| {
            snapshot.borrow_mut().last_diagnostic = Some(payload);
        });
        super::super::emit_event(&event);
        *last_payload = Some(serialized);
    });
}
