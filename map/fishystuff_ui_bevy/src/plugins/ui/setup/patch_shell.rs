use super::super::*;
use super::SetupTextStyles;

pub(super) fn spawn_patch_panel(root: &mut ChildSpawnerCommands, styles: &SetupTextStyles) {
    root.spawn((
        UiPointerBlocker,
        FocusPolicy::Block,
        GlobalZIndex(1300),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(PATCH_MENU_RIGHT),
            bottom: Val::Px(PATCH_MENU_BOTTOM),
            width: Val::Px(PATCH_MENU_WIDTH),
            height: Val::Px(PATCH_MENU_HEIGHT),
            display: Display::None,
            padding: UiRect::all(Val::Px(10.0)),
            row_gap: Val::Px(8.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            overflow: Overflow::scroll_y(),
            ..default()
        },
        Visibility::Hidden,
        ClassList::new("panel patch-panel"),
    ))
    .with_children(|patch| {
        patch.spawn((
            UiTextBundle::new("Patch Range", &styles.label_style),
            ClassList::new("section-title"),
        ));
        spawn_patch_range_group(patch, PatchBound::From, "From", styles);
        spawn_patch_range_group(patch, PatchBound::To, "To (including)", styles);
        spawn_point_icon_size_group(patch, styles);
    });
}

fn spawn_patch_range_group(
    patch: &mut ChildSpawnerCommands,
    bound: PatchBound,
    label: &str,
    styles: &SetupTextStyles,
) {
    patch
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                overflow: Overflow::visible(),
                ..default()
            },
            ClassList::new("patch-range-group"),
        ))
        .with_children(|group| {
            group.spawn((
                UiTextBundle::new(label, &styles.small_style),
                ClassList::new("label"),
            ));
            group
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(30.0),
                        overflow: Overflow::visible(),
                        ..default()
                    },
                    ClassList::new("patch-picker"),
                ))
                .with_children(|picker| {
                    picker
                        .spawn((
                            PatchRangeButton { bound },
                            Button,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(30.0),
                                padding: UiRect::horizontal(Val::Px(8.0)),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            ClassList::new("btn primary"),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                PatchRangeButtonText { bound },
                                UiTextBundle::new("(select patch)", &styles.label_style),
                                ClassList::new("btn-label"),
                            ));
                        });

                    picker.spawn((
                        PatchDropdownList { bound },
                        ZIndex(40),
                        GlobalZIndex(1325),
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Px(34.0),
                            left: Val::Px(0.0),
                            width: Val::Percent(100.0),
                            height: Val::Px(0.0),
                            min_height: Val::Px(0.0),
                            max_height: Val::Px(0.0),
                            padding: UiRect {
                                left: Val::Px(0.0),
                                right: Val::Px(12.0),
                                top: Val::Px(0.0),
                                bottom: Val::Px(0.0),
                            },
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(2.0),
                            border: UiRect::all(Val::Px(1.0)),
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(
                            20.0 / 255.0,
                            24.0 / 255.0,
                            30.0 / 255.0,
                            0.92,
                        )),
                        BorderColor::all(Color::srgba(
                            80.0 / 255.0,
                            97.0 / 255.0,
                            118.0 / 255.0,
                            0.75,
                        )),
                        ScrollPosition::default(),
                        RelativeCursorPosition::default(),
                        Visibility::Hidden,
                        ClassList::new("patch-dropdown-scroll patch-dropdown"),
                    ));

                    picker
                        .spawn((
                            PatchDropdownScrollbarTrack { bound },
                            ZIndex(41),
                            GlobalZIndex(1326),
                            Node {
                                position_type: PositionType::Absolute,
                                top: Val::Px(34.0),
                                right: Val::Px(2.0),
                                width: Val::Px(8.0),
                                height: Val::Px(0.0),
                                min_height: Val::Px(0.0),
                                max_height: Val::Px(0.0),
                                border_radius: BorderRadius::all(Val::Px(999.0)),
                                ..default()
                            },
                            Visibility::Hidden,
                            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)),
                            ClassList::new("patch-dropdown-scrollbar-track"),
                        ))
                        .with_children(|track| {
                            track.spawn((
                                PatchDropdownScrollbarThumb { bound },
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(0.0),
                                    right: Val::Px(0.0),
                                    top: Val::Px(0.0),
                                    height: Val::Px(PATCH_DROPDOWN_SCROLLBAR_MIN_THUMB),
                                    border_radius: BorderRadius::all(Val::Px(999.0)),
                                    ..default()
                                },
                                Visibility::Visible,
                                BackgroundColor(Color::srgba(0.78, 0.88, 1.0, 0.95)),
                                ClassList::new("patch-dropdown-scrollbar-thumb"),
                            ));
                        });
                });
        });
}

fn spawn_point_icon_size_group(patch: &mut ChildSpawnerCommands, styles: &SetupTextStyles) {
    patch
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                margin: UiRect {
                    top: Val::Px(2.0),
                    ..default()
                },
                ..default()
            },
            ClassList::new("point-icon-size-group"),
        ))
        .with_children(|group| {
            group
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ClassList::new("row"),
                ))
                .with_children(|row| {
                    row.spawn((
                        UiTextBundle::new("Fish Icon Size", &styles.small_style),
                        ClassList::new("label"),
                    ));
                    row.spawn((
                        PointIconSizeValueText,
                        UiTextBundle::new("100%", &styles.small_style),
                        ClassList::new("value point-icon-size-value"),
                    ));
                });

            group
                .spawn((
                    PointIconSizeSliderTrack,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(8.0),
                        border_radius: BorderRadius::all(Val::Px(999.0)),
                        position_type: PositionType::Relative,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.16)),
                    RelativeCursorPosition::default(),
                    ClassList::new("point-icon-size-slider-track"),
                ))
                .with_children(|track| {
                    track.spawn((
                        PointIconSizeSliderThumb,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(-3.0),
                            width: Val::Px(POINT_ICON_SIZE_SLIDER_MIN_THUMB),
                            height: Val::Px(POINT_ICON_SIZE_SLIDER_MIN_THUMB),
                            border_radius: BorderRadius::all(Val::Px(999.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.78, 0.88, 1.0, 0.95)),
                        Visibility::Visible,
                        ClassList::new("point-icon-size-slider-thumb"),
                    ));
                });
        });
}
