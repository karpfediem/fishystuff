use super::setup::text_style;
use super::*;
pub(super) fn handle_search_focus(
    mut focused: ResMut<FocusedInput>,
    mut search: ResMut<SearchState>,
    mut query: Query<
        (Entity, &Interaction, &mut ClassList),
        (With<FishSearchInput>, Changed<Interaction>),
    >,
) {
    for (entity, interaction, mut classes) in &mut query {
        if *interaction == Interaction::Pressed {
            focused.entity = Some(entity);
            search.open = !search.results.is_empty();
            classes.add("is-focused");
        }
    }
}

pub(super) fn handle_text_input(
    mut focused: ResMut<FocusedInput>,
    mut search: ResMut<SearchState>,
    fish: Res<FishCatalog>,
    mut keys_in: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut fish_filter: ResMut<FishFilterState>,
    mut input_q: Query<&mut ClassList, With<FishSearchInput>>,
) {
    let is_focused = focused.entity.is_some();
    if !is_focused {
        return;
    }
    let mut changed = false;
    for ev in keys_in.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        let Some(text) = ev.text.as_deref() else {
            continue;
        };
        for ch in text.chars() {
            if ch.is_control() {
                continue;
            }
            search.query.push(ch);
            changed = true;
        }
    }
    if keys.just_pressed(KeyCode::Backspace) {
        if search.query.is_empty() {
            if search.selected_fish_ids.pop().is_some() {
                apply_search_filters_to_ui(&search, &fish, &mut fish_filter);
            }
        } else {
            search.query.pop();
            changed = true;
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        search.open = false;
        focused.entity = None;
        if let Ok(mut classes) = input_q.single_mut() {
            classes.remove("is-focused");
        }
    }
    if keys.just_pressed(KeyCode::ArrowDown) && !search.results.is_empty() {
        search.selected = (search.selected + 1).min(search.results.len() - 1);
    }
    if keys.just_pressed(KeyCode::ArrowUp) && !search.results.is_empty() {
        if search.selected == 0 {
            search.selected = 0;
        } else {
            search.selected -= 1;
        }
    }
    if keys.just_pressed(KeyCode::Enter) {
        if let Some(idx) = search.results.get(search.selected).copied() {
            apply_fish_selection(idx, &fish, &mut fish_filter, &mut search);
        }
    }
    if changed {
        rebuild_results(&mut search, &fish);
    }
}

pub(super) fn sync_ui_input_capture_state(
    focused: Res<FocusedInput>,
    mut capture: ResMut<UiPointerCapture>,
) {
    capture.text_input_active = focused.entity.is_some();
}

pub(super) fn refresh_search_results(fish: Res<FishCatalog>, mut search: ResMut<SearchState>) {
    if !fish.is_changed() {
        return;
    }
    if search.query.is_empty() {
        return;
    }
    rebuild_results(&mut search, &fish);
}

pub(super) fn handle_search_tag_click(
    fish: Res<FishCatalog>,
    mut fish_filter: ResMut<FishFilterState>,
    mut search: ResMut<SearchState>,
    mut query: Query<(&Interaction, &FishSearchTag), Changed<Interaction>>,
) {
    let mut removed = false;
    for (interaction, tag) in &mut query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Some(pos) = search
            .selected_fish_ids
            .iter()
            .position(|fish_id| *fish_id == tag.fish_id)
        {
            search.selected_fish_ids.remove(pos);
            removed = true;
        }
    }
    if removed {
        apply_search_filters_to_ui(&search, &fish, &mut fish_filter);
        if !search.query.trim().is_empty() {
            rebuild_results(&mut search, &fish);
        }
    }
}

pub(super) fn handle_autocomplete_click(
    fish: Res<FishCatalog>,
    mut fish_filter: ResMut<FishFilterState>,
    mut search: ResMut<SearchState>,
    mut query: Query<(&Interaction, &FishAutocompleteEntry), Changed<Interaction>>,
) {
    for (interaction, entry) in &mut query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Some(idx) = search.results.get(entry.idx).copied() {
            apply_fish_selection(idx, &fish, &mut fish_filter, &mut search);
        }
    }
}

pub(super) fn update_search_text(
    search: Res<SearchState>,
    mut query: Query<(&mut Text, &mut ClassList), With<FishSearchText>>,
) {
    if !search.is_changed() {
        return;
    }
    let Ok((mut text, mut classes)) = query.single_mut() else {
        return;
    };
    if search.query.is_empty() {
        text.0 = "Type fish name...".to_string();
        classes.add("placeholder");
    } else {
        text.0 = search.query.clone();
        classes.remove("placeholder");
    }
}

pub(super) fn sync_search_tags(
    search: Res<SearchState>,
    fish: Res<FishCatalog>,
    remote_image_epoch: Res<RemoteImageEpoch>,
    mut remote_images: ResMut<RemoteImageCache>,
    fonts: Res<UiFonts>,
    mut commands: Commands,
    mut tags_q: Query<
        (Entity, Option<&Children>, &mut Visibility, &mut Node),
        With<FishSearchTags>,
    >,
) {
    if !search.is_changed() && !fish.is_changed() && !remote_image_epoch.is_changed() {
        return;
    }
    let Ok((tags_entity, children, mut visibility, mut node)) = tags_q.single_mut() else {
        return;
    };

    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    if search.selected_fish_ids.is_empty() {
        *visibility = Visibility::Hidden;
        node.display = Display::None;
        return;
    }

    *visibility = Visibility::Visible;
    node.display = Display::Flex;
    let chip_style = text_style(11.0, Color::srgb(0.90, 0.92, 0.96), fonts.regular.clone());
    commands.entity(tags_entity).with_children(|tags| {
        for fish_id in &search.selected_fish_ids {
            let entry = fish
                .entries
                .iter()
                .find(|entry| entry.id == *fish_id)
                .cloned();
            let label = entry
                .as_ref()
                .map(|entry| entry.name.clone())
                .unwrap_or_else(|| format!("Fish {fish_id}"));
            let icon_handle = entry
                .as_ref()
                .and_then(|entry| fish_icon_handle(entry.item_id, &mut remote_images));
            tags.spawn((
                FishSearchTag { fish_id: *fish_id },
                Button,
                Node {
                    height: Val::Px(24.0),
                    padding: UiRect::horizontal(Val::Px(8.0)),
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(999.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(
                    120.0 / 255.0,
                    180.0 / 255.0,
                    255.0 / 255.0,
                    0.14,
                )),
                BorderColor::all(Color::srgba(
                    120.0 / 255.0,
                    180.0 / 255.0,
                    255.0 / 255.0,
                    0.42,
                )),
                ClassList::new("search-tag"),
            ))
            .with_children(|tag| {
                if let Some(icon_handle) = icon_handle.clone() {
                    tag.spawn((
                        ImageNode::new(icon_handle),
                        Node {
                            width: Val::Px(14.0),
                            height: Val::Px(14.0),
                            ..default()
                        },
                        ClassList::new("search-tag-icon"),
                    ));
                }
                tag.spawn((
                    UiTextBundle::new(format!("{label} ×"), &chip_style),
                    ClassList::new("search-tag-text"),
                ));
            });
        }
    });
}

pub(super) fn update_autocomplete_ui(
    search: Res<SearchState>,
    fish: Res<FishCatalog>,
    remote_image_epoch: Res<RemoteImageEpoch>,
    mut remote_images: ResMut<RemoteImageCache>,
    mut frame_q: Query<
        (&mut Visibility, &mut Node),
        (
            With<FishAutocompleteFrame>,
            Without<FishAutocompleteEntry>,
            Without<FishAutocompleteEntryIcon>,
        ),
    >,
    mut scroll_q: Query<&mut ScrollPosition, With<FishAutocompleteScroll>>,
    mut entry_q: Query<
        (
            &FishAutocompleteEntry,
            &mut Visibility,
            &mut Node,
            &mut ClassList,
            &Children,
        ),
        (
            Without<FishAutocompleteList>,
            Without<FishAutocompleteEntryIcon>,
        ),
    >,
    mut text_q: Query<&mut Text>,
    mut icon_q: Query<
        (&mut ImageNode, &mut Visibility),
        (
            With<FishAutocompleteEntryIcon>,
            Without<FishAutocompleteEntry>,
            Without<FishAutocompleteFrame>,
        ),
    >,
) {
    if !search.is_changed() && !fish.is_changed() && !remote_image_epoch.is_changed() {
        return;
    }
    let open = search.open && !search.results.is_empty();
    if let Ok((mut vis, mut node)) = frame_q.single_mut() {
        *vis = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        node.display = if open { Display::Flex } else { Display::None };
    }
    if !open {
        if let Ok(mut scroll) = scroll_q.single_mut() {
            scroll.0.y = 0.0;
        }
    }
    for (entry, mut vis, mut node, mut classes, children) in &mut entry_q {
        if !open || entry.idx >= search.results.len() {
            *vis = Visibility::Hidden;
            node.display = Display::None;
            continue;
        }
        *vis = Visibility::Visible;
        node.display = Display::Flex;
        let fish_idx = search.results[entry.idx];
        let fish_entry = fish.entries.get(fish_idx);
        let label = fish_entry
            .map(|f| format!("{} (#{})", f.name, f.id))
            .unwrap_or_else(|| "(unknown)".to_string());
        for child in children.iter() {
            if let Ok((mut icon, mut icon_vis)) = icon_q.get_mut(child) {
                if let Some(fish_entry) = fish_entry {
                    if let Some(handle) = fish_icon_handle(fish_entry.item_id, &mut remote_images) {
                        *icon = ImageNode::new(handle);
                        *icon_vis = Visibility::Visible;
                    } else {
                        *icon_vis = Visibility::Hidden;
                    }
                } else {
                    *icon_vis = Visibility::Hidden;
                }
                continue;
            }
            if let Ok(mut text) = text_q.get_mut(child) {
                text.0 = label;
                break;
            }
        }
        if entry.idx == search.selected {
            classes.add("selected");
        } else {
            classes.remove("selected");
        }
    }
}

pub(super) fn rebuild_results(search: &mut SearchState, fish: &FishCatalog) {
    search.results.clear();
    if search.query.trim().is_empty() {
        search.open = false;
        return;
    }
    let query = search.query.to_lowercase();
    let mut scored: Vec<(i64, usize)> = Vec::new();
    for (idx, entry) in fish.entries.iter().enumerate() {
        if search.selected_fish_ids.contains(&entry.id) {
            continue;
        }
        if let Some(pos) = entry.name_lower.find(&query) {
            let mut score = pos as i64 * 10 + entry.name_lower.len() as i64;
            if pos == 0 {
                score -= 1000;
            }
            scored.push((score, idx));
        }
    }
    scored.sort_by_key(|(score, idx)| (*score, *idx));
    for (_, idx) in scored.into_iter().take(AUTOCOMPLETE_MAX) {
        search.results.push(idx);
    }
    if search.selected >= search.results.len() {
        search.selected = search.results.len().saturating_sub(1);
    }
    search.open = !search.results.is_empty();
}

pub(super) fn apply_fish_selection(
    idx: usize,
    fish: &FishCatalog,
    fish_filter: &mut FishFilterState,
    search: &mut SearchState,
) {
    if let Some(entry) = fish.entries.get(idx) {
        if !search.selected_fish_ids.contains(&entry.id) {
            search.selected_fish_ids.push(entry.id);
        }
        search.query.clear();
        apply_search_filters_to_ui(search, fish, fish_filter);
        rebuild_results(search, fish);
        search.open = false;
    }
}

pub(super) fn apply_search_filters_to_ui(
    search: &SearchState,
    fish: &FishCatalog,
    fish_filter: &mut FishFilterState,
) {
    fish_filter.selected_fish_ids = search.selected_fish_ids.clone();
    if let Some(last_id) = search.selected_fish_ids.last().copied() {
        fish_filter.selected_fish = Some(last_id);
        fish_filter.selected_fish_name = fish
            .entries
            .iter()
            .find(|entry| entry.id == last_id)
            .map(|entry| entry.name.clone())
            .or_else(|| Some(format!("Fish {last_id}")));
    } else {
        fish_filter.selected_fish = None;
        fish_filter.selected_fish_name = None;
    }
}
