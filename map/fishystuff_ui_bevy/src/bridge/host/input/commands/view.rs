use super::super::super::persistence::{apply_restored_view, view_mode_from_contract};
use crate::bridge::contract::FishyMapCommands;
use crate::map::camera::map2d::{reset_map2d_view, Map2dViewState};
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::camera::terrain3d::{reset_terrain3d_view, Terrain3dViewState};
use crate::plugins::camera::CameraZoomBounds;

pub(super) fn apply_view_commands(
    command: &FishyMapCommands,
    zoom_bounds: &CameraZoomBounds,
    view_mode: &mut ViewModeState,
    map_view: &mut Map2dViewState,
    terrain_view: &mut Terrain3dViewState,
) {
    if command.reset_view.unwrap_or(false) {
        match view_mode.mode {
            ViewMode::Map2D => {
                reset_map2d_view(map_view);
                map_view.zoom = zoom_bounds
                    .fit_scale
                    .clamp(zoom_bounds.min_scale, zoom_bounds.max_scale);
            }
            ViewMode::Terrain3D => {
                reset_terrain3d_view(terrain_view);
            }
        }
    }

    if let Some(mode) = command.set_view_mode {
        set_view_mode(view_mode, view_mode_from_contract(mode));
    }

    if let Some(view) = command.restore_view.as_ref() {
        set_view_mode(view_mode, view_mode_from_contract(view.view_mode));
        apply_restored_view(&view.camera, zoom_bounds, map_view, terrain_view);
    }
}

fn set_view_mode(view_mode: &mut ViewModeState, mode: ViewMode) {
    view_mode.mode = mode;
    if mode == ViewMode::Terrain3D {
        view_mode.terrain_initialized = true;
    }
}
