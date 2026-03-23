use super::setup::text_style;
use super::*;
use crate::map::interaction_summary::{
    selection_heading, selection_overview_lines, selection_summary_text,
};
use bevy::ecs::system::SystemParam;
pub(super) fn update_selected_text(
    selection: Res<SelectionState>,
    mut query: Query<&mut Text, With<SelectionSummaryText>>,
) {
    if !selection.is_changed() {
        return;
    }
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    if let Some(info) = &selection.info {
        text.0 = selection_summary_text(info);
    } else {
        text.0 = "No selection.".to_string();
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
        selection_heading(info)
            .map(|heading| heading.value)
            .unwrap_or_else(|| "Selection".to_string())
    } else {
        "FishyStuff Map".to_string()
    };
}

pub(super) fn sync_selection_overview_list(
    selection: Res<SelectionState>,
    fonts: Res<UiFonts>,
    mut commands: Commands,
    list_q: Query<(Entity, Option<&Children>), With<SelectionOverviewList>>,
) {
    let Ok((list_entity, children)) = list_q.single() else {
        return;
    };
    let list_is_empty = children.map(|children| children.is_empty()).unwrap_or(true);
    if !selection.is_changed() && !list_is_empty {
        return;
    }
    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    let style = text_style(12.0, Color::srgb(0.82, 0.84, 0.90), fonts.regular.clone());
    let lines = selection
        .info
        .as_ref()
        .map(selection_overview_lines)
        .unwrap_or_default();
    if lines.is_empty() {
        return;
    }
    commands.entity(list_entity).with_children(|list| {
        for line in lines {
            list.spawn((
                UiTextBundle::new(line, &style),
                Node {
                    width: Val::Percent(100.0),
                    ..default()
                },
                ClassList::new("label selection-overview-item"),
            ));
        }
    });
}

pub(super) fn sync_zone_evidence_list(mut sync: ZoneEvidenceListSync<'_, '_>) {
    let Ok((list_entity, children)) = sync.list_q.single() else {
        return;
    };
    let list_is_empty = children.map(|c| c.is_empty()).unwrap_or(true);
    let selected_fish_changed = *sync.last_selected_fish_ids != sync.fish_filter.selected_fish_ids;
    if !sync.selection.is_changed()
        && !sync.fish.is_changed()
        && !selected_fish_changed
        && !sync.remote_image_epoch.is_changed()
        && !list_is_empty
    {
        return;
    }
    *sync.last_selected_fish_ids = sync.fish_filter.selected_fish_ids.clone();

    if let Some(children) = children {
        for child in children.iter() {
            sync.commands.entity(child).despawn();
        }
    }

    let title_style = text_style(
        12.0,
        Color::srgb(0.90, 0.90, 0.94),
        sync.fonts.regular.clone(),
    );
    let meta_style = text_style(
        11.0,
        Color::srgb(0.70, 0.72, 0.78),
        sync.fonts.regular.clone(),
    );

    let rows = if let Some(stats) = sync.selection.zone_stats.as_ref() {
        if stats.distribution.is_empty() {
            Vec::new()
        } else {
            stats
                .distribution
                .iter()
                .map(|entry| {
                    let name = resolve_zone_fish_name(
                        entry.fish_id,
                        entry.fish_name.as_deref(),
                        &sync.fish,
                    );
                    let ci = match (entry.ci_low, entry.ci_high) {
                        (Some(low), Some(high)) => format!("{low:.3}-{high:.3}"),
                        _ => "n/a".to_string(),
                    };
                    let selected = sync.fish_filter.selected_fish_ids.contains(&entry.fish_id);
                    let icon_handle = fish_icon_handle(entry.item_id, &mut sync.remote_images);
                    (
                        selected,
                        icon_handle,
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

    let placeholder = if sync.selection.zone_stats.is_none() {
        if sync.selection.info.is_some() {
            if sync
                .selection
                .info
                .as_ref()
                .and_then(crate::plugins::api::SelectedInfo::zone_rgb_u32)
                .is_some()
            {
                "No zone evidence loaded."
            } else {
                "Zone evidence is only available for zone-backed selections."
            }
        } else {
            "Select a zone-backed result to inspect fish evidence."
        }
    } else if rows.is_empty() {
        "No fish evidence in this window."
    } else {
        ""
    };

    sync.commands.entity(list_entity).with_children(|list| {
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

        for (selected, icon_handle, title, meta) in rows {
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
                    if let Some(icon_handle) = icon_handle.clone() {
                        row.spawn((
                            ImageNode::new(icon_handle),
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

#[derive(SystemParam)]
pub(super) struct ZoneEvidenceListSync<'w, 's> {
    selection: Res<'w, SelectionState>,
    fish_filter: Res<'w, FishFilterState>,
    fish: Res<'w, FishCatalog>,
    remote_image_epoch: Res<'w, RemoteImageEpoch>,
    remote_images: ResMut<'w, RemoteImageCache>,
    fonts: Res<'w, UiFonts>,
    last_selected_fish_ids: Local<'s, Vec<i32>>,
    commands: Commands<'w, 's>,
    list_q: Query<'w, 's, (Entity, Option<&'static Children>), With<ZoneEvidenceList>>,
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
