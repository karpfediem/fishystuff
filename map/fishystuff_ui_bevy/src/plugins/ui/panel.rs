use super::setup::text_style;
use super::*;
pub(super) fn update_selected_text(
    selection: Res<SelectionState>,
    mut query: Query<&mut Text, With<SelectedZoneText>>,
) {
    if !selection.is_changed() {
        return;
    }
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    if let Some(info) = &selection.info {
        text.0 = format!("RGB: {},{}, {}", info.rgb.0, info.rgb.1, info.rgb.2);
    } else {
        text.0 = "RGB: (none)".to_string();
    }
}

pub(super) fn update_panel_title(
    selection: Res<SelectionState>,
    mut query: Query<&mut Text, With<PanelTitleText>>,
) {
    if !selection.is_changed() {
        return;
    }
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    text.0 = if let Some(info) = &selection.info {
        info.zone_name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("(unknown zone)")
            .to_string()
    } else {
        "FishyStuff Zones".to_string()
    };
}

pub(super) fn sync_zone_evidence_list(
    selection: Res<SelectionState>,
    fish_filter: Res<FishFilterState>,
    fish: Res<FishCatalog>,
    fonts: Res<UiFonts>,
    asset_server: Res<AssetServer>,
    mut last_selected_fish: Local<Option<i32>>,
    mut commands: Commands,
    list_q: Query<(Entity, Option<&Children>), With<ZoneEvidenceList>>,
) {
    let Ok((list_entity, children)) = list_q.single() else {
        return;
    };
    let list_is_empty = children.map(|c| c.is_empty()).unwrap_or(true);
    let selected_fish_changed = *last_selected_fish != fish_filter.selected_fish;
    if !selection.is_changed() && !fish.is_changed() && !selected_fish_changed && !list_is_empty {
        return;
    }
    *last_selected_fish = fish_filter.selected_fish;

    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    let title_style = text_style(12.0, Color::srgb(0.90, 0.90, 0.94), fonts.regular.clone());
    let meta_style = text_style(11.0, Color::srgb(0.70, 0.72, 0.78), fonts.regular.clone());

    let rows = if let Some(stats) = selection.zone_stats.as_ref() {
        if stats.distribution.is_empty() {
            Vec::new()
        } else {
            stats
                .distribution
                .iter()
                .map(|entry| {
                    let name =
                        resolve_zone_fish_name(entry.fish_id, entry.fish_name.as_deref(), &fish);
                    let ci = match (entry.ci_low, entry.ci_high) {
                        (Some(low), Some(high)) => format!("{low:.3}-{high:.3}"),
                        _ => "n/a".to_string(),
                    };
                    let selected = fish_filter.selected_fish == Some(entry.fish_id);
                    let icon_path = fish
                        .icon_url_for_fish(entry.fish_id)
                        .or_else(|| entry.icon_url.clone());
                    (
                        selected,
                        icon_path,
                        format!("{name}  #{id}", id = entry.fish_id),
                        format!(
                            "p {p:.3} · weight {w:.3} · ci {ci}",
                            p = entry.p_mean,
                            w = entry.evidence_weight
                        ),
                    )
                })
                .collect::<Vec<_>>()
        }
    } else {
        Vec::new()
    };

    let placeholder = if selection.zone_stats.is_none() {
        if selection.info.is_some() {
            "No zone evidence loaded."
        } else {
            "Click a zone on the map to load evidence."
        }
    } else if rows.is_empty() {
        "No fish evidence in this window."
    } else {
        ""
    };

    commands.entity(list_entity).with_children(|list| {
        if !placeholder.is_empty() {
            list.spawn((
                UiTextBundle::new(placeholder, &meta_style),
                Node {
                    width: Val::Percent(100.0),
                    ..default()
                },
                ClassList::new("label fish-empty"),
            ));
            return;
        }

        for (selected, icon_path, title, meta) in rows {
            let mut classes = ClassList::new("list-item zone-evidence-item");
            if selected {
                classes.add("selected");
            }
            list.spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(6.0)),
                    row_gap: Val::Px(2.0),
                    flex_direction: FlexDirection::Column,
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.001)),
                classes,
            ))
            .with_children(|item| {
                item.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    },
                    ClassList::new("row zone-evidence-row"),
                ))
                .with_children(|row| {
                    if let Some(icon_path) = icon_path.clone() {
                        row.spawn((
                            ImageNode::new(asset_server.load(icon_path)),
                            Node {
                                width: Val::Px(18.0),
                                height: Val::Px(18.0),
                                ..default()
                            },
                            ClassList::new("zone-evidence-icon"),
                        ));
                    }
                    row.spawn((
                        UiTextBundle::new(title, &title_style),
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                        ClassList::new("fish-name"),
                    ));
                });
                item.spawn((
                    UiTextBundle::new(meta, &meta_style),
                    ClassList::new("label"),
                ));
            });
        }
    });
}

pub(super) fn resolve_zone_fish_name(
    fish_id: i32,
    fish_name: Option<&str>,
    fish: &FishCatalog,
) -> String {
    if let Some(entry) = fish.entries.iter().find(|entry| entry.id == fish_id) {
        return entry.name.clone();
    }
    if let Some(name) = fish_name {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    format!("Fish {fish_id}")
}
