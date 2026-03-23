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
    cursor_state: Res<CursorState>,
    camera_q: Query<(&Camera, &Transform), With<Terrain3dCamera>>,
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
    let cursor = window.cursor_position().or(cursor_state.last_pos);
    estimate.cursor_world = cursor.and_then(|cursor| {
        let (camera, camera_transform) = camera_q.single().ok()?;
        let ray = camera
            .viewport_to_world(&GlobalTransform::from(*camera_transform), cursor)
            .ok()?;
        cursor_world_on_ground_plane(ray)
    });
}

fn cursor_world_on_ground_plane(ray: Ray3d) -> Option<WorldPoint> {
    let point = ray.plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Dir3::Y))?;
    Some(WorldPoint::new(point.x as f64, point.z as f64))
}

#[cfg(test)]
mod tests {
    use super::cursor_world_on_ground_plane;
    use crate::map::spaces::WorldPoint;
    use bevy::math::{Dir3, Ray3d};
    use bevy::prelude::Vec3;

    #[test]
    fn cursor_world_on_ground_plane_intersects_y_zero_plane() {
        let ray = Ray3d::new(
            Vec3::new(25.0, 100.0, -40.0),
            Dir3::new(Vec3::new(0.0, -1.0, 0.0)).expect("direction"),
        );
        assert_eq!(
            cursor_world_on_ground_plane(ray),
            Some(WorldPoint::new(25.0, -40.0))
        );
    }

    #[test]
    fn cursor_world_on_ground_plane_rejects_upward_rays() {
        let ray = Ray3d::new(
            Vec3::new(0.0, 10.0, 0.0),
            Dir3::new(Vec3::new(0.0, 1.0, 0.0)).expect("direction"),
        );
        assert_eq!(cursor_world_on_ground_plane(ray), None);
    }
}
