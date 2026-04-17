use super::super::super::persistence::current_patch_range_ids;
use super::super::super::*;
use super::layers::{
    current_layer_clip_mask_overrides, current_layer_opacity_overrides, current_layer_order,
    current_layer_point_icon_scale_overrides, current_layer_point_icon_visibility_overrides,
    current_layer_waypoint_connection_overrides, current_layer_waypoint_label_overrides,
};

pub(in crate::bridge::host) fn effective_filters(
    bridge_input: &FishyMapInputState,
    patch_filter: &PatchFilterState,
    fish_filter: &FishFilterState,
    semantic_filter: &SemanticFieldFilterState,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> FishyMapFiltersState {
    let (ui_from_patch_id, ui_to_patch_id) = current_patch_range_ids(patch_filter);
    let input_from_patch_id = bridge_input
        .filters
        .from_patch_id
        .clone()
        .or_else(|| bridge_input.filters.patch_id.clone());
    let input_to_patch_id = bridge_input
        .filters
        .to_patch_id
        .clone()
        .or_else(|| bridge_input.filters.patch_id.clone());
    let from_patch_id = input_from_patch_id.or(ui_from_patch_id);
    let to_patch_id = input_to_patch_id.or(ui_to_patch_id);
    FishyMapFiltersState {
        fish_ids: fish_filter.selected_fish_ids.clone(),
        zone_rgbs: semantic_filter.selected_zone_rgbs().to_vec(),
        semantic_field_ids_by_layer: semantic_filter.selected_field_ids_by_layer.clone(),
        fish_filter_terms: bridge_input.filters.fish_filter_terms.clone(),
        search_expression: bridge_input.filters.search_expression.clone(),
        search_text: bridge_input.filters.search_text.clone(),
        prize_only: bridge_input.filters.prize_only,
        patch_id: match (&from_patch_id, &to_patch_id) {
            (Some(from_patch_id), Some(to_patch_id)) if from_patch_id == to_patch_id => {
                Some(from_patch_id.clone())
            }
            _ => None,
        },
        from_patch_id,
        to_patch_id,
        layer_ids_visible: (!layer_registry.ordered().is_empty())
            .then(|| current_layer_order(layer_registry, layer_runtime))
            .map(|layers| {
                layers
                    .into_iter()
                    .filter(|layer| layer_runtime.visible(layer.id))
                    .map(|layer| layer.key.clone())
                    .collect()
            }),
        layer_ids_ordered: (!layer_registry.ordered().is_empty()).then(|| {
            current_layer_order(layer_registry, layer_runtime)
                .into_iter()
                .map(|layer| layer.key.clone())
                .collect()
        }),
        layer_filter_binding_ids_disabled_by_layer: bridge_input
            .filters
            .layer_filter_binding_ids_disabled_by_layer
            .clone(),
        layer_opacities: current_layer_opacity_overrides(layer_registry, layer_runtime),
        layer_clip_masks: current_layer_clip_mask_overrides(layer_registry, layer_runtime),
        layer_waypoint_connections_visible: current_layer_waypoint_connection_overrides(
            layer_registry,
            layer_runtime,
        ),
        layer_waypoint_labels_visible: current_layer_waypoint_label_overrides(
            layer_registry,
            layer_runtime,
        ),
        layer_point_icons_visible: current_layer_point_icon_visibility_overrides(
            layer_registry,
            layer_runtime,
        ),
        layer_point_icon_scales: current_layer_point_icon_scale_overrides(
            layer_registry,
            layer_runtime,
        ),
    }
}
