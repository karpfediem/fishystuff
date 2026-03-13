use super::*;

pub(super) fn spawn_layer_rows(
    panel: &mut ChildSpawnerCommands<'_>,
    registry: &LayerRegistry,
    font: &Handle<Font>,
) {
    for spec in registry.ordered() {
        panel
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    column_gap: Val::Px(6.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                ClassList::new("row layers-row"),
            ))
            .with_children(|row| spawn_layer_row(row, spec.id, &spec.name, font));
    }
}

fn spawn_layer_row(
    row: &mut ChildSpawnerCommands<'_>,
    id: LayerId,
    label: &str,
    font: &Handle<Font>,
) {
    row.spawn((
        Button,
        LayerToggleButton { id },
        Node {
            width: Val::Px(56.0),
            height: Val::Px(24.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ClassList::new("btn toggle layers-btn on"),
    ))
    .with_children(|button| {
        button.spawn((
            LayerToggleText { id },
            Text::new("On"),
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
        LayerLabel { id },
        Text::new(label),
        TextFont {
            font: font.clone(),
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.94)),
        ClassList::new("value layers-label"),
    ));

    spawn_opacity_button(row, LayerOpacityDown { id }, "-", font, true);
    spawn_opacity_button(row, LayerOpacityUp { id }, "+", font, false);
}

fn spawn_opacity_button<T: Component>(
    row: &mut ChildSpawnerCommands<'_>,
    marker: T,
    label: &str,
    font: &Handle<Font>,
    push_to_end: bool,
) {
    let margin = if push_to_end {
        UiRect::left(Val::Auto)
    } else {
        UiRect::default()
    };
    row.spawn((
        Button,
        marker,
        Node {
            width: Val::Px(24.0),
            height: Val::Px(24.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            margin,
            ..default()
        },
        ClassList::new("btn layers-btn"),
    ))
    .with_children(|button| {
        button.spawn((
            Text::new(label),
            TextFont {
                font: font.clone(),
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::WHITE),
            ClassList::new("btn-text"),
        ));
    });
}
