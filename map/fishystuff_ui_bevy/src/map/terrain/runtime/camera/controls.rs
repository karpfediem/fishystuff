use super::*;

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
    mode: Res<ViewModeState>,
    mut view: ResMut<Terrain3dViewState>,
    mut orbit_input: ResMut<OrbitInputState>,
    mut control_mutations: ResMut<CameraControlMutationFlags>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cursor_state: Res<CursorState>,
    ui_capture: Res<UiPointerCapture>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    key_buttons: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mut camera_q: Query<(&Camera, &mut Transform, &mut Projection), With<Terrain3dCamera>>,
) {
    crate::perf_scope!("camera.3d_update");
    if mode.mode != ViewMode::Terrain3D {
        orbit_input.dragging = false;
        orbit_input.drag_mode = OrbitDragMode::Orbit;
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, mut camera_transform, mut projection)) = camera_q.single_mut() else {
        return;
    };
    if !terrain3d_controls_should_run(mode.mode, camera.is_active) {
        orbit_input.dragging = false;
        orbit_input.drag_mode = OrbitDragMode::Orbit;
        return;
    }
    ensure_terrain3d_projection(&mut projection);

    let shift = key_buttons.pressed(KeyCode::ShiftLeft) || key_buttons.pressed(KeyCode::ShiftRight);
    let ctrl =
        key_buttons.pressed(KeyCode::ControlLeft) || key_buttons.pressed(KeyCode::ControlRight);
    let alt = key_buttons.pressed(KeyCode::AltLeft) || key_buttons.pressed(KeyCode::AltRight);

    let use_primary_fallback = alt && mouse_buttons.pressed(MouseButton::Left);
    let use_middle = mouse_buttons.pressed(MouseButton::Middle);
    let drag_active = use_middle || use_primary_fallback;
    let drag_mode = if ctrl {
        OrbitDragMode::Dolly
    } else if shift {
        OrbitDragMode::Pan
    } else {
        OrbitDragMode::Orbit
    };
    let ui_input_blocked = ui_capture.blocked || ui_capture.text_input_active;
    let mut state_changed = false;

    let cursor = window.cursor_position().or(cursor_state.last_pos);

    if !drag_active || ui_input_blocked {
        orbit_input.dragging = false;
    } else if let Some(cursor) = cursor {
        if !orbit_input.dragging {
            orbit_input.dragging = true;
            orbit_input.last_cursor = cursor;
            orbit_input.drag_mode = drag_mode;
        } else {
            let mut delta = cursor - orbit_input.last_cursor;
            if camera_controls_x_mirrored() {
                delta.x = -delta.x;
            }
            orbit_input.last_cursor = cursor;
            let fov_y = match &*projection {
                Projection::Perspective(p) => p.fov,
                _ => 55.0_f32.to_radians(),
            };
            orbit_input.drag_mode = drag_mode;
            match orbit_input.drag_mode {
                OrbitDragMode::Orbit => {
                    view.orbit(delta, ORBIT_SENSITIVITY);
                    state_changed = true;
                }
                OrbitDragMode::Pan => {
                    view.pan(delta, Vec2::new(window.width(), window.height()), fov_y);
                    state_changed = true;
                }
                OrbitDragMode::Dolly => {
                    view.dolly(delta.y * DOLLY_DIRECTION, DOLLY_DRAG_SPEED);
                    state_changed = true;
                }
            }
        }
    }

    for ev in mouse_wheel.read() {
        if ui_input_blocked {
            continue;
        }
        let mut scroll = ev.y;
        if matches!(ev.unit, MouseScrollUnit::Pixel) {
            scroll /= 90.0;
        }
        view.dolly(scroll * DOLLY_DIRECTION, DOLLY_WHEEL_SPEED);
        state_changed = true;
    }

    if !ui_capture.text_input_active
        && (key_buttons.just_pressed(KeyCode::Home)
            || (shift && key_buttons.just_pressed(KeyCode::KeyC)))
    {
        reset_terrain3d_view(&mut view);
        state_changed = true;
    }

    if state_changed {
        control_mutations.terrain3d_updated = true;
    }
    apply_terrain3d_camera_state(&view, &mut camera_transform);
}
