use super::super::scroll::{
    cursor_x_in_track, point_icon_scale_from_ratio, point_icon_scale_to_ratio, val_to_px,
};
use super::super::*;

pub(in crate::plugins::ui) fn handle_point_icon_size_slider_drag(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut display_state: ResMut<MapDisplayState>,
    mut drag_state: ResMut<PointIconSizeSliderDragState>,
    track_q: Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            Option<&InheritedVisibility>,
        ),
        With<PointIconSizeSliderTrack>,
    >,
    thumb_q: Query<
        (
            &Node,
            &ComputedNode,
            &UiGlobalTransform,
            Option<&InheritedVisibility>,
        ),
        With<PointIconSizeSliderThumb>,
    >,
) {
    if mouse_buttons.just_released(MouseButton::Left) {
        drag_state.active = false;
    }
    if !mouse_buttons.pressed(MouseButton::Left) && !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Some(cursor) = windows
        .single()
        .ok()
        .and_then(|window| window.physical_cursor_position())
    else {
        return;
    };

    let Ok((track_computed, track_transform, track_visibility)) = track_q.single() else {
        drag_state.active = false;
        return;
    };
    let Ok((thumb_node, thumb_computed, thumb_transform, thumb_visibility)) = thumb_q.single()
    else {
        drag_state.active = false;
        return;
    };

    if !track_visibility.map(|v| v.get()).unwrap_or(true)
        || !thumb_visibility.map(|v| v.get()).unwrap_or(true)
    {
        drag_state.active = false;
        return;
    }

    let track_w = track_computed.size().x * track_computed.inverse_scale_factor();
    if track_w <= f32::EPSILON {
        drag_state.active = false;
        return;
    }

    let thumb_w = val_to_px(thumb_node.width)
        .unwrap_or(POINT_ICON_SIZE_SLIDER_MIN_THUMB)
        .clamp(POINT_ICON_SIZE_SLIDER_MIN_THUMB, track_w);
    let travel_w = (track_w - thumb_w).max(0.0);
    if travel_w <= f32::EPSILON {
        display_state.point_icon_scale = POINT_ICON_SCALE_MAX;
        drag_state.active = false;
        return;
    }

    let mut thumb_left = val_to_px(thumb_node.left).unwrap_or(0.0);
    thumb_left = thumb_left.clamp(0.0, travel_w);

    let Some(cursor_track_x) = cursor_x_in_track(track_computed, track_transform, cursor, track_w)
    else {
        drag_state.active = false;
        return;
    };

    if mouse_buttons.just_pressed(MouseButton::Left) {
        if thumb_computed.contains_point(*thumb_transform, cursor) {
            drag_state.active = true;
            drag_state.grab_offset_px = (cursor_track_x - thumb_left).clamp(0.0, thumb_w);
        } else if track_computed.contains_point(*track_transform, cursor) {
            drag_state.active = true;
            drag_state.grab_offset_px = 0.5 * thumb_w;
        } else {
            drag_state.active = false;
            return;
        }
    }

    if !drag_state.active {
        return;
    }

    let desired_thumb_left = (cursor_track_x - drag_state.grab_offset_px).clamp(0.0, travel_w);
    let ratio = (desired_thumb_left / travel_w).clamp(0.0, 1.0);
    display_state.point_icon_scale = point_icon_scale_from_ratio(ratio);
}

pub(in crate::plugins::ui) fn sync_point_icon_size_slider(
    display_state: Res<MapDisplayState>,
    track_q: Query<(&ComputedNode, Option<&InheritedVisibility>), With<PointIconSizeSliderTrack>>,
    mut thumb_q: Query<(&mut Node, &mut Visibility), With<PointIconSizeSliderThumb>>,
) {
    let Ok((track_computed, track_visibility)) = track_q.single() else {
        return;
    };
    let Ok((mut thumb_node, mut thumb_visibility)) = thumb_q.single_mut() else {
        return;
    };

    if !track_visibility.map(|v| v.get()).unwrap_or(true) {
        *thumb_visibility = Visibility::Hidden;
        return;
    }

    let track_w = track_computed.size().x * track_computed.inverse_scale_factor();
    if track_w <= f32::EPSILON {
        *thumb_visibility = Visibility::Hidden;
        return;
    }

    let thumb_w = val_to_px(thumb_node.width)
        .unwrap_or(POINT_ICON_SIZE_SLIDER_MIN_THUMB)
        .clamp(POINT_ICON_SIZE_SLIDER_MIN_THUMB, track_w);
    let travel_w = (track_w - thumb_w).max(0.0);
    let ratio = point_icon_scale_to_ratio(display_state.point_icon_scale);
    let thumb_left = if travel_w <= f32::EPSILON {
        0.0
    } else {
        travel_w * ratio
    };

    thumb_node.left = Val::Px(thumb_left.clamp(0.0, travel_w));
    *thumb_visibility = Visibility::Visible;
}

pub(in crate::plugins::ui) fn sync_point_icon_size_text(
    display_state: Res<MapDisplayState>,
    mut text_q: Query<&mut Text, With<PointIconSizeValueText>>,
) {
    if !display_state.is_changed() {
        return;
    }
    let Ok(mut text) = text_q.single_mut() else {
        return;
    };
    let percent = (display_state
        .point_icon_scale
        .clamp(POINT_ICON_SCALE_MIN, POINT_ICON_SCALE_MAX)
        * 100.0)
        .round() as i32;
    text.0 = format!("{percent}%");
}
