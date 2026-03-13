use super::*;

pub(super) fn spawn_debug_controls(
    panel: &mut ChildSpawnerCommands<'_>,
    tile_debug: &TileDebugControls,
    font: &Handle<Font>,
) {
    panel
        .spawn((
            Node {
                width: Val::Percent(100.0),
                column_gap: Val::Px(8.0),
                align_items: AlignItems::Center,
                margin: UiRect::top(Val::Px(6.0)),
                ..default()
            },
            ClassList::new("row layers-debug-row"),
        ))
        .with_children(|row| {
            let mut eviction_classes = ClassList::new("btn toggle layers-btn");
            if !tile_debug.disable_eviction {
                eviction_classes.add("on");
            }
            row.spawn((
                Button,
                LayerDebugToggleButton,
                Node {
                    width: Val::Px(84.0),
                    height: Val::Px(24.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ClassList::new("btn toggle layers-btn"),
            ))
            .with_children(|button| {
                button.spawn((
                    LayerDebugToggleText,
                    Text::new("Debug: Off"),
                    TextFont {
                        font: font.clone(),
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ClassList::new("btn-text"),
                ));
            });
            row.spawn((
                Button,
                LayerEvictionToggleButton,
                Node {
                    width: Val::Px(84.0),
                    height: Val::Px(24.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                eviction_classes,
            ))
            .with_children(|button| {
                button.spawn((
                    LayerEvictionToggleText,
                    Text::new(if tile_debug.disable_eviction {
                        "Evict: Off"
                    } else {
                        "Evict: On"
                    }),
                    TextFont {
                        font: font.clone(),
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ClassList::new("btn-text"),
                ));
            });
            row.spawn((
                Text::new("Show diagnostics / toggle tile eviction"),
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.74, 0.8)),
                ClassList::new("label"),
            ));
        });

    panel.spawn((
        LayerDebugText,
        Text::new(""),
        TextFont {
            font: font.clone(),
            font_size: 11.0,
            ..default()
        },
        TextColor(Color::srgb(0.75, 0.75, 0.8)),
        Visibility::Hidden,
        ClassList::new("diagnostics-text layers-debug-text"),
    ));
}
