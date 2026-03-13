mod fish;
mod selection;
mod view;

use crate::bridge::host::BrowserBridgeState;
use crate::map::camera::map2d::Map2dViewState;
use crate::map::camera::mode::ViewModeState;
use crate::map::camera::terrain3d::Terrain3dViewState;
use crate::plugins::api::{
    ApiBootstrapState, FishCatalog, FishFilterState, PatchFilterState, PendingRequests,
    SelectionState,
};
use crate::plugins::camera::CameraZoomBounds;
use crate::prelude::*;

pub(in crate::bridge::host) fn apply_browser_commands(
    mut bridge: ResMut<BrowserBridgeState>,
    zoom_bounds: Res<CameraZoomBounds>,
    bootstrap: Res<ApiBootstrapState>,
    fish: Res<FishCatalog>,
    mut fish_filter: ResMut<FishFilterState>,
    patch_filter: Res<PatchFilterState>,
    mut selection: ResMut<SelectionState>,
    mut pending: ResMut<PendingRequests>,
    mut view_mode: ResMut<ViewModeState>,
    mut map_view: ResMut<Map2dViewState>,
    mut terrain_view: ResMut<Terrain3dViewState>,
) {
    let mut commands = Vec::new();
    std::mem::swap(&mut bridge.pending_commands, &mut commands);

    for command in commands {
        view::apply_view_commands(
            &command,
            &zoom_bounds,
            &mut view_mode,
            &mut map_view,
            &mut terrain_view,
        );

        if let Some(fish_id) = command.focus_fish_id {
            fish::apply_focus_fish_command(&mut bridge, &fish, &mut fish_filter, fish_id);
        }

        if let Some(zone_rgb) = command.select_zone_rgb {
            selection::apply_zone_selection_command(
                &bootstrap,
                &patch_filter,
                &mut selection,
                &mut pending,
                zone_rgb,
            );
        }
    }
}
