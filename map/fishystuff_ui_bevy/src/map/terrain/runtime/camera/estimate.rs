use super::*;

#[derive(Resource, Default)]
pub struct TerrainViewEstimate {
    pub view_world: Option<WorldRect>,
    pub cursor_world: Option<WorldPoint>,
}

pub(in crate::map::terrain::runtime) fn update_view_estimate(
    mode: Res<ViewModeState>,
    view: Res<Terrain3dViewState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut estimate: ResMut<TerrainViewEstimate>,
) {
    if mode.mode != ViewMode::Terrain3D {
        estimate.view_world = None;
        estimate.cursor_world = None;
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    estimate.view_world = Some(estimate_view_world_rect(
        *view,
        Vec2::new(window.width(), window.height()),
    ));
    estimate.cursor_world = None;
}
