use super::super::*;

pub(in crate::map::ui_layers) fn handle_layer_toggle_clicks(
    mut settings: ResMut<LayerSettings>,
    mut interaction_q: Query<
        (&LayerToggleButton, &Interaction),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (button, interaction) in &mut interaction_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let visible = settings.visible(button.id);
        settings.set_visible(button.id, !visible);
    }
}

pub(in crate::map::ui_layers) fn handle_layer_opacity_clicks(
    mut settings: ResMut<LayerSettings>,
    mut down_q: Query<(&LayerOpacityDown, &Interaction), (Changed<Interaction>, With<Button>)>,
    mut up_q: Query<(&LayerOpacityUp, &Interaction), (Changed<Interaction>, With<Button>)>,
) {
    for (button, interaction) in &mut down_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let opacity = (settings.opacity(button.id) - 0.05).max(0.0);
        settings.set_opacity(button.id, opacity);
    }
    for (button, interaction) in &mut up_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let opacity = (settings.opacity(button.id) + 0.05).min(1.0);
        settings.set_opacity(button.id, opacity);
    }
}

pub(in crate::map::ui_layers) fn sync_layer_labels(
    registry: Res<LayerRegistry>,
    settings: Res<LayerSettings>,
    stats: Res<TileStats>,
    mut toggle_q: Query<(&LayerToggleButton, &mut ClassList), With<Button>>,
    mut text_q: ParamSet<(
        Query<(&LayerLabel, &mut Text)>,
        Query<(&LayerToggleText, &mut Text)>,
    )>,
) {
    if !settings.is_changed() && !registry.is_changed() && !stats.is_changed() {
        return;
    }
    for (toggle, mut classes) in &mut toggle_q {
        if settings.visible(toggle.id) {
            classes.add("on");
        } else {
            classes.remove("on");
        }
    }
    for (toggle_text, mut text) in &mut text_q.p1() {
        let next = if settings.visible(toggle_text.id) {
            "On".to_string()
        } else {
            "Off".to_string()
        };
        if text.0 != next {
            text.0 = next;
        }
    }
    for (label, mut text) in &mut text_q.p0() {
        if settings.get(label.id).is_none() {
            let next = registry.label(label.id).to_string();
            if text.0 != next {
                text.0 = next;
            }
            continue;
        }
        let next = registry.label(label.id).to_string();
        if text.0 != next {
            text.0 = next;
        }
    }
}
