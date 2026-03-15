mod filters;
mod layers;
mod theme;

use crate::bridge::host::BrowserBridgeState;
use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::map::ui_layers::LayerDebugSettings;
use crate::plugins::api::{FishFilterState, MapDisplayState, PatchFilterState};
use crate::plugins::camera::{Map2dCamera, Terrain3dCamera};
use crate::prelude::*;

pub(in crate::bridge::host) fn apply_browser_input_state(
    bridge: Res<BrowserBridgeState>,
    mut patch_filter: ResMut<PatchFilterState>,
    mut fish_filter: ResMut<FishFilterState>,
    mut display_state: ResMut<MapDisplayState>,
    mut debug_layers: ResMut<LayerDebugSettings>,
    mut layer_runtime: ResMut<LayerRuntime>,
    layer_registry: Res<LayerRegistry>,
    mut clear_color: ResMut<ClearColor>,
    mut map_camera_q: Query<&mut Camera, (With<Map2dCamera>, Without<Terrain3dCamera>)>,
    mut terrain_camera_q: Query<&mut Camera, (With<Terrain3dCamera>, Without<Map2dCamera>)>,
) {
    if !bridge.is_changed() && !layer_registry.is_changed() {
        return;
    }

    layer_runtime.sync_to_registry(&layer_registry);
    filters::apply_display_flags(&bridge.input, &mut display_state, &mut debug_layers);
    filters::apply_fish_filters(&bridge.input, &mut fish_filter);
    filters::apply_patch_filters(&bridge.input, &mut patch_filter);
    layers::apply_layer_filters(&bridge.input, &layer_registry, &mut layer_runtime);
    theme::apply_theme_background(
        &bridge.input,
        &mut clear_color,
        &mut map_camera_q,
        &mut terrain_camera_q,
    );
}
