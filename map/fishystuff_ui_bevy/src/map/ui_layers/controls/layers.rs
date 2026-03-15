use super::super::*;
use bevy::ecs::system::SystemParam;

type LayerToggleInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (&'static LayerToggleButton, &'static Interaction),
    (Changed<Interaction>, With<Button>),
>;

type LayerOpacityDownQuery<'w, 's> = Query<
    'w,
    's,
    (&'static LayerOpacityDown, &'static Interaction),
    (Changed<Interaction>, With<Button>),
>;

type LayerOpacityUpQuery<'w, 's> = Query<
    'w,
    's,
    (&'static LayerOpacityUp, &'static Interaction),
    (Changed<Interaction>, With<Button>),
>;

pub(in crate::map::ui_layers) fn handle_layer_toggle_clicks(
    mut settings: ResMut<LayerSettings>,
    mut interaction_q: LayerToggleInteractionQuery<'_, '_>,
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
    mut down_q: LayerOpacityDownQuery<'_, '_>,
    mut up_q: LayerOpacityUpQuery<'_, '_>,
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
    mut text_q: LayerTextQueries<'_, '_>,
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
    for (toggle_text, mut text) in &mut text_q.toggle_texts {
        let next = if settings.visible(toggle_text.id) {
            "On".to_string()
        } else {
            "Off".to_string()
        };
        if text.0 != next {
            text.0 = next;
        }
    }
    for (label, mut text) in &mut text_q.labels {
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

#[derive(SystemParam)]
pub(in crate::map::ui_layers) struct LayerTextQueries<'w, 's> {
    labels: Query<'w, 's, (&'static LayerLabel, &'static mut Text)>,
    toggle_texts: Query<'w, 's, (&'static LayerToggleText, &'static mut Text)>,
}
