use super::super::*;
use crate::plugins::ui::patch_index_for_timestamp;

pub(in crate::bridge::host) fn apply_patch_range_override(
    patch_filter: &mut PatchFilterState,
    from_patch_id: Option<&str>,
    to_patch_id: Option<&str>,
) {
    let mut ordered = patch_filter.patches.iter().collect::<Vec<_>>();
    if ordered.is_empty() || (from_patch_id.is_none() && to_patch_id.is_none()) {
        return;
    }
    ordered.sort_by_key(|patch| patch.start_ts_utc);
    let from_idx = from_patch_id
        .and_then(|patch_id| {
            ordered
                .iter()
                .position(|patch| patch.patch_id.0 == patch_id)
        })
        .unwrap_or(0);
    let mut to_idx = to_patch_id
        .and_then(|patch_id| {
            ordered
                .iter()
                .position(|patch| patch.patch_id.0 == patch_id)
        })
        .unwrap_or(ordered.len() - 1);
    if to_idx < from_idx {
        to_idx = from_idx;
    }
    let from_patch = ordered[from_idx];
    let to_ts = if to_idx + 1 < ordered.len() {
        ordered[to_idx + 1].start_ts_utc.saturating_sub(1)
    } else {
        now_utc_seconds()
    };
    patch_filter.selected_patch = Some(from_patch.patch_id.0.clone());
    patch_filter.from_ts = Some(from_patch.start_ts_utc);
    patch_filter.to_ts = Some(to_ts);
}

pub(in crate::bridge::host) fn current_patch_range_ids(
    patch_filter: &PatchFilterState,
) -> (Option<String>, Option<String>) {
    let mut ordered = patch_filter.patches.iter().collect::<Vec<_>>();
    if ordered.is_empty() {
        return (None, None);
    }
    ordered.sort_by_key(|patch| patch.start_ts_utc);

    let from_idx = patch_filter
        .from_ts
        .map(|from_ts| patch_index_for_timestamp(&ordered, from_ts))
        .or_else(|| {
            patch_filter.selected_patch.as_deref().and_then(|patch_id| {
                ordered
                    .iter()
                    .position(|patch| patch.patch_id.0 == patch_id)
            })
        });
    let Some(from_idx) = from_idx else {
        return (None, None);
    };
    let mut to_idx = patch_filter
        .to_ts
        .map(|to_ts| patch_index_for_timestamp(&ordered, to_ts))
        .unwrap_or(ordered.len() - 1);
    if to_idx < from_idx {
        to_idx = from_idx;
    }

    (
        Some(ordered[from_idx].patch_id.0.clone()),
        Some(ordered[to_idx].patch_id.0.clone()),
    )
}
