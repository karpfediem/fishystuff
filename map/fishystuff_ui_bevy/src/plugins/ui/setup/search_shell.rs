use super::super::*;
use super::SetupTextStyles;

pub(super) fn spawn_search_anchor(root: &mut ChildSpawnerCommands, styles: &SetupTextStyles) {
    root.spawn((
        Node {
            width: Val::Percent(100.0),
            display: Display::None,
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(14.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::FlexStart,
            ..default()
        },
        Visibility::Hidden,
        ClassList::new("search-anchor"),
    ))
    .with_children(|anchor| {
        anchor
            .spawn((
                UiPointerBlocker,
                FocusPolicy::Block,
                GlobalZIndex(1250),
                Node {
                    width: Val::Percent(46.0),
                    min_width: Val::Px(320.0),
                    max_width: Val::Px(560.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    row_gap: Val::Px(6.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ClassList::new("search-shell"),
            ))
            .with_children(|search| {
                search
                    .spawn((
                        FishSearchInput,
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(34.0),
                            padding: UiRect::horizontal(Val::Px(10.0)),
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        ClassList::new("input search-input"),
                    ))
                    .with_children(|input| {
                        input.spawn((
                            FishSearchText,
                            UiTextBundle::new("Type fish name...", &styles.value_style),
                            ClassList::new("input-text placeholder"),
                        ));
                    });

                search.spawn((
                    FishSearchTags,
                    Node {
                        width: Val::Percent(100.0),
                        display: Display::None,
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(6.0),
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                    Visibility::Hidden,
                    ClassList::new("search-tags"),
                ));

                search
                    .spawn((
                        FishAutocompleteFrame,
                        Node {
                            width: Val::Percent(100.0),
                            display: Display::None,
                            position_type: PositionType::Relative,
                            max_height: Val::Px(AUTOCOMPLETE_DROPDOWN_MAX_HEIGHT),
                            ..default()
                        },
                        Visibility::Hidden,
                        ClassList::new("search-results-frame"),
                    ))
                    .with_children(|frame| {
                        frame
                            .spawn((
                                FishAutocompleteScroll,
                                Node {
                                    width: Val::Percent(100.0),
                                    max_height: Val::Px(AUTOCOMPLETE_DROPDOWN_MAX_HEIGHT),
                                    padding: UiRect {
                                        left: Val::Px(0.0),
                                        right: Val::Px(12.0),
                                        top: Val::Px(0.0),
                                        bottom: Val::Px(0.0),
                                    },
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(AUTOCOMPLETE_LIST_ROW_GAP),
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
                                ClassList::new("autocomplete-scroll search-results"),
                            ))
                            .with_children(|scroll| {
                                scroll
                                    .spawn((
                                        FishAutocompleteList,
                                        Node {
                                            width: Val::Percent(100.0),
                                            flex_direction: FlexDirection::Column,
                                            row_gap: Val::Px(AUTOCOMPLETE_LIST_ROW_GAP),
                                            flex_shrink: 0.0,
                                            ..default()
                                        },
                                    ))
                                    .with_children(|list| {
                                        for idx in 0..AUTOCOMPLETE_MAX {
                                            spawn_autocomplete_entry(
                                                list,
                                                idx,
                                                styles.value_style.clone(),
                                            );
                                        }
                                    });
                            });

                        frame
                            .spawn((
                                FishAutocompleteScrollbarTrack,
                                Node {
                                    position_type: PositionType::Absolute,
                                    right: Val::Px(3.0),
                                    top: Val::Px(5.0),
                                    bottom: Val::Px(5.0),
                                    width: Val::Px(8.0),
                                    border_radius: BorderRadius::all(Val::Px(999.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)),
                                ClassList::new("autocomplete-scrollbar-track"),
                            ))
                            .with_children(|track| {
                                track.spawn((
                                    FishAutocompleteScrollbarThumb,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(0.0),
                                        right: Val::Px(0.0),
                                        top: Val::Px(0.0),
                                        height: Val::Px(AUTOCOMPLETE_SCROLLBAR_MIN_THUMB),
                                        border_radius: BorderRadius::all(Val::Px(999.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.78, 0.88, 1.0, 0.95)),
                                    Visibility::Visible,
                                    ClassList::new("autocomplete-scrollbar-thumb"),
                                ));
                            });
                    });
            });
    });
}

fn spawn_autocomplete_entry(
    parent: &mut ChildSpawnerCommands,
    idx: usize,
    text_style: UiTextStyle,
) {
    parent
        .spawn((
            FishAutocompleteEntry { idx },
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(26.0),
                display: Display::None,
                padding: UiRect::horizontal(Val::Px(6.0)),
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                ..default()
            },
            Visibility::Hidden,
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.001)),
            ClassList::new("list-item"),
        ))
        .with_children(|entry| {
            entry.spawn((
                FishAutocompleteEntryIcon,
                ImageNode::default(),
                Node {
                    width: Val::Px(16.0),
                    height: Val::Px(16.0),
                    ..default()
                },
                Visibility::Hidden,
                ClassList::new("search-result-icon"),
            ));
            entry.spawn((
                FishAutocompleteEntryText,
                UiTextBundle::new("", &text_style),
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                ClassList::new("fish-name"),
            ));
        });
}
