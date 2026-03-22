use super::super::super::*;
use crate::map::layers::LayerManifestStatus;

pub(in crate::bridge::host) fn current_layer_order<'a>(
    layer_registry: &'a LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Vec<&'a crate::map::layers::LayerSpec> {
    let mut ordered = layer_registry.ordered().iter().collect::<Vec<_>>();
    ordered.sort_by(|lhs, rhs| {
        layer_runtime
            .get(rhs.id)
            .map(|state| state.display_order)
            .unwrap_or(rhs.display_order)
            .cmp(
                &layer_runtime
                    .get(lhs.id)
                    .map(|state| state.display_order)
                    .unwrap_or(lhs.display_order),
            )
            .then_with(|| rhs.display_order.cmp(&lhs.display_order))
            .then_with(|| lhs.key.cmp(&rhs.key))
    });
    ordered
}

pub(in crate::bridge::host::snapshot) fn current_layer_summaries(
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Vec<FishyMapLayerSummary> {
    current_layer_order(layer_registry, layer_runtime)
        .into_iter()
        .map(|layer| {
            let runtime_state = layer_runtime.get(layer.id);
            FishyMapLayerSummary {
                layer_id: layer.key.clone(),
                name: layer.name.clone(),
                visible: runtime_state
                    .map(|state| state.visible)
                    .unwrap_or(layer.visible_default),
                opacity: runtime_state
                    .map(|state| state.opacity)
                    .unwrap_or(layer.opacity_default),
                opacity_default: layer.opacity_default,
                display_order: runtime_state
                    .map(|state| state.display_order)
                    .unwrap_or(layer.display_order),
                kind: match layer.kind {
                    LayerKind::TiledRaster => "tiled-raster".to_string(),
                    LayerKind::VectorGeoJson => "vector-geojson".to_string(),
                },
                visible_tile_count: runtime_state
                    .map(|state| state.visible_tile_count)
                    .unwrap_or_default(),
                resident_tile_count: runtime_state
                    .map(|state| state.resident_tile_count)
                    .unwrap_or_default(),
                pending_count: runtime_state
                    .map(|state| state.pending_count)
                    .unwrap_or_default(),
                inflight_count: runtime_state
                    .map(|state| state.inflight_count)
                    .unwrap_or_default(),
                manifest_status: runtime_state
                    .map(|state| manifest_status_label(state.manifest_status).to_string())
                    .unwrap_or_else(|| "missing".to_string()),
                vector_status: runtime_state
                    .map(|state| vector_status_label(state.vector_status).to_string())
                    .unwrap_or_else(|| "inactive".to_string()),
                vector_progress: runtime_state
                    .map(|state| state.vector_progress)
                    .unwrap_or_default(),
                vector_feature_count: runtime_state
                    .map(|state| state.vector_feature_count)
                    .unwrap_or_default(),
                vector_vertex_count: runtime_state
                    .map(|state| state.vector_vertex_count)
                    .unwrap_or_default(),
                vector_triangle_count: runtime_state
                    .map(|state| state.vector_triangle_count)
                    .unwrap_or_default(),
                vector_mesh_count: runtime_state
                    .map(|state| state.vector_mesh_count)
                    .unwrap_or_default(),
                vector_chunked_bucket_count: runtime_state
                    .map(|state| state.vector_chunked_bucket_count)
                    .unwrap_or_default(),
                vector_build_ms: runtime_state
                    .map(|state| state.vector_build_ms)
                    .unwrap_or_default(),
                vector_last_frame_build_ms: runtime_state
                    .map(|state| state.vector_last_frame_build_ms)
                    .unwrap_or_default(),
                vector_cache_entries: runtime_state
                    .map(|state| state.vector_cache_entries)
                    .unwrap_or_default(),
            }
        })
        .collect()
}

fn manifest_status_label(status: LayerManifestStatus) -> &'static str {
    match status {
        LayerManifestStatus::Missing => "missing",
        LayerManifestStatus::Loading => "loading",
        LayerManifestStatus::Ready => "ready",
        LayerManifestStatus::Failed => "failed",
    }
}

fn vector_status_label(status: crate::map::layers::LayerVectorStatus) -> &'static str {
    match status {
        crate::map::layers::LayerVectorStatus::Inactive => "inactive",
        crate::map::layers::LayerVectorStatus::NotRequested => "not-requested",
        crate::map::layers::LayerVectorStatus::Fetching => "fetching",
        crate::map::layers::LayerVectorStatus::Parsing => "parsing",
        crate::map::layers::LayerVectorStatus::Building => "building",
        crate::map::layers::LayerVectorStatus::Ready => "ready",
        crate::map::layers::LayerVectorStatus::Failed => "failed",
    }
}

pub(in crate::bridge::host::snapshot::filters) fn current_layer_opacity_overrides(
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Option<BTreeMap<String, f32>> {
    let mut overrides = BTreeMap::new();
    for layer in current_layer_order(layer_registry, layer_runtime) {
        if layer.key == "minimap" {
            continue;
        }
        let opacity = layer_runtime.opacity(layer.id).clamp(0.0, 1.0);
        if (opacity - layer.opacity_default).abs() <= f32::EPSILON {
            continue;
        }
        overrides.insert(layer.key.clone(), opacity);
    }
    (!overrides.is_empty()).then_some(overrides)
}

pub(in crate::bridge::host::snapshot::filters) fn current_layer_clip_mask_overrides(
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Option<BTreeMap<String, String>> {
    let mut overrides = BTreeMap::new();
    for layer in current_layer_order(layer_registry, layer_runtime) {
        if layer.key == "minimap" {
            continue;
        }
        let Some(mask_layer_id) = layer_runtime.clip_mask_layer(layer.id) else {
            continue;
        };
        let Some(mask_layer) = layer_registry.get(mask_layer_id) else {
            continue;
        };
        if mask_layer.id == layer.id {
            continue;
        }
        overrides.insert(layer.key.clone(), mask_layer.key.clone());
    }
    (!overrides.is_empty()).then_some(overrides)
}
