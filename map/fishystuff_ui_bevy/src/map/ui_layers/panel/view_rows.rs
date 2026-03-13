use super::super::controls::show_drape_label;
use super::*;

pub(super) fn spawn_view_rows(
    panel: &mut ChildSpawnerCommands<'_>,
    display_state: &MapDisplayState,
    view_mode: &ViewModeState,
    terrain_cfg: &Terrain3dConfig,
    font: &Handle<Font>,
) {
    spawn_view_toggle_row(panel, display_state, font);
    spawn_view_mode_row(panel, view_mode, font);
    spawn_drape_row(panel, terrain_cfg, font);
}

fn spawn_view_toggle_row(
    panel: &mut ChildSpawnerCommands<'_>,
    display_state: &MapDisplayState,
    font: &Handle<Font>,
) {
    panel
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(6.0),
                ..default()
            },
            ClassList::new("row layers-row"),
        ))
        .with_children(|row| {
            spawn_view_toggle(
                row,
                ViewToggleKind::Effort,
                "Effort",
                display_state.show_effort,
                font,
            );
            spawn_view_toggle(
                row,
                ViewToggleKind::Points,
                "Points",
                display_state.show_points,
                font,
            );
            spawn_view_toggle(
                row,
                ViewToggleKind::PointIcons,
                "Icons",
                display_state.show_point_icons,
                font,
            );
            spawn_view_toggle(
                row,
                ViewToggleKind::Drift,
                "Drift",
                display_state.show_drift,
                font,
            );
        });
}

fn spawn_view_mode_row(
    panel: &mut ChildSpawnerCommands<'_>,
    view_mode: &ViewModeState,
    font: &Handle<Font>,
) {
    panel
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(6.0),
                ..default()
            },
            ClassList::new("row layers-row"),
        ))
        .with_children(|row| {
            spawn_view_mode_toggle(
                row,
                ViewMode::Map2D,
                "2D",
                view_mode.mode == ViewMode::Map2D,
                font,
            );
            spawn_view_mode_toggle(
                row,
                ViewMode::Terrain3D,
                "3D",
                view_mode.mode == ViewMode::Terrain3D,
                font,
            );
            row.spawn((
                Button,
                Reset3dViewButton,
                Node {
                    flex_grow: 1.0,
                    height: Val::Px(24.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ClassList::new("btn layers-btn"),
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("Reset 3D"),
                    TextFont {
                        font: font.clone(),
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ClassList::new("btn-text"),
                ));
            });
        });
}

fn spawn_drape_row(
    panel: &mut ChildSpawnerCommands<'_>,
    terrain_cfg: &Terrain3dConfig,
    font: &Handle<Font>,
) {
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
        .with_children(|row| {
            let mut drape_classes = ClassList::new("btn toggle layers-btn");
            if terrain_cfg.show_drape {
                drape_classes.add("on");
            }
            row.spawn((
                Button,
                TerrainShowDrapeToggle,
                Node {
                    flex_grow: 1.0,
                    height: Val::Px(24.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                drape_classes,
            ))
            .with_children(|button| {
                button.spawn((
                    TerrainShowDrapeText,
                    Text::new(show_drape_label(terrain_cfg.show_drape)),
                    TextFont {
                        font: font.clone(),
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ClassList::new("btn-text"),
                ));
            });
        });
}

fn spawn_view_toggle(
    row: &mut ChildSpawnerCommands<'_>,
    kind: ViewToggleKind,
    label: &str,
    active: bool,
    font: &Handle<Font>,
) {
    let mut classes = ClassList::new("btn toggle layers-btn");
    if active {
        classes.add("on");
    }
    row.spawn((
        Button,
        ViewToggleButton { kind },
        Node {
            flex_grow: 1.0,
            height: Val::Px(24.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        classes,
    ))
    .with_children(|button| {
        button.spawn((
            ViewToggleText { kind },
            Text::new(label),
            TextFont {
                font: font.clone(),
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::WHITE),
            ClassList::new("btn-text"),
        ));
    });
}

fn spawn_view_mode_toggle(
    row: &mut ChildSpawnerCommands<'_>,
    mode: ViewMode,
    label: &str,
    active: bool,
    font: &Handle<Font>,
) {
    let mut classes = ClassList::new("btn toggle layers-btn");
    if active {
        classes.add("on");
    }
    row.spawn((
        Button,
        ViewModeButton { mode },
        Node {
            flex_grow: 1.0,
            height: Val::Px(24.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        classes,
    ))
    .with_children(|button| {
        button.spawn((
            ViewModeText { mode },
            Text::new(label),
            TextFont {
                font: font.clone(),
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::WHITE),
            ClassList::new("btn-text"),
        ));
    });
}
