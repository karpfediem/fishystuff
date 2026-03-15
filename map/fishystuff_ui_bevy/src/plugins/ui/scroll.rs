use super::*;
use bevy::ecs::system::SystemParam;

pub(super) type UiScrollTargetQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static RelativeCursorPosition,
        &'static Node,
        &'static ComputedNode,
        &'static UiGlobalTransform,
        Option<&'static InheritedVisibility>,
        Option<&'static ZoneEvidenceScroll>,
        Option<&'static FishAutocompleteScroll>,
        &'static mut ScrollPosition,
    ),
    Or<(
        With<ZoneEvidenceScroll>,
        With<FishAutocompleteScroll>,
        With<PatchDropdownList>,
    )>,
>;

pub(super) type AutocompleteScrollPositionQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut ScrollPosition,
        &'static ComputedNode,
        Option<&'static InheritedVisibility>,
    ),
    With<FishAutocompleteScroll>,
>;

pub(super) type AutocompleteTrackQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static ComputedNode,
        &'static UiGlobalTransform,
        Option<&'static InheritedVisibility>,
    ),
    With<FishAutocompleteScrollbarTrack>,
>;

pub(super) type AutocompleteThumbQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Node,
        &'static ComputedNode,
        &'static UiGlobalTransform,
        Option<&'static InheritedVisibility>,
    ),
    With<FishAutocompleteScrollbarThumb>,
>;

pub(super) fn handle_ui_scroll_wheel(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: UiScrollTargetQuery<'_, '_>,
) {
    let cursor = windows
        .single()
        .ok()
        .and_then(|window| window.physical_cursor_position());

    for mouse_wheel in mouse_wheel.read() {
        let mut delta = match mouse_wheel.unit {
            MouseScrollUnit::Line => -Vec2::new(mouse_wheel.x, mouse_wheel.y) * SCROLL_LINE_HEIGHT,
            MouseScrollUnit::Pixel => {
                -Vec2::new(mouse_wheel.x, mouse_wheel.y) * SCROLL_PIXEL_MULTIPLIER
            }
        };
        if keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
            std::mem::swap(&mut delta.x, &mut delta.y);
        }
        if delta == Vec2::ZERO {
            continue;
        }

        for (
            relative_cursor,
            node,
            computed,
            transform,
            visibility,
            evidence_scroll,
            autocomplete_scroll,
            mut scroll_position,
        ) in &mut query
        {
            if !visibility.map(|v| v.get()).unwrap_or(true) {
                continue;
            }
            let cursor_over = relative_cursor.cursor_over()
                || cursor
                    .map(|cursor_pos| computed.contains_point(*transform, cursor_pos))
                    .unwrap_or(false);
            if !cursor_over {
                continue;
            }
            apply_scroll_delta(
                &mut delta,
                node,
                computed,
                &mut scroll_position,
                evidence_scroll.is_some() || autocomplete_scroll.is_some(),
            );
            if delta == Vec2::ZERO {
                break;
            }
        }
    }
}

pub(super) fn apply_scroll_delta(
    delta: &mut Vec2,
    node: &Node,
    computed: &ComputedNode,
    scroll_position: &mut ScrollPosition,
    force_scroll_y: bool,
) {
    let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();

    if node.overflow.x == OverflowAxis::Scroll && delta.x != 0.0 {
        let max = max_offset.x.max(0.0);
        let prev = scroll_position.0.x;
        let primary = (prev + delta.x).clamp(0.0, max);
        if (primary - prev).abs() > f32::EPSILON {
            scroll_position.0.x = primary;
            delta.x = 0.0;
        }
    }

    if (force_scroll_y || node.overflow.y == OverflowAxis::Scroll) && delta.y != 0.0 {
        let max = max_offset.y.max(0.0);
        let prev = scroll_position.0.y;
        let primary = (prev + delta.y).clamp(0.0, max);
        if (primary - prev).abs() > f32::EPSILON {
            scroll_position.0.y = primary;
            delta.y = 0.0;
        }
    }
}

pub(super) fn evidence_scroll_metrics(
    scroll_y: f32,
    scroll_computed: &ComputedNode,
    viewport_h: f32,
    list_children: Option<&Children>,
) -> (usize, f32, f32, f32) {
    let inv = scroll_computed.inverse_scale_factor();
    let layout_max_offset =
        ((scroll_computed.content_size().y - scroll_computed.size().y) * inv).max(0.0);

    let row_count = list_children
        .map(|children| children.len().max(1))
        .unwrap_or(1usize);
    let row_step = EVIDENCE_ROW_HEIGHT_ESTIMATE + EVIDENCE_LIST_ROW_GAP;
    let estimated_content_h =
        row_count as f32 * row_step - EVIDENCE_LIST_ROW_GAP + EVIDENCE_SCROLL_PADDING_Y;
    let estimated_max_offset = (estimated_content_h - viewport_h).max(0.0);

    let mut max_offset = layout_max_offset.max(estimated_max_offset);
    if max_offset <= f32::EPSILON && scroll_y > f32::EPSILON {
        max_offset = scroll_y + row_step;
    }
    max_offset = max_offset.max(scroll_y.max(0.0));

    (
        row_count,
        layout_max_offset,
        estimated_max_offset,
        max_offset,
    )
}

pub(super) fn autocomplete_scroll_metrics(
    scroll_y: f32,
    scroll_computed: &ComputedNode,
    viewport_h: f32,
    visible_entries: usize,
) -> (usize, f32, f32, f32) {
    let inv = scroll_computed.inverse_scale_factor();
    let layout_max_offset =
        ((scroll_computed.content_size().y - scroll_computed.size().y) * inv).max(0.0);

    let row_count = visible_entries.max(1);
    let row_step = AUTOCOMPLETE_ROW_HEIGHT_ESTIMATE + AUTOCOMPLETE_LIST_ROW_GAP;
    let estimated_content_h =
        row_count as f32 * row_step - AUTOCOMPLETE_LIST_ROW_GAP + AUTOCOMPLETE_SCROLL_PADDING_Y;
    let estimated_max_offset = (estimated_content_h - viewport_h).max(0.0);

    let mut max_offset = layout_max_offset.max(estimated_max_offset);
    if max_offset <= f32::EPSILON && scroll_y > f32::EPSILON {
        max_offset = scroll_y + row_step;
    }
    max_offset = max_offset.max(scroll_y.max(0.0));

    (
        row_count,
        layout_max_offset,
        estimated_max_offset,
        max_offset,
    )
}

pub(super) fn patch_dropdown_scroll_metrics(
    scroll_y: f32,
    scroll_computed: &ComputedNode,
    viewport_h: f32,
    list_children: Option<&Children>,
) -> (usize, f32, f32, f32) {
    let inv = scroll_computed.inverse_scale_factor();
    let layout_max_offset =
        ((scroll_computed.content_size().y - scroll_computed.size().y) * inv).max(0.0);

    let row_count = list_children
        .map(|children| children.len().max(1))
        .unwrap_or(1usize);
    let row_step = PATCH_DROPDOWN_ROW_HEIGHT_ESTIMATE + PATCH_DROPDOWN_LIST_ROW_GAP;
    let estimated_content_h =
        row_count as f32 * row_step - PATCH_DROPDOWN_LIST_ROW_GAP + PATCH_DROPDOWN_SCROLL_PADDING_Y;
    let estimated_max_offset = (estimated_content_h - viewport_h).max(0.0);

    let mut max_offset = layout_max_offset.max(estimated_max_offset);
    if max_offset <= f32::EPSILON && scroll_y > f32::EPSILON {
        max_offset = scroll_y + row_step;
    }
    max_offset = max_offset.max(scroll_y.max(0.0));

    (
        row_count,
        layout_max_offset,
        estimated_max_offset,
        max_offset,
    )
}

pub(super) fn val_to_px(value: Val) -> Option<f32> {
    match value {
        Val::Px(px) => Some(px),
        _ => None,
    }
}

pub(super) fn point_icon_scale_to_ratio(scale: f32) -> f32 {
    let clamped = scale.clamp(POINT_ICON_SCALE_MIN, POINT_ICON_SCALE_MAX);
    let span = POINT_ICON_SCALE_MAX - POINT_ICON_SCALE_MIN;
    if span <= f32::EPSILON {
        return 1.0;
    }
    ((clamped - POINT_ICON_SCALE_MIN) / span).clamp(0.0, 1.0)
}

pub(super) fn point_icon_scale_from_ratio(ratio: f32) -> f32 {
    let span = POINT_ICON_SCALE_MAX - POINT_ICON_SCALE_MIN;
    (POINT_ICON_SCALE_MIN + ratio.clamp(0.0, 1.0) * span)
        .clamp(POINT_ICON_SCALE_MIN, POINT_ICON_SCALE_MAX)
}

pub(super) fn cursor_x_in_track(
    track_computed: &ComputedNode,
    track_transform: &UiGlobalTransform,
    cursor: Vec2,
    track_w: f32,
) -> Option<f32> {
    let (_, _, translation) = track_transform.to_scale_angle_translation();
    let track_w_physical = track_computed.size().x;
    if track_w_physical <= f32::EPSILON {
        return None;
    }
    let track_left_physical = translation.x - 0.5 * track_w_physical;
    let cursor_x_physical = (cursor.x - track_left_physical).clamp(0.0, track_w_physical);
    Some((cursor_x_physical * track_computed.inverse_scale_factor()).clamp(0.0, track_w))
}

pub(super) fn cursor_y_in_track(
    track_computed: &ComputedNode,
    track_transform: &UiGlobalTransform,
    cursor: Vec2,
    track_h: f32,
) -> Option<f32> {
    let (_, _, translation) = track_transform.to_scale_angle_translation();
    let track_h_physical = track_computed.size().y;
    if track_h_physical <= f32::EPSILON {
        return None;
    }
    let track_top_physical = translation.y - 0.5 * track_h_physical;
    let cursor_y_physical = (cursor.y - track_top_physical).clamp(0.0, track_h_physical);
    Some((cursor_y_physical * track_computed.inverse_scale_factor()).clamp(0.0, track_h))
}

pub(super) fn handle_zone_evidence_scrollbar_drag(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut drag_state: ResMut<ZoneEvidenceScrollbarDragState>,
    mut scroll_q: Query<
        (
            &mut ScrollPosition,
            &ComputedNode,
            Option<&InheritedVisibility>,
        ),
        With<ZoneEvidenceScroll>,
    >,
    list_q: Query<(Option<&Children>, Option<&InheritedVisibility>), With<ZoneEvidenceList>>,
    track_q: Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            Option<&InheritedVisibility>,
        ),
        With<ZoneEvidenceScrollbarTrack>,
    >,
    thumb_q: Query<
        (
            &Node,
            &ComputedNode,
            &UiGlobalTransform,
            Option<&InheritedVisibility>,
        ),
        With<ZoneEvidenceScrollbarThumb>,
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

    let Ok((mut scroll_position, scroll_computed, scroll_visibility)) = scroll_q.single_mut()
    else {
        return;
    };
    let Ok((list_children, list_visibility)) = list_q.single() else {
        return;
    };
    let Ok((track_computed, track_transform, track_visibility)) = track_q.single() else {
        return;
    };
    let Ok((thumb_node, thumb_computed, thumb_transform, thumb_visibility)) = thumb_q.single()
    else {
        return;
    };

    if !scroll_visibility.map(|v| v.get()).unwrap_or(true)
        || !list_visibility.map(|v| v.get()).unwrap_or(true)
        || !track_visibility.map(|v| v.get()).unwrap_or(true)
        || !thumb_visibility.map(|v| v.get()).unwrap_or(true)
    {
        drag_state.active = false;
        return;
    }

    let inv = scroll_computed.inverse_scale_factor();
    let viewport_h = scroll_computed.size().y * inv;
    let track_h = track_computed.size().y * track_computed.inverse_scale_factor();
    if viewport_h <= f32::EPSILON || track_h <= f32::EPSILON {
        drag_state.active = false;
        return;
    }

    let (_, _, _, max_offset) = evidence_scroll_metrics(
        scroll_position.0.y,
        scroll_computed,
        viewport_h,
        list_children,
    );
    if max_offset <= f32::EPSILON {
        drag_state.active = false;
        return;
    }

    let mut thumb_h = val_to_px(thumb_node.height).unwrap_or(EVIDENCE_SCROLLBAR_MIN_THUMB);
    thumb_h = thumb_h.clamp(EVIDENCE_SCROLLBAR_MIN_THUMB, track_h);
    let mut thumb_top = val_to_px(thumb_node.top).unwrap_or(0.0);
    let travel_h = (track_h - thumb_h).max(0.0);
    thumb_top = thumb_top.clamp(0.0, travel_h);
    if travel_h <= f32::EPSILON {
        drag_state.active = false;
        return;
    }

    let Some(cursor_track_y) = cursor_y_in_track(track_computed, track_transform, cursor, track_h)
    else {
        drag_state.active = false;
        return;
    };

    if mouse_buttons.just_pressed(MouseButton::Left) {
        if thumb_computed.contains_point(*thumb_transform, cursor) {
            drag_state.active = true;
            drag_state.grab_offset_px = (cursor_track_y - thumb_top).clamp(0.0, thumb_h);
        } else if track_computed.contains_point(*track_transform, cursor) {
            drag_state.active = true;
            drag_state.grab_offset_px = 0.5 * thumb_h;
        } else {
            drag_state.active = false;
            return;
        }
    }

    if !drag_state.active {
        return;
    }

    let desired_thumb_top = (cursor_track_y - drag_state.grab_offset_px).clamp(0.0, travel_h);
    let ratio = (desired_thumb_top / travel_h).clamp(0.0, 1.0);
    scroll_position.0.y = (ratio * max_offset).clamp(0.0, max_offset);
}

pub(super) fn handle_autocomplete_scrollbar_drag(
    mut drag: AutocompleteScrollbarDragContext<'_, '_>,
) {
    if drag.mouse_buttons.just_released(MouseButton::Left) {
        drag.drag_state.active = false;
    }
    if !drag.search.open || drag.search.results.is_empty() {
        drag.drag_state.active = false;
        return;
    }
    if !drag.mouse_buttons.pressed(MouseButton::Left)
        && !drag.mouse_buttons.just_pressed(MouseButton::Left)
    {
        return;
    }

    let Some(cursor) = drag
        .windows
        .single()
        .ok()
        .and_then(|window| window.physical_cursor_position())
    else {
        return;
    };

    let Ok((mut scroll_position, scroll_computed, scroll_visibility)) = drag.scroll_q.single_mut()
    else {
        return;
    };
    let Ok(frame_visibility) = drag.frame_q.single() else {
        return;
    };
    let Ok((track_computed, track_transform, track_visibility)) = drag.track_q.single() else {
        return;
    };
    let Ok((thumb_node, thumb_computed, thumb_transform, thumb_visibility)) = drag.thumb_q.single()
    else {
        return;
    };

    if !scroll_visibility.map(|v| v.get()).unwrap_or(true)
        || !frame_visibility.map(|v| v.get()).unwrap_or(true)
        || !track_visibility.map(|v| v.get()).unwrap_or(true)
        || !thumb_visibility.map(|v| v.get()).unwrap_or(true)
    {
        drag.drag_state.active = false;
        return;
    }

    let inv = scroll_computed.inverse_scale_factor();
    let viewport_h = scroll_computed.size().y * inv;
    let track_h = track_computed.size().y * track_computed.inverse_scale_factor();
    if viewport_h <= f32::EPSILON || track_h <= f32::EPSILON {
        drag.drag_state.active = false;
        return;
    }

    let (_, _, _, max_offset) = autocomplete_scroll_metrics(
        scroll_position.0.y,
        scroll_computed,
        viewport_h,
        drag.search.results.len(),
    );
    if max_offset <= f32::EPSILON {
        drag.drag_state.active = false;
        return;
    }

    let mut thumb_h = val_to_px(thumb_node.height).unwrap_or(AUTOCOMPLETE_SCROLLBAR_MIN_THUMB);
    thumb_h = thumb_h.clamp(AUTOCOMPLETE_SCROLLBAR_MIN_THUMB, track_h);
    let mut thumb_top = val_to_px(thumb_node.top).unwrap_or(0.0);
    let travel_h = (track_h - thumb_h).max(0.0);
    thumb_top = thumb_top.clamp(0.0, travel_h);
    if travel_h <= f32::EPSILON {
        drag.drag_state.active = false;
        return;
    }

    let Some(cursor_track_y) = cursor_y_in_track(track_computed, track_transform, cursor, track_h)
    else {
        drag.drag_state.active = false;
        return;
    };

    if drag.mouse_buttons.just_pressed(MouseButton::Left) {
        if thumb_computed.contains_point(*thumb_transform, cursor) {
            drag.drag_state.active = true;
            drag.drag_state.grab_offset_px = (cursor_track_y - thumb_top).clamp(0.0, thumb_h);
        } else if track_computed.contains_point(*track_transform, cursor) {
            drag.drag_state.active = true;
            drag.drag_state.grab_offset_px = 0.5 * thumb_h;
        } else {
            drag.drag_state.active = false;
            return;
        }
    }

    if !drag.drag_state.active {
        return;
    }

    let desired_thumb_top = (cursor_track_y - drag.drag_state.grab_offset_px).clamp(0.0, travel_h);
    let ratio = (desired_thumb_top / travel_h).clamp(0.0, 1.0);
    scroll_position.0.y = (ratio * max_offset).clamp(0.0, max_offset);
}

#[derive(SystemParam)]
pub(super) struct AutocompleteScrollbarDragContext<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    search: Res<'w, SearchState>,
    drag_state: ResMut<'w, FishAutocompleteScrollbarDragState>,
    scroll_q: AutocompleteScrollPositionQuery<'w, 's>,
    frame_q: Query<'w, 's, Option<&'static InheritedVisibility>, With<FishAutocompleteFrame>>,
    track_q: AutocompleteTrackQuery<'w, 's>,
    thumb_q: AutocompleteThumbQuery<'w, 's>,
}

pub(super) fn sync_autocomplete_scrollbar(
    search: Res<SearchState>,
    scroll_q: Query<
        (&ScrollPosition, &ComputedNode, Option<&InheritedVisibility>),
        With<FishAutocompleteScroll>,
    >,
    frame_q: Query<Option<&InheritedVisibility>, With<FishAutocompleteFrame>>,
    track_q: Query<
        (&ComputedNode, Option<&InheritedVisibility>),
        With<FishAutocompleteScrollbarTrack>,
    >,
    mut thumb_q: Query<(&mut Node, &mut Visibility), With<FishAutocompleteScrollbarThumb>>,
) {
    let Ok((mut thumb_node, mut thumb_visibility)) = thumb_q.single_mut() else {
        return;
    };
    if !search.open || search.results.is_empty() {
        *thumb_visibility = Visibility::Hidden;
        return;
    }

    let Ok((scroll, scroll_computed, scroll_visibility)) = scroll_q.single() else {
        *thumb_visibility = Visibility::Hidden;
        return;
    };
    let Ok(frame_visibility) = frame_q.single() else {
        *thumb_visibility = Visibility::Hidden;
        return;
    };
    let Ok((track_computed, track_visibility)) = track_q.single() else {
        *thumb_visibility = Visibility::Hidden;
        return;
    };

    let scroll_visible = scroll_visibility.map(|v| v.get()).unwrap_or(true);
    let frame_visible = frame_visibility.map(|v| v.get()).unwrap_or(true);
    let track_visible = track_visibility.map(|v| v.get()).unwrap_or(true);
    if !scroll_visible || !frame_visible || !track_visible {
        *thumb_visibility = Visibility::Hidden;
        return;
    }

    let inv = scroll_computed.inverse_scale_factor();
    let viewport_h = scroll_computed.size().y * inv;
    if viewport_h <= f32::EPSILON {
        *thumb_visibility = Visibility::Hidden;
        return;
    }

    let track_h = track_computed.size().y * track_computed.inverse_scale_factor();
    if track_h <= f32::EPSILON {
        *thumb_visibility = Visibility::Hidden;
        return;
    }

    let (_, _, _, max_offset) = autocomplete_scroll_metrics(
        scroll.0.y,
        scroll_computed,
        viewport_h,
        search.results.len(),
    );
    if max_offset <= f32::EPSILON {
        thumb_node.top = Val::Px(0.0);
        thumb_node.height = Val::Px(track_h);
        *thumb_visibility = Visibility::Visible;
        return;
    }

    let content_h = viewport_h + max_offset;
    let ratio = (viewport_h / content_h).clamp(0.0, 1.0);
    let thumb_h = (track_h * ratio).clamp(AUTOCOMPLETE_SCROLLBAR_MIN_THUMB, track_h);
    let travel_h = (track_h - thumb_h).max(0.0);
    let offset_ratio = (scroll.0.y.clamp(0.0, max_offset) / max_offset).clamp(0.0, 1.0);
    let thumb_top = travel_h * offset_ratio;

    thumb_node.top = Val::Px(thumb_top);
    thumb_node.height = Val::Px(thumb_h);
    *thumb_visibility = Visibility::Visible;
}

pub(super) fn sync_zone_evidence_scrollbar(
    scroll_q: Query<
        (&ScrollPosition, &ComputedNode, Option<&InheritedVisibility>),
        With<ZoneEvidenceScroll>,
    >,
    list_q: Query<(Option<&Children>, Option<&InheritedVisibility>), With<ZoneEvidenceList>>,
    track_q: Query<(&ComputedNode, Option<&InheritedVisibility>), With<ZoneEvidenceScrollbarTrack>>,
    mut thumb_q: Query<(&mut Node, &mut Visibility), With<ZoneEvidenceScrollbarThumb>>,
) {
    let Ok((scroll, scroll_computed, scroll_visibility)) = scroll_q.single() else {
        return;
    };
    let Ok((list_children, list_visibility)) = list_q.single() else {
        return;
    };
    let Ok((track_computed, track_visibility)) = track_q.single() else {
        return;
    };
    let Ok((mut thumb_node, mut thumb_visibility)) = thumb_q.single_mut() else {
        return;
    };

    let scroll_visible = scroll_visibility.map(|v| v.get()).unwrap_or(true);
    let list_visible = list_visibility.map(|v| v.get()).unwrap_or(true);
    let track_visible = track_visibility.map(|v| v.get()).unwrap_or(true);
    if !scroll_visible || !list_visible || !track_visible {
        *thumb_visibility = Visibility::Hidden;
        return;
    }

    let inv = scroll_computed.inverse_scale_factor();
    let viewport_h = scroll_computed.size().y * inv;
    if viewport_h <= f32::EPSILON {
        *thumb_visibility = Visibility::Hidden;
        return;
    }

    let track_h = track_computed.size().y * track_computed.inverse_scale_factor();
    if track_h <= f32::EPSILON {
        *thumb_visibility = Visibility::Hidden;
        return;
    }

    let (_, _, _, max_offset) =
        evidence_scroll_metrics(scroll.0.y, scroll_computed, viewport_h, list_children);
    if max_offset <= f32::EPSILON {
        thumb_node.top = Val::Px(0.0);
        thumb_node.height = Val::Px(track_h);
        *thumb_visibility = Visibility::Visible;
        return;
    }

    let content_h = viewport_h + max_offset;
    let ratio = (viewport_h / content_h).clamp(0.0, 1.0);
    let thumb_h = (track_h * ratio).clamp(EVIDENCE_SCROLLBAR_MIN_THUMB, track_h);
    let travel_h = (track_h - thumb_h).max(0.0);
    let offset_ratio = (scroll.0.y.clamp(0.0, max_offset) / max_offset).clamp(0.0, 1.0);
    let thumb_top = travel_h * offset_ratio;

    thumb_node.top = Val::Px(thumb_top);
    thumb_node.height = Val::Px(thumb_h);
    *thumb_visibility = Visibility::Visible;
}
