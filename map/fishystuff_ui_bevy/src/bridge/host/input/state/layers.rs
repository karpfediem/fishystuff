use super::super::super::persistence::{
    apply_layer_clip_mask_override, apply_layer_opacity_override, apply_layer_order_override,
    apply_layer_point_icon_scale_override, apply_layer_point_icon_visibility_override,
    apply_layer_waypoint_connections_override, apply_layer_waypoint_labels_override,
    reset_layer_opacity_override, reset_layer_point_icon_scale_override,
    reset_layer_point_icon_visibility_override, reset_layer_waypoint_connections_override,
    reset_layer_waypoint_labels_override,
};
use crate::bridge::contract::FishyMapInputState;
use crate::map::layers::{LayerRegistry, LayerRuntime};

pub(super) fn apply_layer_filters(
    input: &FishyMapInputState,
    layer_registry: &LayerRegistry,
    layer_runtime: &mut LayerRuntime,
) {
    if let Some(visible_layers) = input.filters.layer_ids_visible.as_ref() {
        for spec in layer_registry.ordered() {
            let visible = visible_layers.iter().any(|id| id == &spec.key);
            layer_runtime.set_visible(spec.id, visible);
        }
    }
    if let Some(ordered_layers) = input.filters.layer_ids_ordered.as_ref() {
        apply_layer_order_override(layer_registry, layer_runtime, ordered_layers);
    }
    reset_layer_opacity_override(layer_registry, layer_runtime);
    if let Some(layer_opacities) = input.filters.layer_opacities.as_ref() {
        apply_layer_opacity_override(layer_registry, layer_runtime, layer_opacities);
    }
    layer_runtime.clear_clip_masks();
    if let Some(layer_clip_masks) = input.filters.layer_clip_masks.as_ref() {
        apply_layer_clip_mask_override(layer_registry, layer_runtime, layer_clip_masks);
    }
    reset_layer_waypoint_connections_override(layer_registry, layer_runtime);
    if let Some(overrides) = input.filters.layer_waypoint_connections_visible.as_ref() {
        apply_layer_waypoint_connections_override(layer_registry, layer_runtime, overrides);
    }
    reset_layer_waypoint_labels_override(layer_registry, layer_runtime);
    if let Some(overrides) = input.filters.layer_waypoint_labels_visible.as_ref() {
        apply_layer_waypoint_labels_override(layer_registry, layer_runtime, overrides);
    }
    reset_layer_point_icon_visibility_override(layer_registry, layer_runtime);
    if let Some(overrides) = input.filters.layer_point_icons_visible.as_ref() {
        apply_layer_point_icon_visibility_override(layer_registry, layer_runtime, overrides);
    }
    reset_layer_point_icon_scale_override(layer_registry, layer_runtime);
    if let Some(overrides) = input.filters.layer_point_icon_scales.as_ref() {
        apply_layer_point_icon_scale_override(layer_registry, layer_runtime, overrides);
    }
}
