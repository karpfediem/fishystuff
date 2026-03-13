use crate::map::spaces::world::MapToWorld;
use crate::prelude::*;

#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct Map2dViewState {
    pub center_world_x: f32,
    pub center_world_z: f32,
    pub zoom: f32,
}

impl Default for Map2dViewState {
    fn default() -> Self {
        default_map2d_view_state()
    }
}

pub fn reset_map2d_view(state: &mut Map2dViewState) {
    *state = default_map2d_view_state();
}

pub fn apply_map2d_camera_state(
    view: &Map2dViewState,
    transform: &mut Transform,
    projection: &mut Projection,
) {
    let mut ortho = match projection {
        Projection::Orthographic(existing) => existing.clone(),
        _ => OrthographicProjection::default_2d(),
    };
    ortho.scale = view.zoom.max(1e-5);
    *projection = Projection::Orthographic(ortho);
    transform.translation = Vec3::new(view.center_world_x, view.center_world_z, 1000.0);
    transform.rotation = Quat::IDENTITY;
    transform.scale = Vec3::ONE;
}

fn default_map2d_view_state() -> Map2dViewState {
    let world_bounds = MapToWorld::default().world_bounds();
    Map2dViewState {
        center_world_x: ((world_bounds.min.x + world_bounds.max.x) * 0.5) as f32,
        center_world_z: ((world_bounds.min.z + world_bounds.max.z) * 0.5) as f32,
        zoom: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_map2d_camera_state, reset_map2d_view, Map2dViewState};
    use bevy::prelude::{Projection, Transform, Vec3};

    #[test]
    fn apply_map2d_camera_state_restores_flat_camera() {
        let map_state = Map2dViewState {
            center_world_x: 120.0,
            center_world_z: -340.0,
            zoom: 2.5,
        };
        let mut transform = Transform::from_xyz(1.0, 2.0, 3.0);
        let mut projection = Projection::Perspective(Default::default());

        apply_map2d_camera_state(&map_state, &mut transform, &mut projection);

        let Projection::Orthographic(ortho) = projection else {
            panic!("expected orthographic projection");
        };
        assert_eq!(ortho.scale, 2.5);
        assert_eq!(transform.translation, Vec3::new(120.0, -340.0, 1000.0));
    }

    #[test]
    fn reset_map2d_view_returns_valid_defaults() {
        let mut view = Map2dViewState {
            center_world_x: 99.0,
            center_world_z: -42.0,
            zoom: 0.25,
        };
        reset_map2d_view(&mut view);

        assert_eq!(view.zoom, 1.0);
    }
}
