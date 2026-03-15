use super::*;

pub(super) type ToggleButtonInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        &'static mut ClassList,
        &'static Children,
        Option<&'static ToggleEffort>,
        Option<&'static TogglePoints>,
        Option<&'static ToggleDrift>,
        Option<&'static ToggleZoneMask>,
    ),
    Changed<Interaction>,
>;

pub(super) type ToggleVisualQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut ClassList,
        &'static Children,
        Option<&'static ToggleEffort>,
        Option<&'static TogglePoints>,
        Option<&'static ToggleDrift>,
        Option<&'static ToggleZoneMask>,
    ),
>;

pub(super) fn handle_toggle_buttons(
    mut display_state: ResMut<MapDisplayState>,
    layer_registry: Res<LayerRegistry>,
    mut layer_settings: ResMut<LayerSettings>,
    mut query: ToggleButtonInteractionQuery<'_, '_>,
    mut text_q: Query<&mut Text>,
) {
    for (interaction, mut classes, children, effort, points, drift, zone_mask) in &mut query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if effort.is_some() {
            display_state.show_effort = !display_state.show_effort;
            apply_toggle_visuals(
                display_state.show_effort,
                "Effort",
                &mut classes,
                children,
                &mut text_q,
            );
        } else if points.is_some() {
            display_state.show_points = !display_state.show_points;
            apply_toggle_visuals(
                display_state.show_points,
                "Points",
                &mut classes,
                children,
                &mut text_q,
            );
        } else if drift.is_some() {
            display_state.show_drift = !display_state.show_drift;
            apply_toggle_visuals(
                display_state.show_drift,
                "Drift",
                &mut classes,
                children,
                &mut text_q,
            );
        } else if zone_mask.is_some() {
            display_state.show_zone_mask = !display_state.show_zone_mask;
            if let Some(mask_layer_id) =
                layer_registry.first_id_by_pick_mode(PickMode::ExactTilePixel)
            {
                layer_settings.set_visible(mask_layer_id, display_state.show_zone_mask);
            }
            apply_toggle_visuals(
                display_state.show_zone_mask,
                "Mask",
                &mut classes,
                children,
                &mut text_q,
            );
        }
    }
}

pub(super) fn sync_toggle_visuals(
    display_state: Res<MapDisplayState>,
    mut query: ToggleVisualQuery<'_, '_>,
    mut text_q: Query<&mut Text>,
) {
    if !display_state.is_changed() {
        return;
    }
    for (mut classes, children, effort, points, drift, zone_mask) in &mut query {
        if effort.is_some() {
            apply_toggle_visuals(
                display_state.show_effort,
                "Effort",
                &mut classes,
                children,
                &mut text_q,
            );
        } else if points.is_some() {
            apply_toggle_visuals(
                display_state.show_points,
                "Points",
                &mut classes,
                children,
                &mut text_q,
            );
        } else if drift.is_some() {
            apply_toggle_visuals(
                display_state.show_drift,
                "Drift",
                &mut classes,
                children,
                &mut text_q,
            );
        } else if zone_mask.is_some() {
            apply_toggle_visuals(
                display_state.show_zone_mask,
                "Mask",
                &mut classes,
                children,
                &mut text_q,
            );
        }
    }
}

pub(super) fn handle_mask_opacity_buttons(
    mut display_state: ResMut<MapDisplayState>,
    layer_registry: Res<LayerRegistry>,
    mut layer_settings: ResMut<LayerSettings>,
    query_down: Query<&Interaction, (With<MaskOpacityDown>, Changed<Interaction>)>,
    query_up: Query<&Interaction, (With<MaskOpacityUp>, Changed<Interaction>)>,
) {
    let mut changed = false;
    for interaction in &query_down {
        if *interaction == Interaction::Pressed {
            display_state.zone_mask_opacity =
                (display_state.zone_mask_opacity - 0.05).clamp(0.0, 1.0);
            changed = true;
        }
    }
    for interaction in &query_up {
        if *interaction == Interaction::Pressed {
            display_state.zone_mask_opacity =
                (display_state.zone_mask_opacity + 0.05).clamp(0.0, 1.0);
            changed = true;
        }
    }
    if changed {
        display_state.zone_mask_opacity = (display_state.zone_mask_opacity * 100.0).round() / 100.0;
        if let Some(mask_layer_id) = layer_registry.first_id_by_pick_mode(PickMode::ExactTilePixel)
        {
            layer_settings.set_opacity(mask_layer_id, display_state.zone_mask_opacity);
        }
    }
}

pub(super) fn sync_mask_opacity_text(
    display_state: Res<MapDisplayState>,
    mut query: Query<&mut Text, With<MaskOpacityText>>,
) {
    if !display_state.is_changed() {
        return;
    }
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    text.0 = format!(
        "Mask {:>3}%",
        (display_state.zone_mask_opacity * 100.0).round() as i32
    );
}

pub(super) fn apply_toggle_visuals(
    active: bool,
    label: &str,
    classes: &mut ClassList,
    children: &Children,
    text_q: &mut Query<&mut Text>,
) {
    if active {
        classes.add("on");
    } else {
        classes.remove("on");
    }
    for child in children.iter() {
        if let Ok(mut text) = text_q.get_mut(child) {
            text.0 = format!("{label}: {}", if active { "On" } else { "Off" });
            break;
        }
    }
}
