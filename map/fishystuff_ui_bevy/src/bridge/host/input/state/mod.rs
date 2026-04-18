mod bookmarks;
mod filters;
mod layers;
mod search;
mod theme;

use crate::bridge::host::BrowserBridgeState;
use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::map::ui_layers::LayerDebugSettings;
use crate::plugins::api::{LayerFilterBindingOverrideState, MapDisplayState};
use crate::plugins::bookmarks::BookmarkState;
use crate::plugins::camera::Map2dCamera;
use crate::plugins::local_layers::sync_display_layer_controls;
use crate::prelude::*;
use bevy::window::RequestRedraw;

pub(in crate::bridge::host) use search::resolve_browser_search_filters;

pub(in crate::bridge::host) fn apply_browser_input_state(
    bridge: Res<BrowserBridgeState>,
    mut layer_filter_binding_overrides: ResMut<LayerFilterBindingOverrideState>,
    mut bookmarks: ResMut<BookmarkState>,
    mut display_state: ResMut<MapDisplayState>,
    mut debug_layers: ResMut<LayerDebugSettings>,
    mut layer_runtime: ResMut<LayerRuntime>,
    layer_registry: Res<LayerRegistry>,
    mut clear_color: ResMut<ClearColor>,
    mut map_camera_q: Query<&mut Camera, With<Map2dCamera>>,
    mut request_redraw: MessageWriter<RequestRedraw>,
) {
    crate::perf_scope!("bridge.state_apply");
    if !bridge.is_changed() && !layer_registry.is_changed() {
        return;
    }

    crate::perf_counter_add!("bridge.state_apply.count", 1);
    layer_runtime.sync_to_registry(&layer_registry);
    bookmarks::apply_bookmarks(&bridge.input, &mut bookmarks);
    filters::apply_display_flags(&bridge.input, &mut display_state, &mut debug_layers);
    filters::apply_layer_filter_binding_overrides(
        &bridge.input,
        &mut layer_filter_binding_overrides,
    );
    layers::apply_layer_filters(&bridge.input, &layer_registry, &mut layer_runtime);
    sync_display_layer_controls(&mut display_state, &layer_registry, &layer_runtime);
    theme::apply_theme_background(&bridge.input, &mut clear_color, &mut map_camera_q);
    record_bridge_input_metrics(&bridge.input);
    if bridge.is_changed() {
        request_redraw.write(RequestRedraw);
    }
}

fn record_bridge_input_metrics(input: &crate::bridge::contract::FishyMapInputState) {
    let semantic_layers = input.filters.semantic_field_ids_by_layer.len();
    let semantic_fields = input
        .filters
        .semantic_field_ids_by_layer
        .values()
        .map(Vec::len)
        .sum::<usize>();
    let disabled_binding_layers = input
        .filters
        .layer_filter_binding_ids_disabled_by_layer
        .as_ref()
        .map(|disabled| disabled.len())
        .unwrap_or(0);
    let disabled_binding_ids = input
        .filters
        .layer_filter_binding_ids_disabled_by_layer
        .as_ref()
        .map(|disabled| disabled.values().map(Vec::len).sum::<usize>())
        .unwrap_or(0);

    crate::perf_last!(
        "bridge.input.filters.fish_ids",
        input.filters.fish_ids.len()
    );
    crate::perf_last!(
        "bridge.input.filters.zone_rgbs",
        input.filters.zone_rgbs.len()
    );
    crate::perf_last!(
        "bridge.input.filters.fish_filter_terms",
        input.filters.fish_filter_terms.len()
    );
    crate::perf_last!("bridge.input.filters.semantic_layers", semantic_layers);
    crate::perf_last!("bridge.input.filters.semantic_fields", semantic_fields);
    crate::perf_last!(
        "bridge.input.filters.search_nodes",
        input.filters.search_expression.node_count()
    );
    crate::perf_last!(
        "bridge.input.filters.search_terms",
        input.filters.search_expression.term_count()
    );
    crate::perf_last!(
        "bridge.input.filters.search_max_depth",
        input.filters.search_expression.max_depth()
    );
    crate::perf_last!(
        "bridge.input.filters.visible_layers",
        input
            .filters
            .layer_ids_visible
            .as_ref()
            .map(Vec::len)
            .unwrap_or(0)
    );
    crate::perf_last!(
        "bridge.input.filters.ordered_layers",
        input
            .filters
            .layer_ids_ordered
            .as_ref()
            .map(Vec::len)
            .unwrap_or(0)
    );
    crate::perf_last!(
        "bridge.input.filters.disabled_binding_layers",
        disabled_binding_layers
    );
    crate::perf_last!(
        "bridge.input.filters.disabled_binding_ids",
        disabled_binding_ids
    );
    crate::perf_last!(
        "bridge.input.filters.clip_mask_layers",
        input
            .filters
            .layer_clip_masks
            .as_ref()
            .map(|clip_masks| clip_masks.len())
            .unwrap_or(0)
    );
    crate::perf_last!(
        "bridge.input.shared_fish.caught_ids",
        input.ui.shared_fish_state.caught_ids.len()
    );
    crate::perf_last!(
        "bridge.input.shared_fish.favourite_ids",
        input.ui.shared_fish_state.favourite_ids.len()
    );
}
