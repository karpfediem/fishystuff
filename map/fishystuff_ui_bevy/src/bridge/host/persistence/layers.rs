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
        if trimmed.is_empty() || trimmed == "minimap" || !seen.insert(trimmed.to_string()) {
            continue;
        }
        if layer_registry.id_by_key(trimmed).is_some() {
            top_first_ids.push(trimmed.to_string());
        }
    }

    for layer in &current_order {
        if layer.key == "minimap" || seen.contains(&layer.key) {
            continue;
        }
        seen.insert(layer.key.clone());
        top_first_ids.push(layer.key.clone());
    }

    if let Some(minimap) = layer_registry.get_by_key("minimap") {
        top_first_ids.push(minimap.key.clone());
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
        if trimmed.is_empty() || trimmed == "minimap" {
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
        if layer_id.is_empty() || mask_layer_id.is_empty() || layer_id == "minimap" {
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
