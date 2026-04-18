use super::super::*;

pub(in crate::bridge::host) fn apply_restored_view(
    camera: &FishyMapCameraSnapshot,
    zoom_bounds: &CameraZoomBounds,
    map_view: &mut Map2dViewState,
) {
    if let Some(value) = camera.center_world_x {
        map_view.center_world_x = value as f32;
    }
    if let Some(value) = camera.center_world_z {
        map_view.center_world_z = value as f32;
    }
    if let Some(value) = camera.zoom {
        map_view.zoom = (value as f32).clamp(zoom_bounds.min_scale, zoom_bounds.max_scale);
    }
}

pub(in crate::bridge::host) fn contract_view_mode(mode: ViewMode) -> FishyMapViewMode {
    let _ = mode;
    FishyMapViewMode::TwoD
}

pub(in crate::bridge::host) fn view_mode_from_contract(mode: FishyMapViewMode) -> ViewMode {
    let _ = mode;
    ViewMode::Map2D
}
