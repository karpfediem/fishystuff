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
use crate::plugins::camera::{Map2dCamera, Terrain3dCamera};
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
    mut map_camera_q: Query<&mut Camera, (With<Map2dCamera>, Without<Terrain3dCamera>)>,
    mut terrain_camera_q: Query<&mut Camera, (With<Terrain3dCamera>, Without<Map2dCamera>)>,
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
    theme::apply_theme_background(
        &bridge.input,
        &mut clear_color,
        &mut map_camera_q,
        &mut terrain_camera_q,
    );
    if bridge.is_changed() {
        request_redraw.write(RequestRedraw);
    }
}
