use super::super::*;

pub(in crate::bridge::host) fn effective_view_snapshot(
    view_mode: &ViewModeState,
    map_view: &Map2dViewState,
    terrain_view: &Terrain3dViewState,
) -> FishyMapViewSnapshot {
    match view_mode.mode {
        ViewMode::Map2D => FishyMapViewSnapshot {
            view_mode: FishyMapViewMode::TwoD,
            camera: FishyMapCameraSnapshot {
                center_world_x: Some(map_view.center_world_x as f64),
                center_world_z: Some(map_view.center_world_z as f64),
                zoom: Some(map_view.zoom as f64),
                ..FishyMapCameraSnapshot::default()
            },
        },
        ViewMode::Terrain3D => FishyMapViewSnapshot {
            view_mode: FishyMapViewMode::ThreeD,
            camera: FishyMapCameraSnapshot {
                pivot_world_x: Some(terrain_view.pivot_world.x as f64),
                pivot_world_y: Some(terrain_view.pivot_world.y as f64),
                pivot_world_z: Some(terrain_view.pivot_world.z as f64),
                yaw: Some(terrain_view.yaw as f64),
                pitch: Some(terrain_view.pitch as f64),
                distance: Some(terrain_view.distance as f64),
                ..FishyMapCameraSnapshot::default()
            },
        },
    }
}
