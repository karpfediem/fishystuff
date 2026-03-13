use super::super::super::*;

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
        .map(|layer| FishyMapLayerSummary {
            layer_id: layer.key.clone(),
            name: layer.name.clone(),
            visible: layer_runtime.visible(layer.id),
            opacity: layer_runtime.opacity(layer.id),
            opacity_default: layer.opacity_default,
            display_order: layer_runtime
                .get(layer.id)
                .map(|state| state.display_order)
                .unwrap_or(layer.display_order),
            kind: match layer.kind {
                LayerKind::TiledRaster => "tiled-raster".to_string(),
                LayerKind::VectorGeoJson => "vector-geojson".to_string(),
            },
        })
        .collect()
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
