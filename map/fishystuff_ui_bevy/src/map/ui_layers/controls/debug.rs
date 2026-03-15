use super::super::*;

type DebugToggleInteractionQuery<'w, 's> = Query<
    'w,
    's,
    &'static Interaction,
    (
        Changed<Interaction>,
        With<Button>,
        With<LayerDebugToggleButton>,
    ),
>;

type EvictionToggleInteractionQuery<'w, 's> = Query<
    'w,
    's,
    &'static Interaction,
    (
        Changed<Interaction>,
        With<Button>,
        With<LayerEvictionToggleButton>,
    ),
>;

pub(in crate::map::ui_layers) fn handle_debug_toggle_clicks(
    mut debug: ResMut<LayerDebugSettings>,
    mut interaction_q: DebugToggleInteractionQuery<'_, '_>,
) {
    for interaction in &mut interaction_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        debug.enabled = !debug.enabled;
    }
}

pub(in crate::map::ui_layers) fn handle_eviction_toggle_clicks(
    mut controls: ResMut<TileDebugControls>,
    mut interaction_q: EvictionToggleInteractionQuery<'_, '_>,
) {
    for interaction in &mut interaction_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        controls.disable_eviction = !controls.disable_eviction;
    }
}

pub(in crate::map::ui_layers) fn sync_debug_toggle_label(
    debug: Res<LayerDebugSettings>,
    mut text_q: Query<&mut Text, With<LayerDebugToggleText>>,
    mut button_q: Query<&mut ClassList, (With<LayerDebugToggleButton>, With<Button>)>,
) {
    if !debug.is_changed() {
        return;
    }
    for mut classes in &mut button_q {
        if debug.enabled {
            classes.add("on");
        } else {
            classes.remove("on");
        }
    }
    for mut text in &mut text_q {
        text.0 = if debug.enabled {
            "Debug: On".to_string()
        } else {
            "Debug: Off".to_string()
        };
    }
}

pub(in crate::map::ui_layers) fn sync_eviction_toggle_label(
    controls: Res<TileDebugControls>,
    mut text_q: Query<&mut Text, With<LayerEvictionToggleText>>,
    mut button_q: Query<&mut ClassList, (With<LayerEvictionToggleButton>, With<Button>)>,
) {
    if !controls.is_changed() {
        return;
    }
    for mut classes in &mut button_q {
        if controls.disable_eviction {
            classes.remove("on");
        } else {
            classes.add("on");
        }
    }
    for mut text in &mut text_q {
        text.0 = if controls.disable_eviction {
            "Evict: Off".to_string()
        } else {
            "Evict: On".to_string()
        };
    }
}
