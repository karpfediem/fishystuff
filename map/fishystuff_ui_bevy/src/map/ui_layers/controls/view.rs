use super::super::*;
use crate::map::layers::FISH_EVIDENCE_LAYER_KEY;

type ViewToggleInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (&'static ViewToggleButton, &'static Interaction),
    (Changed<Interaction>, With<Button>),
>;

type ViewModeInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (&'static ViewModeButton, &'static Interaction),
    (Changed<Interaction>, With<Button>),
>;

type Reset3dInteractionQuery<'w, 's> = Query<
    'w,
    's,
    &'static Interaction,
    (Changed<Interaction>, With<Button>, With<Reset3dViewButton>),
>;

type ShowDrapeInteractionQuery<'w, 's> = Query<
    'w,
    's,
    &'static Interaction,
    (
        Changed<Interaction>,
        With<Button>,
        With<TerrainShowDrapeToggle>,
    ),
>;

pub(in crate::map::ui_layers) fn handle_view_toggle_clicks(
    mut display_state: ResMut<MapDisplayState>,
    layer_registry: Res<LayerRegistry>,
    mut layer_settings: ResMut<LayerSettings>,
    mut interaction_q: ViewToggleInteractionQuery<'_, '_>,
) {
    for (button, interaction) in &mut interaction_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button.kind {
            ViewToggleKind::Effort => display_state.show_effort = !display_state.show_effort,
            ViewToggleKind::Points => {
                let next = !display_state.show_points;
                display_state.show_points = next;
                if let Some(points_layer_id) = layer_registry.id_by_key(FISH_EVIDENCE_LAYER_KEY) {
                    layer_settings.set_visible(points_layer_id, next);
                }
            }
            ViewToggleKind::PointIcons => {
                let next = !display_state.show_point_icons;
                display_state.show_point_icons = next;
                if let Some(points_layer_id) = layer_registry.id_by_key(FISH_EVIDENCE_LAYER_KEY) {
                    layer_settings.set_point_icons_visible(points_layer_id, next);
                }
            }
            ViewToggleKind::Drift => display_state.show_drift = !display_state.show_drift,
        }
    }
}

pub(in crate::map::ui_layers) fn handle_view_mode_clicks(
    mut mode: ResMut<ViewModeState>,
    mut view_3d: ResMut<Terrain3dViewState>,
    mut interaction_q: ViewModeInteractionQuery<'_, '_>,
    mut reset_q: Reset3dInteractionQuery<'_, '_>,
) {
    for (button, interaction) in &mut interaction_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if mode.mode != button.mode {
            mode.mode = button.mode;
            if mode.mode == ViewMode::Terrain3D {
                mode.terrain_initialized = true;
            }
        }
    }
    for interaction in &mut reset_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        reset_terrain3d_view(&mut view_3d);
    }
}

pub(in crate::map::ui_layers) fn handle_terrain_tuning_clicks(
    mut cfg: ResMut<Terrain3dConfig>,
    mut show_drape_q: ShowDrapeInteractionQuery<'_, '_>,
) {
    for interaction in &mut show_drape_q {
        if *interaction == Interaction::Pressed {
            cfg.show_drape = !cfg.show_drape;
        }
    }
}

pub(in crate::map::ui_layers) fn sync_view_toggle_labels(
    display_state: Res<MapDisplayState>,
    mut button_q: Query<(&ViewToggleButton, &mut ClassList), With<Button>>,
    mut text_q: Query<(&ViewToggleText, &mut Text)>,
) {
    if !display_state.is_changed() {
        return;
    }
    for (button, mut classes) in &mut button_q {
        let active = match button.kind {
            ViewToggleKind::Effort => display_state.show_effort,
            ViewToggleKind::Points => display_state.show_points,
            ViewToggleKind::PointIcons => display_state.show_point_icons,
            ViewToggleKind::Drift => display_state.show_drift,
        };
        if active {
            classes.add("on");
        } else {
            classes.remove("on");
        }
    }
    for (toggle, mut text) in &mut text_q {
        let (label, active) = match toggle.kind {
            ViewToggleKind::Effort => ("Effort", display_state.show_effort),
            ViewToggleKind::Points => ("Points", display_state.show_points),
            ViewToggleKind::PointIcons => ("Icons", display_state.show_point_icons),
            ViewToggleKind::Drift => ("Drift", display_state.show_drift),
        };
        text.0 = format!("{label}: {}", if active { "On" } else { "Off" });
    }
}

pub(in crate::map::ui_layers) fn sync_view_mode_labels(
    mode: Res<ViewModeState>,
    mut button_q: Query<(&ViewModeButton, &mut ClassList), With<Button>>,
    mut text_q: Query<(&ViewModeText, &mut Text)>,
) {
    if !mode.is_changed() {
        return;
    }
    for (button, mut classes) in &mut button_q {
        if button.mode == mode.mode {
            classes.add("on");
        } else {
            classes.remove("on");
        }
    }
    for (button, mut text) in &mut text_q {
        let base = match button.mode {
            ViewMode::Map2D => "2D",
            ViewMode::Terrain3D => "3D",
        };
        let active = if button.mode == mode.mode {
            "On"
        } else {
            "Off"
        };
        text.0 = format!("{base}: {active}");
    }
}

pub(in crate::map::ui_layers) fn sync_terrain_tuning_labels(
    cfg: Res<Terrain3dConfig>,
    mut drape_btn_q: Query<&mut ClassList, (With<Button>, With<TerrainShowDrapeToggle>)>,
    mut text_q: Query<&mut Text, With<TerrainShowDrapeText>>,
) {
    if !cfg.is_changed() {
        return;
    }
    for mut classes in &mut drape_btn_q {
        if cfg.show_drape {
            classes.add("on");
        } else {
            classes.remove("on");
        }
    }
    for mut text in &mut text_q {
        text.0 = show_drape_label(cfg.show_drape);
    }
}

pub(in crate::map::ui_layers) fn show_drape_label(enabled: bool) -> String {
    if enabled {
        "Drape: On".to_string()
    } else {
        "Drape: Off".to_string()
    }
}
