use super::super::scroll::{cursor_y_in_track, patch_dropdown_scroll_metrics, val_to_px};
use super::super::setup::text_style;
use super::super::*;
use super::selection::{display_patches, normalize_patch_selection, patch_list_hash, patch_name};

pub(in crate::plugins::ui) fn handle_patch_dropdown_toggle(
    mut state: ResMut<PatchDropdownState>,
    mut query: Query<(&Interaction, &PatchRangeButton), Changed<Interaction>>,
) {
    for (interaction, button) in &mut query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        state.open = if state.open == Some(button.bound) {
            None
        } else {
            Some(button.bound)
        };
    }
}

pub(in crate::plugins::ui) fn sync_patch_dropdown_visibility(
    state: Res<PatchDropdownState>,
    mut q: Query<
        (
            Option<&PatchDropdownList>,
            Option<&PatchDropdownScrollbarTrack>,
            &mut Visibility,
            &mut Node,
        ),
        Or<(With<PatchDropdownList>, With<PatchDropdownScrollbarTrack>)>,
    >,
) {
    if !state.is_changed() {
        return;
    }
    for (list, track, mut vis, mut node) in &mut q {
        let bound = list
            .map(|l| l.bound)
            .or_else(|| track.map(|t| t.bound))
            .expect("patch dropdown query must have bound marker");
        let open = state.open == Some(bound);
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let height = if open {
            PATCH_DROPDOWN_OPEN_HEIGHT
        } else {
            0.0
        };
        node.height = Val::Px(height);
        node.min_height = Val::Px(height);
        node.max_height = Val::Px(height);
    }
}

pub(in crate::plugins::ui) fn sync_patch_list(
    patch_filter: Res<PatchFilterState>,
    mut state: ResMut<PatchDropdownState>,
    fonts: Res<UiFonts>,
    mut commands: Commands,
    list_q: Query<(Entity, &PatchDropdownList)>,
    children_q: Query<&Children>,
) {
    let patches = patch_filter.patches.as_slice();
    let hash = patch_list_hash(patches);
    if hash == state.last_hash {
        return;
    }
    state.last_hash = hash;
    let display = display_patches(patches);
    for (list_entity, list_cfg) in &list_q {
        if let Ok(children) = children_q.get(list_entity) {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }
        commands.entity(list_entity).with_children(|list| {
            if display.is_empty() {
                list.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(26.0),
                        max_height: Val::Px(26.0),
                        padding: UiRect::horizontal(Val::Px(6.0)),
                        align_items: AlignItems::Center,
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.001)),
                    ClassList::new("list-item"),
                ))
                .with_children(|entry| {
                    entry.spawn((
                        UiTextBundle::new(
                            "(no patches loaded)",
                            &text_style(12.0, Color::srgb(0.72, 0.72, 0.78), fonts.regular.clone()),
                        ),
                        ClassList::new("fish-name"),
                    ));
                });
            } else {
                for patch in &display {
                    let name = patch_name(patch);
                    list.spawn((
                        PatchEntry {
                            bound: list_cfg.bound,
                            patch_id: patch.patch_id.0.clone(),
                        },
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            min_height: Val::Px(26.0),
                            max_height: Val::Px(26.0),
                            padding: UiRect::horizontal(Val::Px(6.0)),
                            align_items: AlignItems::Center,
                            flex_shrink: 0.0,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.001)),
                        ClassList::new("list-item"),
                    ))
                    .with_children(|entry| {
                        entry.spawn((
                            UiTextBundle::new(
                                name,
                                &text_style(
                                    12.0,
                                    Color::srgb(0.9, 0.9, 0.92),
                                    fonts.regular.clone(),
                                ),
                            ),
                            ClassList::new("fish-name"),
                        ));
                    });
                }
            }
        });
    }
}

pub(in crate::plugins::ui) fn handle_patch_dropdown_scrollbar_drag(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    state: Res<PatchDropdownState>,
    mut drag_state: ResMut<PatchDropdownScrollbarDragState>,
    mut list_q: Query<
        (
            &PatchDropdownList,
            &mut ScrollPosition,
            &ComputedNode,
            Option<&Children>,
            Option<&InheritedVisibility>,
        ),
        With<PatchDropdownList>,
    >,
    track_q: Query<
        (
            &PatchDropdownScrollbarTrack,
            &ComputedNode,
            &UiGlobalTransform,
            Option<&InheritedVisibility>,
        ),
        With<PatchDropdownScrollbarTrack>,
    >,
    thumb_q: Query<
        (
            &PatchDropdownScrollbarThumb,
            &Node,
            &ComputedNode,
            &UiGlobalTransform,
            Option<&InheritedVisibility>,
        ),
        With<PatchDropdownScrollbarThumb>,
    >,
) {
    if mouse_buttons.just_released(MouseButton::Left) {
        drag_state.active_bound = None;
    }
    let Some(open_bound) = state.open else {
        drag_state.active_bound = None;
        return;
    };
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

    let mut list_data = None;
    for (cfg, scroll, computed, children, visibility) in &mut list_q {
        if cfg.bound == open_bound {
            list_data = Some((scroll, computed, children, visibility));
            break;
        }
    }
    let Some((mut scroll_position, scroll_computed, list_children, list_visibility)) = list_data
    else {
        drag_state.active_bound = None;
        return;
    };

    let Some((track_computed, track_transform, track_visibility)) =
        track_q
            .iter()
            .find_map(|(cfg, computed, transform, visibility)| {
                (cfg.bound == open_bound).then_some((computed, transform, visibility))
            })
    else {
        drag_state.active_bound = None;
        return;
    };

    let Some((thumb_node, thumb_computed, thumb_transform, thumb_visibility)) = thumb_q
        .iter()
        .find_map(|(cfg, node, computed, transform, visibility)| {
            (cfg.bound == open_bound).then_some((node, computed, transform, visibility))
        })
    else {
        drag_state.active_bound = None;
        return;
    };

    if !list_visibility.map(|v| v.get()).unwrap_or(true)
        || !track_visibility.map(|v| v.get()).unwrap_or(true)
        || !thumb_visibility.map(|v| v.get()).unwrap_or(true)
    {
        drag_state.active_bound = None;
        return;
    }

    let inv = scroll_computed.inverse_scale_factor();
    let viewport_h = scroll_computed.size().y * inv;
    let track_h = track_computed.size().y * track_computed.inverse_scale_factor();
    if viewport_h <= f32::EPSILON || track_h <= f32::EPSILON {
        drag_state.active_bound = None;
        return;
    }

    let (_, _, _, max_offset) = patch_dropdown_scroll_metrics(
        scroll_position.0.y,
        scroll_computed,
        viewport_h,
        list_children,
    );
    if max_offset <= f32::EPSILON {
        drag_state.active_bound = None;
        return;
    }

    let mut thumb_h = val_to_px(thumb_node.height).unwrap_or(PATCH_DROPDOWN_SCROLLBAR_MIN_THUMB);
    thumb_h = thumb_h.clamp(PATCH_DROPDOWN_SCROLLBAR_MIN_THUMB, track_h);
    let mut thumb_top = val_to_px(thumb_node.top).unwrap_or(0.0);
    let travel_h = (track_h - thumb_h).max(0.0);
    thumb_top = thumb_top.clamp(0.0, travel_h);
    if travel_h <= f32::EPSILON {
        drag_state.active_bound = None;
        return;
    }

    let Some(cursor_track_y) = cursor_y_in_track(track_computed, track_transform, cursor, track_h)
    else {
        drag_state.active_bound = None;
        return;
    };

    if mouse_buttons.just_pressed(MouseButton::Left) {
        if thumb_computed.contains_point(*thumb_transform, cursor) {
            drag_state.active_bound = Some(open_bound);
            drag_state.grab_offset_px = (cursor_track_y - thumb_top).clamp(0.0, thumb_h);
        } else if track_computed.contains_point(*track_transform, cursor) {
            drag_state.active_bound = Some(open_bound);
            drag_state.grab_offset_px = 0.5 * thumb_h;
        } else {
            drag_state.active_bound = None;
            return;
        }
    }

    if drag_state.active_bound != Some(open_bound) {
        return;
    }

    let desired_thumb_top = (cursor_track_y - drag_state.grab_offset_px).clamp(0.0, travel_h);
    let ratio = (desired_thumb_top / travel_h).clamp(0.0, 1.0);
    scroll_position.0.y = (ratio * max_offset).clamp(0.0, max_offset);
}

pub(in crate::plugins::ui) fn sync_patch_dropdown_scrollbar(
    state: Res<PatchDropdownState>,
    list_q: Query<
        (
            &PatchDropdownList,
            &ScrollPosition,
            &ComputedNode,
            Option<&Children>,
            Option<&InheritedVisibility>,
        ),
        With<PatchDropdownList>,
    >,
    track_q: Query<
        (
            &PatchDropdownScrollbarTrack,
            &ComputedNode,
            Option<&InheritedVisibility>,
        ),
        With<PatchDropdownScrollbarTrack>,
    >,
    mut thumb_q: Query<
        (&PatchDropdownScrollbarThumb, &mut Node, &mut Visibility),
        With<PatchDropdownScrollbarThumb>,
    >,
) {
    for (thumb_cfg, mut thumb_node, mut thumb_visibility) in &mut thumb_q {
        if state.open != Some(thumb_cfg.bound) {
            *thumb_visibility = Visibility::Hidden;
            continue;
        }

        let Some((scroll, scroll_computed, list_children, list_visibility)) = list_q
            .iter()
            .find_map(|(cfg, scroll, computed, children, visibility)| {
                (cfg.bound == thumb_cfg.bound).then_some((scroll, computed, children, visibility))
            })
        else {
            *thumb_visibility = Visibility::Hidden;
            continue;
        };

        let Some((track_computed, track_visibility)) =
            track_q.iter().find_map(|(cfg, computed, visibility)| {
                (cfg.bound == thumb_cfg.bound).then_some((computed, visibility))
            })
        else {
            *thumb_visibility = Visibility::Hidden;
            continue;
        };

        let list_visible = list_visibility.map(|v| v.get()).unwrap_or(true);
        let track_visible = track_visibility.map(|v| v.get()).unwrap_or(true);
        if !list_visible || !track_visible {
            *thumb_visibility = Visibility::Hidden;
            continue;
        }

        let inv = scroll_computed.inverse_scale_factor();
        let viewport_h = scroll_computed.size().y * inv;
        let track_h = track_computed.size().y * track_computed.inverse_scale_factor();
        if viewport_h <= f32::EPSILON || track_h <= f32::EPSILON {
            *thumb_visibility = Visibility::Hidden;
            continue;
        }

        let (_, _, _, max_offset) =
            patch_dropdown_scroll_metrics(scroll.0.y, scroll_computed, viewport_h, list_children);
        if max_offset <= f32::EPSILON {
            thumb_node.top = Val::Px(0.0);
            thumb_node.height = Val::Px(track_h);
            *thumb_visibility = Visibility::Visible;
            continue;
        }

        let content_h = viewport_h + max_offset;
        let ratio = (viewport_h / content_h).clamp(0.0, 1.0);
        let thumb_h = (track_h * ratio).clamp(PATCH_DROPDOWN_SCROLLBAR_MIN_THUMB, track_h);
        let travel_h = (track_h - thumb_h).max(0.0);
        let offset_ratio = (scroll.0.y.clamp(0.0, max_offset) / max_offset).clamp(0.0, 1.0);
        let thumb_top = travel_h * offset_ratio;

        thumb_node.top = Val::Px(thumb_top);
        thumb_node.height = Val::Px(thumb_h);
        *thumb_visibility = Visibility::Visible;
    }
}

pub(in crate::plugins::ui) fn handle_patch_entry_click(
    mut patch_filter: ResMut<PatchFilterState>,
    mut state: ResMut<PatchDropdownState>,
    mut query: Query<(&Interaction, &PatchEntry), Changed<Interaction>>,
) {
    for (interaction, entry) in &mut query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match entry.bound {
            PatchBound::From => {
                state.from_patch_id = Some(entry.patch_id.clone());
            }
            PatchBound::To => {
                state.to_patch_id = Some(entry.patch_id.clone());
            }
        }
        state.open = None;
        normalize_patch_selection(&mut patch_filter, &mut state);
    }
}
