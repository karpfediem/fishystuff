use super::super::snapshot::current_layer_order;
use super::super::*;

pub(in crate::bridge::host) fn apply_layer_order_override(
    layer_registry: &LayerRegistry,
    layer_runtime: &mut LayerRuntime,
    ordered_layer_ids: &[String],
) {
    let mut seen = HashSet::new();
    let mut top_first_ids = Vec::with_capacity(layer_registry.ordered().len());
    let current_order = current_layer_order(layer_registry, layer_runtime);

    for layer_id in ordered_layer_ids {
        let trimmed = layer_id.trim();
        if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
            continue;
        }
        if layer_registry.id_by_key(trimmed).is_some() {
            top_first_ids.push(trimmed.to_string());
        }
    }

    for layer in &current_order {
        if seen.contains(&layer.key) {
            continue;
        }
        seen.insert(layer.key.clone());
        top_first_ids.push(layer.key.clone());
    }

    let mut slots = layer_registry.ordered().iter().collect::<Vec<_>>();
    slots.sort_by(|lhs, rhs| {
        lhs.display_order
            .cmp(&rhs.display_order)
            .then_with(|| lhs.key.cmp(&rhs.key))
    });

    for (slot, layer_id) in slots.iter().zip(top_first_ids.iter().rev()) {
        if let Some(layer) = layer_registry.get_by_key(layer_id) {
            layer_runtime.set_stack(layer.id, slot.display_order, slot.z_base);
        }
    }
}

pub(in crate::bridge::host) fn apply_layer_opacity_override(
    layer_registry: &LayerRegistry,
    layer_runtime: &mut LayerRuntime,
    layer_opacities: &BTreeMap<String, f32>,
) {
    for (layer_id, opacity) in layer_opacities {
        let trimmed = layer_id.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(layer) = layer_registry.get_by_key(trimmed) {
            layer_runtime.set_opacity(layer.id, *opacity);
        }
    }
}

pub(in crate::bridge::host) fn reset_layer_opacity_override(
    layer_registry: &LayerRegistry,
    layer_runtime: &mut LayerRuntime,
) {
    for layer in layer_registry.ordered() {
        layer_runtime.set_opacity(layer.id, layer.opacity_default);
    }
}

pub(in crate::bridge::host) fn apply_layer_clip_mask_override(
    layer_registry: &LayerRegistry,
    layer_runtime: &mut LayerRuntime,
    layer_clip_masks: &BTreeMap<String, String>,
) {
    for (layer_id, mask_layer_id) in layer_clip_masks {
        let layer_id = layer_id.trim();
        let mask_layer_id = mask_layer_id.trim();
        if layer_id.is_empty() || mask_layer_id.is_empty() {
            continue;
        }
        let Some(layer) = layer_registry.get_by_key(layer_id) else {
            continue;
        };
        let Some(mask_layer) = layer_registry.get_by_key(mask_layer_id) else {
            continue;
        };
        if layer.id == mask_layer.id {
            continue;
        }
        layer_runtime.set_clip_mask(layer.id, Some(mask_layer.id));
    }
}

pub(in crate::bridge::host) fn apply_layer_waypoint_connections_override(
    layer_registry: &LayerRegistry,
    layer_runtime: &mut LayerRuntime,
    overrides: &BTreeMap<String, bool>,
) {
    for (layer_id, visible) in overrides {
        let trimmed = layer_id.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(layer) = layer_registry.get_by_key(trimmed) else {
            continue;
        };
        let Some(source) = layer.waypoint_source.as_ref() else {
            continue;
        };
        if !source.supports_connections {
            continue;
        }
        layer_runtime.set_waypoint_connections_visible(layer.id, *visible);
    }
}

pub(in crate::bridge::host) fn reset_layer_waypoint_connections_override(
    layer_registry: &LayerRegistry,
    layer_runtime: &mut LayerRuntime,
) {
    for layer in layer_registry.ordered() {
        let visible = layer
            .waypoint_source
            .as_ref()
            .is_some_and(|source| source.supports_connections && source.show_connections_default);
        layer_runtime.set_waypoint_connections_visible(layer.id, visible);
    }
}

pub(in crate::bridge::host) fn apply_layer_waypoint_labels_override(
    layer_registry: &LayerRegistry,
    layer_runtime: &mut LayerRuntime,
    overrides: &BTreeMap<String, bool>,
) {
    for (layer_id, visible) in overrides {
        let trimmed = layer_id.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(layer) = layer_registry.get_by_key(trimmed) else {
            continue;
        };
        let Some(source) = layer.waypoint_source.as_ref() else {
            continue;
        };
        if !source.supports_labels {
            continue;
        }
        layer_runtime.set_waypoint_labels_visible(layer.id, *visible);
    }
}

pub(in crate::bridge::host) fn reset_layer_waypoint_labels_override(
    layer_registry: &LayerRegistry,
    layer_runtime: &mut LayerRuntime,
) {
    for layer in layer_registry.ordered() {
        let visible = layer
            .waypoint_source
            .as_ref()
            .is_some_and(|source| source.supports_labels && source.show_labels_default);
        layer_runtime.set_waypoint_labels_visible(layer.id, visible);
    }
}

pub(in crate::bridge::host) fn apply_layer_point_icon_visibility_override(
    layer_registry: &LayerRegistry,
    layer_runtime: &mut LayerRuntime,
    overrides: &BTreeMap<String, bool>,
) {
    for (layer_id, visible) in overrides {
        let trimmed = layer_id.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(layer) = layer_registry.get_by_key(trimmed) else {
            continue;
        };
        if layer.key != crate::map::layers::FISH_EVIDENCE_LAYER_KEY {
            continue;
        }
        layer_runtime.set_point_icons_visible(layer.id, *visible);
    }
}

pub(in crate::bridge::host) fn reset_layer_point_icon_visibility_override(
    layer_registry: &LayerRegistry,
    layer_runtime: &mut LayerRuntime,
) {
    for layer in layer_registry.ordered() {
        let visible = layer.key == crate::map::layers::FISH_EVIDENCE_LAYER_KEY;
        layer_runtime.set_point_icons_visible(layer.id, visible);
    }
}

pub(in crate::bridge::host) fn apply_layer_point_icon_scale_override(
    layer_registry: &LayerRegistry,
    layer_runtime: &mut LayerRuntime,
    overrides: &BTreeMap<String, f32>,
) {
    for (layer_id, scale) in overrides {
        let trimmed = layer_id.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(layer) = layer_registry.get_by_key(trimmed) else {
            continue;
        };
        if layer.key != crate::map::layers::FISH_EVIDENCE_LAYER_KEY {
            continue;
        }
        layer_runtime.set_point_icon_scale(layer.id, *scale);
    }
}

pub(in crate::bridge::host) fn reset_layer_point_icon_scale_override(
    layer_registry: &LayerRegistry,
    layer_runtime: &mut LayerRuntime,
) {
    for layer in layer_registry.ordered() {
        layer_runtime.set_point_icon_scale(
            layer.id,
            crate::bridge::contract::FISHYMAP_POINT_ICON_SCALE_MIN,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::apply_layer_order_override;
    use crate::bridge::host::snapshot::current_layer_order;
    use crate::map::layers::{LayerRegistry, LayerRuntime};
    use fishystuff_api::models::layers::{
        LayerDescriptor, LayerKind as LayerKindDto, LayerTransformDto, LayersResponse,
        LodPolicyDto, TilesetRef,
    };

    fn raster_descriptor(layer_id: &str, name: &str, display_order: i32) -> LayerDescriptor {
        LayerDescriptor {
            layer_id: layer_id.to_string(),
            name: name.to_string(),
            enabled: true,
            kind: LayerKindDto::TiledRaster,
            transform: LayerTransformDto::IdentityMapSpace,
            tileset: TilesetRef {
                manifest_url: format!("/images/tiles/{layer_id}/v1/tileset.json"),
                tile_url_template: format!("/images/tiles/{layer_id}/v1/{{z}}/{{x}}_{{y}}.png"),
                version: "v1".to_string(),
            },
            tile_px: 512,
            max_level: 0,
            y_flip: false,
            field_source: None,
            field_metadata_source: None,
            vector_source: None,
            lod_policy: LodPolicyDto::default(),
            request_weight: 1.0,
            ui: fishystuff_api::models::layers::LayerUiInfo { display_order },
            pick_mode: fishystuff_api::models::layers::LayerPickMode::None,
        }
    }

    #[test]
    fn layer_order_override_can_reposition_minimap() {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![
                raster_descriptor("minimap", "Minimap", 0),
                raster_descriptor("zone_mask", "Zone Mask", 20),
                raster_descriptor("regions", "Regions", 40),
            ],
        });

        let mut runtime = LayerRuntime::default();
        runtime.sync_to_registry(&registry);

        apply_layer_order_override(
            &registry,
            &mut runtime,
            &[
                "zone_mask".to_string(),
                "minimap".to_string(),
                "regions".to_string(),
            ],
        );

        let ordered = current_layer_order(&registry, &runtime)
            .into_iter()
            .map(|layer| layer.key.clone())
            .collect::<Vec<_>>();
        assert_eq!(ordered, vec!["zone_mask", "minimap", "regions"]);
    }
}
