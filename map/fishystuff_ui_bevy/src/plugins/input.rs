use bevy::camera::ScalingMode;
use bevy::ecs::system::SystemParam;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input::ButtonInput;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{CursorMoved, PrimaryWindow};

use crate::map::camera::map2d::{apply_map2d_camera_state, reset_map2d_view, Map2dViewState};
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::terrain::mode::CameraControlMutationFlags;
use crate::plugins::camera::{CameraZoomBounds, Map2dCamera};
use crate::plugins::ui::{UiPointerBlocker, UiPointerCapture};
use crate::prelude::*;

#[derive(Resource, Default)]
pub struct PanState {
    pub dragging: bool,
    pub last_cursor: Vec2,
    pub drag_distance: f32,
}

#[derive(Resource, Default)]
pub struct CursorState {
    pub last_pos: Option<Vec2>,
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanState>()
            .init_resource::<CursorState>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_message::<MouseWheel>()
            .add_message::<CursorMoved>()
            .add_message::<KeyboardInput>()
            .add_systems(
                Update,
                (
                    track_cursor,
                    update_ui_pointer_capture,
                    update_map2d_camera_controls,
                ),
            );
    }
}

fn map2d_controls_should_run(mode: ViewMode, camera_active: bool) -> bool {
    mode == ViewMode::Map2D && camera_active
}

fn track_cursor(
    windows: Query<Entity, With<PrimaryWindow>>,
    mut cursor: ResMut<CursorState>,
    mut cursor_moved: MessageReader<CursorMoved>,
) {
    let Ok(window_entity) = windows.single() else {
        return;
    };
    for ev in cursor_moved.read() {
        if ev.window == window_entity {
            cursor.last_pos = Some(ev.position);
        }
    }
}

fn update_map2d_camera_controls(mut controls: Map2dCameraControls<'_, '_>) {
    crate::perf_scope!("camera.2d_update");
    if controls.view_mode.mode != ViewMode::Map2D {
        controls.pan.dragging = false;
        return;
    }
    let Ok(window) = controls.windows.single() else {
        return;
    };
    let Ok((camera, mut projection, mut transform)) = controls.camera_q.single_mut() else {
        return;
    };
    if !map2d_controls_should_run(controls.view_mode.mode, camera.is_active) {
        controls.pan.dragging = false;
        return;
    }
    let ortho_template = match &*projection {
        Projection::Orthographic(existing) => existing.clone(),
        _ => OrthographicProjection::default_2d(),
    };
    let mut working_center = Vec2::new(
        controls.view_state.center_world_x,
        controls.view_state.center_world_z,
    );
    let unclamped_zoom = controls.view_state.zoom.max(1e-5);
    let mut working_zoom = unclamped_zoom.clamp(
        controls.zoom_bounds.min_scale,
        controls.zoom_bounds.max_scale,
    );
    let mut working_transform =
        Transform::from_translation(Vec3::new(working_center.x, working_center.y, 1000.0));
    working_transform.rotation = Quat::IDENTITY;
    working_transform.scale = Vec3::ONE;
    let mut changed = (working_zoom - unclamped_zoom).abs() > 1e-6;

    let ui_input_blocked = controls.ui_capture.blocked || controls.ui_capture.text_input_active;
    if ui_input_blocked {
        controls.pan.dragging = false;
    }

    if controls.key_buttons.just_pressed(KeyCode::Home) && !controls.ui_capture.text_input_active {
        reset_map2d_view(&mut controls.view_state);
        controls.view_state.zoom = controls.zoom_bounds.fit_scale.clamp(
            controls.zoom_bounds.min_scale,
            controls.zoom_bounds.max_scale,
        );
        working_center = Vec2::new(
            controls.view_state.center_world_x,
            controls.view_state.center_world_z,
        );
        working_zoom = controls.view_state.zoom;
        working_transform.translation.x = working_center.x;
        working_transform.translation.y = working_center.y;
        changed = true;
    }
    let cursor_pos = window.cursor_position().or(controls.cursor.last_pos);

    if controls.mouse_buttons.just_pressed(MouseButton::Left) && !ui_input_blocked {
        if let Some(pos) = cursor_pos {
            controls.pan.dragging = true;
            controls.pan.last_cursor = pos;
            controls.pan.drag_distance = 0.0;
        }
    }

    if controls.mouse_buttons.pressed(MouseButton::Left)
        && controls.pan.dragging
        && !ui_input_blocked
    {
        if let (Some(prev), Some(curr)) = (
            screen_to_world_with_scale(
                window,
                &ortho_template,
                &working_transform,
                controls.pan.last_cursor,
                working_zoom,
            ),
            cursor_pos.and_then(|pos| {
                screen_to_world_with_scale(
                    window,
                    &ortho_template,
                    &working_transform,
                    pos,
                    working_zoom,
                )
            }),
        ) {
            let delta = prev - curr;
            working_center.x += delta.x;
            working_center.y += delta.y;
            working_transform.translation.x = working_center.x;
            working_transform.translation.y = working_center.y;
            controls.pan.drag_distance += delta.length();
            changed = true;
        }
        if let Some(pos) = cursor_pos {
            controls.pan.last_cursor = pos;
        }
    }

    if controls.mouse_buttons.just_released(MouseButton::Left) {
        controls.pan.dragging = false;
    }

    for ev in controls.mouse_wheel.read() {
        if ui_input_blocked {
            continue;
        }
        let mut scroll = ev.y;
        if matches!(ev.unit, MouseScrollUnit::Pixel) {
            scroll /= 100.0;
        }
        scroll = scroll.clamp(-10.0, 10.0);
        let zoom_delta = 2.0_f32.powf(-scroll / ZOOM_TICKS_PER_DOUBLE);
        let new_scale = (working_zoom * zoom_delta).clamp(
            controls.zoom_bounds.min_scale,
            controls.zoom_bounds.max_scale,
        );
        if let Some(cursor) = cursor_pos {
            let before = screen_to_world_with_scale(
                window,
                &ortho_template,
                &working_transform,
                cursor,
                working_zoom,
            );
            let after = screen_to_world_with_scale(
                window,
                &ortho_template,
                &working_transform,
                cursor,
                new_scale,
            );
            if let (Some(before), Some(after)) = (before, after) {
                let delta = before - after;
                working_center.x += delta.x;
                working_center.y += delta.y;
                working_transform.translation.x = working_center.x;
                working_transform.translation.y = working_center.y;
            }
        }
        working_zoom = new_scale;
        changed = true;
    }

    if changed {
        controls.view_state.center_world_x = working_center.x;
        controls.view_state.center_world_z = working_center.y;
        controls.view_state.zoom = working_zoom;
        controls.control_mutations.map2d_updated = true;
    }
    // Always enforce the known 2D pose/projection to avoid transform contamination from 3D mode.
    apply_map2d_camera_state(&controls.view_state, &mut transform, &mut projection);
}

#[derive(SystemParam)]
struct Map2dCameraControls<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    key_buttons: Res<'w, ButtonInput<KeyCode>>,
    mouse_wheel: MessageReader<'w, 's, MouseWheel>,
    pan: ResMut<'w, PanState>,
    control_mutations: ResMut<'w, CameraControlMutationFlags>,
    view_state: ResMut<'w, Map2dViewState>,
    cursor: Res<'w, CursorState>,
    ui_capture: Res<'w, UiPointerCapture>,
    view_mode: Res<'w, ViewModeState>,
    zoom_bounds: Res<'w, CameraZoomBounds>,
    camera_q: Query<
        'w,
        's,
        (
            &'static Camera,
            &'static mut Projection,
            &'static mut Transform,
        ),
        With<Map2dCamera>,
    >,
}

fn update_ui_pointer_capture(
    windows: Query<&Window, With<PrimaryWindow>>,
    blockers: Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            Option<&InheritedVisibility>,
        ),
        With<UiPointerBlocker>,
    >,
    mut capture: ResMut<UiPointerCapture>,
) {
    let Ok(window) = windows.single() else {
        capture.blocked = false;
        return;
    };
    let Some(cursor) = window.physical_cursor_position() else {
        capture.blocked = false;
        return;
    };
    capture.blocked = blockers.iter().any(|(node, transform, visibility)| {
        visibility.map(|v| v.get()).unwrap_or(true) && node.contains_point(*transform, cursor)
    });
}

fn screen_to_world_with_scale(
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

#[cfg(test)]
mod tests {
    use super::map2d_controls_should_run;
    use crate::map::camera::mode::ViewMode;

    #[test]
    fn map2d_controls_require_map_mode_and_active_camera() {
        assert!(map2d_controls_should_run(ViewMode::Map2D, true));
        assert!(!map2d_controls_should_run(ViewMode::Map2D, false));
        assert!(!map2d_controls_should_run(ViewMode::Terrain3D, true));
        assert!(!map2d_controls_should_run(ViewMode::Terrain3D, false));
    }
}
