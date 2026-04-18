use bevy::camera::ScalingMode;
use bevy::window::Window;

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

pub fn map2d_cursor_to_world(
    window: &Window,
    projection: &Projection,
    transform: &Transform,
    cursor: Vec2,
) -> Option<Vec2> {
    let Projection::Orthographic(ortho) = projection else {
        return None;
    };
    map2d_cursor_to_world_with_scale(window, ortho, transform, cursor, ortho.scale)
}

pub fn map2d_cursor_to_world_with_scale(
    window: &Window,
    projection: &OrthographicProjection,
    transform: &Transform,
    cursor: Vec2,
    scale: f32,
) -> Option<Vec2> {
    let (proj_w, proj_h) = match projection.scaling_mode {
        ScalingMode::WindowSize => (window.width(), window.height()),
        ScalingMode::AutoMin {
            min_width,
            min_height,
        } => {
            if window.width() * min_height > min_width * window.height() {
                (window.width() * min_height / window.height(), min_height)
            } else {
                (min_width, window.height() * min_width / window.width())
            }
        }
        ScalingMode::AutoMax {
            max_width,
            max_height,
        } => {
            if window.width() * max_height < max_width * window.height() {
                (window.width() * max_height / window.height(), max_height)
            } else {
                (max_width, window.height() * max_width / window.width())
            }
        }
        ScalingMode::FixedVertical { viewport_height } => (
            window.width() * viewport_height / window.height(),
            viewport_height,
        ),
        ScalingMode::FixedHorizontal { viewport_width } => (
            viewport_width,
            window.height() * viewport_width / window.width(),
        ),
        ScalingMode::Fixed { width, height } => (width, height),
    };

    if window.width() <= 0.0 || window.height() <= 0.0 {
        return None;
    }

    let origin_x = proj_w * projection.viewport_origin.x;
    let origin_y = proj_h * projection.viewport_origin.y;
    let min_x = scale * -origin_x;
    let max_x = scale * (proj_w - origin_x);
    let min_y = scale * -origin_y;
    let max_y = scale * (proj_h - origin_y);

    let mut vp = cursor;
    vp.y = window.height() - vp.y;

    let nx = (vp.x / window.width()).clamp(0.0, 1.0);
    let ny = (vp.y / window.height()).clamp(0.0, 1.0);
    let local_x = min_x + nx * (max_x - min_x);
    let local_y = min_y + ny * (max_y - min_y);
    let world = transform
        .to_matrix()
        .transform_point3(Vec3::new(local_x, local_y, 0.0));
    Some(Vec2::new(world.x, world.y))
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
    use super::{
        apply_map2d_camera_state, map2d_cursor_to_world, map2d_cursor_to_world_with_scale,
        reset_map2d_view, Map2dViewState,
    };
    use bevy::prelude::{OrthographicProjection, Projection, Transform, Vec2, Vec3, Window};

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

    #[test]
    fn cursor_to_world_wrapper_uses_projection_scale() {
        let window = Window::default();
        let mut ortho = OrthographicProjection::default_2d();
        ortho.scale = 2.5;
        let projection = Projection::Orthographic(ortho.clone());
        let transform = Transform::from_xyz(120.0, -340.0, 1000.0);
        let cursor = Vec2::new(320.0, 240.0);

        let world = map2d_cursor_to_world(&window, &projection, &transform, cursor);
        let expected =
            map2d_cursor_to_world_with_scale(&window, &ortho, &transform, cursor, ortho.scale);

        assert_eq!(world, expected);
    }

    #[test]
    fn cursor_to_world_with_scale_supports_anchor_preserving_zoom() {
        let window = Window::default();
        let ortho = OrthographicProjection::default_2d();
        let cursor = Vec2::new(420.0, 180.0);
        let old_scale = 2.0;
        let new_scale = 0.75;
        let before_transform = Transform::from_xyz(120.0, -340.0, 1000.0);

        let before =
            map2d_cursor_to_world_with_scale(&window, &ortho, &before_transform, cursor, old_scale)
                .expect("world point before zoom");
        let after =
            map2d_cursor_to_world_with_scale(&window, &ortho, &before_transform, cursor, new_scale)
                .expect("world point after zoom");

        let delta = before - after;
        let mut anchored_transform = before_transform;
        anchored_transform.translation.x += delta.x;
        anchored_transform.translation.y += delta.y;

        let anchored = map2d_cursor_to_world_with_scale(
            &window,
            &ortho,
            &anchored_transform,
            cursor,
            new_scale,
        )
        .expect("anchored world point");

        assert!((anchored.x - before.x).abs() < 1e-4);
        assert!((anchored.y - before.y).abs() < 1e-4);
    }
}
