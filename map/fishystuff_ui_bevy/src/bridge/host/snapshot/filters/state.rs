use super::super::super::*;
use super::layers::{
    current_layer_clip_mask_overrides, current_layer_opacity_overrides, current_layer_order,
    current_layer_point_icon_scale_overrides, current_layer_point_icon_visibility_overrides,
    current_layer_waypoint_connection_overrides, current_layer_waypoint_label_overrides,
};

fn effective_search_expression_for_reporting(
    bridge_input: &FishyMapInputState,
    search_expression: &SearchExpressionState,
) -> FishyMapSearchExpressionNode {
    if !search_expression.expression.is_empty() {
        return search_expression.expression.clone();
    }
    crate::map::search_filters::effective_search_expression(
        bridge_input,
        &[],
        &std::collections::BTreeMap::new(),
    )
}

pub(in crate::bridge::host) fn effective_filter_snapshot(
    bridge_input: &FishyMapInputState,
    search_expression: &SearchExpressionState,
    layer_effective_filters: &LayerEffectiveFilterState,
) -> FishyMapEffectiveFiltersSnapshot {
    let effective_search_expression =
        effective_search_expression_for_reporting(bridge_input, search_expression);
    let shared_fish_state = if search_expression.shared_fish_state.is_empty() {
        bridge_input.ui.shared_fish_state.clone()
    } else {
        search_expression.shared_fish_state.clone()
    };
    FishyMapEffectiveFiltersSnapshot {
        search_expression: effective_search_expression,
        shared_fish_state,
        zone_membership_by_layer: layer_effective_filters
            .zone_membership_filters()
            .map(|(layer_id, filter)| {
                let mut zone_rgbs = filter.zone_rgbs.iter().copied().collect::<Vec<_>>();
                zone_rgbs.sort_unstable();
                (
                    layer_id.to_string(),
                    FishyMapEffectiveZoneMembershipFilterSnapshot {
                        active: filter.active,
                        zone_rgbs,
                        revision: filter.revision,
                    },
                )
            })
            .collect(),
        semantic_field_filters_by_layer: layer_effective_filters
            .semantic_field_filters()
            .map(|(layer_id, filter)| {
                (
                    layer_id.to_string(),
                    FishyMapEffectiveSemanticFieldFilterSnapshot {
                        active: filter.active,
                        field_ids: filter.field_ids.clone(),
                        revision: filter.revision,
                    },
                )
            })
            .collect(),
    }
}

pub(in crate::bridge::host) fn effective_filters(
    bridge_input: &FishyMapInputState,
    search_expression: &SearchExpressionState,
    layer_effective_filters: &LayerEffectiveFilterState,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> FishyMapFiltersState {
    let effective_filters =
        effective_filter_snapshot(bridge_input, search_expression, layer_effective_filters);
    let projection =
        FishyMapSearchProjection::from_expression(&effective_filters.search_expression);
    FishyMapFiltersState {
        fish_ids: projection.fish_ids,
        zone_rgbs: projection.zone_rgbs,
        semantic_field_ids_by_layer: projection.semantic_field_ids_by_layer,
        fish_filter_terms: projection.fish_filter_terms,
        search_expression: effective_filters.search_expression,
        search_text: bridge_input.filters.search_text.clone(),
        prize_only: bridge_input.filters.prize_only,
        patch_id: projection.patch_id,
        from_patch_id: projection.from_patch_id,
        to_patch_id: projection.to_patch_id,
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
