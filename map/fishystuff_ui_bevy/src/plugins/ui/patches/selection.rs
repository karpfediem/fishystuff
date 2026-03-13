use super::super::*;

pub(in crate::plugins::ui) fn sync_patch_defaults(
    mut patch_filter: ResMut<PatchFilterState>,
    mut state: ResMut<PatchDropdownState>,
) {
    if !LEGACY_PATCH_UI_ENABLED {
        return;
    }
    if !patch_filter.is_changed() && !state.is_changed() {
        return;
    }
    if patch_filter.patches.is_empty() {
        if state.from_patch_id.is_some() || state.to_patch_id.is_some() {
            state.from_patch_id = None;
            state.to_patch_id = None;
        }
        return;
    }
    normalize_patch_selection(&mut patch_filter, &mut state);
}

pub(in crate::plugins::ui) fn update_patch_button_texts(
    patch_filter: Res<PatchFilterState>,
    state: Res<PatchDropdownState>,
    mut query: Query<(&PatchRangeButtonText, &mut Text)>,
) {
    if !patch_filter.is_changed() && !state.is_changed() {
        return;
    }
    for (button, mut text) in &mut query {
        let selected_id = match button.bound {
            PatchBound::From => state.from_patch_id.as_deref(),
            PatchBound::To => state.to_patch_id.as_deref(),
        };
        let label = selected_id
            .and_then(|id| find_patch_name(&patch_filter.patches, id))
            .unwrap_or_else(|| "(select patch)".to_string());
        text.0 = match button.bound {
            PatchBound::From => format!("From: {}", label),
            PatchBound::To => format!("To (incl): {}", label),
        };
    }
}

pub(in crate::plugins::ui) fn sync_patch_entry_selection(
    state: Res<PatchDropdownState>,
    mut query: Query<(&PatchEntry, &mut ClassList)>,
) {
    if !state.is_changed() {
        return;
    }
    for (entry, mut classes) in &mut query {
        let selected = match entry.bound {
            PatchBound::From => state.from_patch_id.as_deref(),
            PatchBound::To => state.to_patch_id.as_deref(),
        };
        if selected == Some(entry.patch_id.as_str()) {
            classes.add("selected");
        } else {
            classes.remove("selected");
        }
    }
}

pub(crate) fn patch_list_hash(patches: &[Patch]) -> u64 {
    let mut hash = 14695981039346656037u64;
    for patch in patches {
        for byte in patch.patch_id.0.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(1099511628211);
        }
    }
    hash
}

pub(crate) fn patch_name(patch: &Patch) -> String {
    patch
        .patch_name
        .clone()
        .unwrap_or_else(|| patch.patch_id.0.clone())
}

pub(crate) fn find_patch_name(patches: &[Patch], id: &str) -> Option<String> {
    patches.iter().find(|p| p.patch_id.0 == id).map(patch_name)
}

pub(crate) fn display_patches(patches: &[Patch]) -> Vec<&Patch> {
    let mut list = patches.iter().collect::<Vec<_>>();
    list.sort_by_key(|p| p.start_ts_utc);
    list.reverse();
    list
}

pub(crate) fn normalize_patch_selection(
    patch_filter: &mut PatchFilterState,
    state: &mut PatchDropdownState,
) {
    let mut ordered = patch_filter.patches.iter().collect::<Vec<_>>();
    if ordered.is_empty() {
        return;
    }
    ordered.sort_by_key(|p| p.start_ts_utc);

    let from_idx = state
        .from_patch_id
        .as_deref()
        .and_then(|id| ordered.iter().position(|p| p.patch_id.0 == id))
        .unwrap_or(0);

    let mut to_idx = state
        .to_patch_id
        .as_deref()
        .and_then(|id| ordered.iter().position(|p| p.patch_id.0 == id))
        .or_else(|| {
            patch_filter
                .to_ts
                .map(|ts| patch_index_for_timestamp(&ordered, ts))
        })
        .unwrap_or(ordered.len() - 1);

    if to_idx < from_idx {
        to_idx = from_idx;
    }

    let from_patch = ordered[from_idx];
    let to_patch = ordered[to_idx];
    let to_ts = if to_idx + 1 < ordered.len() {
        ordered[to_idx + 1].start_ts_utc.saturating_sub(1)
    } else {
        crate::plugins::api::now_utc_seconds()
    };

    let from_patch_id = from_patch.patch_id.0.clone();
    let to_patch_id = to_patch.patch_id.0.clone();
    // Avoid rewriting every frame (especially for the latest patch where `to_ts` uses now).
    // Once selection is stable, keep the previously computed to_ts until selection changes.
    let selection_stable = state.from_patch_id.as_deref() == Some(from_patch_id.as_str())
        && state.to_patch_id.as_deref() == Some(to_patch_id.as_str())
        && patch_filter.selected_patch.as_deref() == Some(from_patch_id.as_str())
        && patch_filter.from_ts == Some(from_patch.start_ts_utc)
        && patch_filter.to_ts.is_some();
    if selection_stable {
        return;
    }

    state.from_patch_id = Some(from_patch_id.clone());
    state.to_patch_id = Some(to_patch_id);
    patch_filter.selected_patch = Some(from_patch_id);
    patch_filter.from_ts = Some(from_patch.start_ts_utc);
    patch_filter.to_ts = Some(to_ts);
}

pub(crate) fn patch_index_for_timestamp(ordered: &[&Patch], ts: i64) -> usize {
    let mut idx = 0usize;
    for (i, patch) in ordered.iter().enumerate() {
        if patch.start_ts_utc <= ts {
            idx = i;
        } else {
            break;
        }
    }
    idx
}
