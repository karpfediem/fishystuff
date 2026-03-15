use super::*;
use bevy::ecs::system::SystemParam;

#[derive(Resource, Default)]
pub(in crate::map::terrain::runtime) struct OrbitInputState {
    dragging: bool,
    drag_mode: OrbitDragMode,
    last_cursor: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum OrbitDragMode {
    #[default]
    Orbit,
    Pan,
    Dolly,
}

pub(in crate::map::terrain::runtime) fn update_terrain3d_camera_controls(
    mut controls: Terrain3dCameraControls<'_, '_>,
) {
    crate::perf_scope!("camera.3d_update");
    if controls.mode.mode != ViewMode::Terrain3D {
        controls.orbit_input.dragging = false;
        controls.orbit_input.drag_mode = OrbitDragMode::Orbit;
        return;
    }

    let Ok(window) = controls.windows.single() else {
        return;
    };
    let Ok((camera, mut camera_transform, mut projection)) = controls.camera_q.single_mut() else {
        return;
    };
    if !terrain3d_controls_should_run(controls.mode.mode, camera.is_active) {
        controls.orbit_input.dragging = false;
        controls.orbit_input.drag_mode = OrbitDragMode::Orbit;
        return;
    }
    ensure_terrain3d_projection(&mut projection);

    let shift = controls.key_buttons.pressed(KeyCode::ShiftLeft)
        || controls.key_buttons.pressed(KeyCode::ShiftRight);
    let ctrl = controls.key_buttons.pressed(KeyCode::ControlLeft)
        || controls.key_buttons.pressed(KeyCode::ControlRight);
    let alt = controls.key_buttons.pressed(KeyCode::AltLeft)
        || controls.key_buttons.pressed(KeyCode::AltRight);

    let use_primary_fallback = alt && controls.mouse_buttons.pressed(MouseButton::Left);
    let use_middle = controls.mouse_buttons.pressed(MouseButton::Middle);
    let drag_active = use_middle || use_primary_fallback;
    let drag_mode = if ctrl {
        OrbitDragMode::Dolly
    } else if shift {
        OrbitDragMode::Pan
    } else {
        OrbitDragMode::Orbit
    };
    let ui_input_blocked = controls.ui_capture.blocked || controls.ui_capture.text_input_active;
    let mut state_changed = false;

    let cursor = window.cursor_position().or(controls.cursor_state.last_pos);

    if !drag_active || ui_input_blocked {
        controls.orbit_input.dragging = false;
    } else if let Some(cursor) = cursor {
        if !controls.orbit_input.dragging {
            controls.orbit_input.dragging = true;
            controls.orbit_input.last_cursor = cursor;
            controls.orbit_input.drag_mode = drag_mode;
        } else {
            let mut delta = cursor - controls.orbit_input.last_cursor;
            if camera_controls_x_mirrored() {
                delta.x = -delta.x;
            }
            controls.orbit_input.last_cursor = cursor;
            let fov_y = match &*projection {
                Projection::Perspective(p) => p.fov,
                _ => 55.0_f32.to_radians(),
            };
            controls.orbit_input.drag_mode = drag_mode;
            match controls.orbit_input.drag_mode {
                OrbitDragMode::Orbit => {
                    controls.view.orbit(delta, ORBIT_SENSITIVITY);
                    state_changed = true;
                }
                OrbitDragMode::Pan => {
                    controls
                        .view
                        .pan(delta, Vec2::new(window.width(), window.height()), fov_y);
                    state_changed = true;
                }
                OrbitDragMode::Dolly => {
                    controls
                        .view
                        .dolly(delta.y * DOLLY_DIRECTION, DOLLY_DRAG_SPEED);
                    state_changed = true;
                }
            }
        }
    }

    for ev in controls.mouse_wheel.read() {
        if ui_input_blocked {
            continue;
        }
        let mut scroll = ev.y;
        if matches!(ev.unit, MouseScrollUnit::Pixel) {
            scroll /= 90.0;
        }
        controls
            .view
            .dolly(scroll * DOLLY_DIRECTION, DOLLY_WHEEL_SPEED);
        state_changed = true;
    }

    if !controls.ui_capture.text_input_active
        && (controls.key_buttons.just_pressed(KeyCode::Home)
            || (shift && controls.key_buttons.just_pressed(KeyCode::KeyC)))
    {
        reset_terrain3d_view(&mut controls.view);
        state_changed = true;
    }

    if state_changed {
        controls.control_mutations.terrain3d_updated = true;
    }
    apply_terrain3d_camera_state(&controls.view, &mut camera_transform);
}

#[derive(SystemParam)]
pub(in crate::map::terrain::runtime) struct Terrain3dCameraControls<'w, 's> {
    mode: Res<'w, ViewModeState>,
    view: ResMut<'w, Terrain3dViewState>,
    orbit_input: ResMut<'w, OrbitInputState>,
    control_mutations: ResMut<'w, CameraControlMutationFlags>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    cursor_state: Res<'w, CursorState>,
    ui_capture: Res<'w, UiPointerCapture>,
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    key_buttons: Res<'w, ButtonInput<KeyCode>>,
    mouse_wheel: MessageReader<'w, 's, MouseWheel>,
    camera_q: Query<
        'w,
        's,
        (
            &'static Camera,
            &'static mut Transform,
            &'static mut Projection,
        ),
        With<Terrain3dCamera>,
    >,
}
