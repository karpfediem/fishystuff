use super::super::super::persistence::{apply_restored_view, view_mode_from_contract};
use crate::bridge::contract::FishyMapCommands;
use crate::map::camera::map2d::{reset_map2d_view, Map2dViewState};
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::plugins::camera::CameraZoomBounds;

pub(super) fn apply_view_commands(
    command: &FishyMapCommands,
    zoom_bounds: &CameraZoomBounds,
    view_mode: &mut ViewModeState,
    map_view: &mut Map2dViewState,
) {
    if command.reset_view.unwrap_or(false) {
        reset_map2d_view(map_view);
        map_view.zoom = zoom_bounds
            .fit_scale
            .clamp(zoom_bounds.min_scale, zoom_bounds.max_scale);
    }

    if let Some(mode) = command.set_view_mode {
        set_view_mode(view_mode, view_mode_from_contract(mode));
    }

    if let Some(view) = command.restore_view.as_ref() {
        set_view_mode(view_mode, view_mode_from_contract(view.view_mode));
        apply_restored_view(&view.camera, zoom_bounds, map_view);
    }
}

fn set_view_mode(view_mode: &mut ViewModeState, mode: ViewMode) {
    view_mode.mode = mode;
}
