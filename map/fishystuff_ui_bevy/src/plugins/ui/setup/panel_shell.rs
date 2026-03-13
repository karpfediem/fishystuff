use super::super::*;
use super::SetupTextStyles;

pub(super) fn spawn_zone_panel(root: &mut ChildSpawnerCommands, styles: &SetupTextStyles) {
    root.spawn((
        PanelRoot,
        UiPointerBlocker,
        FocusPolicy::Block,
        GlobalZIndex(1100),
        Node {
            width: Val::Px(ZONE_MENU_WIDTH),
            height: Val::Px(ZONE_MENU_HEIGHT),
            display: Display::None,
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(12.0),
            padding: UiRect::all(Val::Px(12.0)),
            row_gap: Val::Px(8.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            overflow: Overflow::clip_y(),
            ..default()
        },
        Visibility::Hidden,
        ClassList::new("panel"),
    ))
    .with_children(|panel| {
        panel
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                ClassList::new("panel-header"),
            ))
            .with_children(|header| {
                header.spawn((
                    PanelTitleText,
                    UiTextBundle::new("FishyStuff Zones", &styles.title_style),
                    ClassList::new("panel-title"),
                ));
            });

        panel.spawn((
            SelectedZoneText,
            UiTextBundle::new("RGB: (none)", &styles.small_style),
            ClassList::new("label"),
        ));
        panel.spawn((
            UiTextBundle::new("Evidence", &styles.small_style),
            ClassList::new("section-title"),
        ));
        panel
            .spawn((
                Node {
                    display: Display::Grid,
                    width: Val::Percent(100.0),
                    height: Val::Px(260.0),
                    min_height: Val::Px(160.0),
                    max_width: Val::Px(332.0),
                    grid_template_columns: vec![
                        RepeatedGridTrack::flex(1, 1.0),
                        RepeatedGridTrack::auto(1),
                    ],
                    grid_template_rows: vec![RepeatedGridTrack::flex(1, 1.0)],
                    column_gap: Val::Px(3.0),
                    ..default()
                },
                ClassList::new("zone-evidence-frame"),
            ))
            .with_children(|frame| {
                frame
                    .spawn((
                        ZoneEvidenceScroll,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            grid_row: GridPlacement::start(1),
                            grid_column: GridPlacement::start(1),
                            padding: UiRect {
                                left: Val::Px(6.0),
                                right: Val::Px(8.0),
                                top: Val::Px(6.0),
                                bottom: Val::Px(6.0),
                            },
                            row_gap: Val::Px(6.0),
                            flex_direction: FlexDirection::Column,
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        ScrollPosition::default(),
                        RelativeCursorPosition::default(),
                        ClassList::new("zone-evidence-scroll"),
                    ))
                    .with_children(|scroll| {
                        scroll.spawn((
                            ZoneEvidenceList,
                            Node {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(6.0),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            ClassList::new("zone-evidence-list"),
                        ));
                    });

                frame
                    .spawn((
                        Node {
                            min_width: Val::Px(8.0),
                            grid_row: GridPlacement::start(1),
                            grid_column: GridPlacement::start(2),
                            border_radius: BorderRadius::all(Val::Px(999.0)),
                            ..default()
                        },
                        ZoneEvidenceScrollbarTrack,
                        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)),
                        ClassList::new("zone-evidence-scrollbar-track"),
                    ))
                    .with_children(|track| {
                        track.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                right: Val::Px(0.0),
                                top: Val::Px(0.0),
                                height: Val::Px(EVIDENCE_SCROLLBAR_MIN_THUMB),
                                border_radius: BorderRadius::all(Val::Px(999.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.78, 0.88, 1.0, 0.95)),
                            ZoneEvidenceScrollbarThumb,
                            Visibility::Visible,
                            ClassList::new("zone-evidence-scrollbar-thumb"),
                        ));
                    });
            });
    });
}
