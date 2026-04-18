use super::super::*;

pub(in crate::bridge::host) fn effective_view_snapshot(
    view_mode: &ViewModeState,
    map_view: &Map2dViewState,
) -> FishyMapViewSnapshot {
    let _ = view_mode;
    FishyMapViewSnapshot {
        view_mode: FishyMapViewMode::TwoD,
        camera: FishyMapCameraSnapshot {
            center_world_x: Some(map_view.center_world_x as f64),
            center_world_z: Some(map_view.center_world_z as f64),
            zoom: Some(map_view.zoom as f64),
            ..FishyMapCameraSnapshot::default()
        },
    }
}
