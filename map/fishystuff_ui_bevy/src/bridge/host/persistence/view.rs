use super::super::*;

pub(in crate::bridge::host) fn apply_restored_view(
    camera: &FishyMapCameraSnapshot,
    zoom_bounds: &CameraZoomBounds,
    map_view: &mut Map2dViewState,
    terrain_view: &mut Terrain3dViewState,
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
    if let Some(value) = camera.pivot_world_x {
        terrain_view.pivot_world.x = value as f32;
    }
    if let Some(value) = camera.pivot_world_y {
        terrain_view.pivot_world.y = value as f32;
    }
    if let Some(value) = camera.pivot_world_z {
        terrain_view.pivot_world.z = value as f32;
    }
    if let Some(value) = camera.yaw {
        terrain_view.yaw = value as f32;
    }
    if let Some(value) = camera.pitch {
        terrain_view.pitch = value as f32;
    }
    if let Some(value) = camera.distance {
        terrain_view.set_distance_clamped(value as f32);
    }
}

pub(in crate::bridge::host) fn contract_view_mode(mode: ViewMode) -> FishyMapViewMode {
    match mode {
        ViewMode::Map2D => FishyMapViewMode::TwoD,
        ViewMode::Terrain3D => FishyMapViewMode::ThreeD,
    }
}

pub(in crate::bridge::host) fn view_mode_from_contract(mode: FishyMapViewMode) -> ViewMode {
    match mode {
        FishyMapViewMode::TwoD => ViewMode::Map2D,
        FishyMapViewMode::ThreeD => ViewMode::Terrain3D,
    }
}

pub(in crate::bridge::host) fn rgb_u32_to_tuple(value: u32) -> (u8, u8, u8) {
    (
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    )
}
