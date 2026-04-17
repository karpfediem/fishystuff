use super::super::*;
use crate::plugins::api::Patch;
use crate::plugins::ui::patch_index_for_timestamp;
use chrono::{Days, NaiveDate};

fn patch_idx(ordered: &[&Patch], patch_id: &str) -> Option<usize> {
    ordered
        .iter()
        .position(|patch| patch.patch_id.0 == patch_id)
}

fn patch_range_end_ts(ordered: &[&Patch], idx: usize) -> i64 {
    if idx + 1 < ordered.len() {
        ordered[idx + 1].start_ts_utc.saturating_sub(1)
    } else {
        now_utc_seconds()
    }
}

fn parse_iso_date_day_start_ts(value: &str) -> Option<i64> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .ok()?
        .and_hms_opt(0, 0, 0)
        .map(|date_time| date_time.and_utc().timestamp())
}

fn parse_iso_date_day_end_ts(value: &str) -> Option<i64> {
    let next_day = NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .ok()?
        .checked_add_days(Days::new(1))?;
    next_day
        .and_hms_opt(0, 0, 0)
        .map(|date_time| date_time.and_utc().timestamp().saturating_sub(1))
}

pub(in crate::bridge::host) fn apply_patch_range_override(
    patch_filter: &mut PatchFilterState,
    from_patch_id: Option<&str>,
    to_patch_id: Option<&str>,
) {
    let mut ordered = patch_filter.patches.iter().collect::<Vec<_>>();
    if ordered.is_empty() {
        if from_patch_id.is_none() && to_patch_id.is_none() {
            patch_filter.selected_patch = None;
            patch_filter.from_ts = None;
            patch_filter.to_ts = None;
        }
        return;
    }
    if from_patch_id.is_none() && to_patch_id.is_none() {
        patch_filter.selected_patch = None;
        patch_filter.from_ts = None;
        patch_filter.to_ts = None;
        return;
    }
    ordered.sort_by_key(|patch| patch.start_ts_utc);
    let resolved_from_idx = from_patch_id.and_then(|patch_id| patch_idx(&ordered, patch_id));
    let resolved_to_idx = to_patch_id.and_then(|patch_id| patch_idx(&ordered, patch_id));
    let from_ts = resolved_from_idx
        .map(|idx| ordered[idx].start_ts_utc)
        .or_else(|| from_patch_id.and_then(parse_iso_date_day_start_ts))
        .unwrap_or(ordered[0].start_ts_utc);
    let mut to_ts = resolved_to_idx
        .map(|idx| patch_range_end_ts(&ordered, idx))
        .or_else(|| to_patch_id.and_then(parse_iso_date_day_end_ts))
        .unwrap_or_else(|| patch_range_end_ts(&ordered, ordered.len() - 1));
    if to_ts < from_ts {
        to_ts = from_ts;
    }
    let next_selected_patch = resolved_from_idx.map(|idx| ordered[idx].patch_id.0.clone());
    if patch_filter.selected_patch == next_selected_patch
        && patch_filter.from_ts == Some(from_ts)
        && patch_filter.to_ts == Some(to_ts)
    {
        return;
    }
    patch_filter.selected_patch = next_selected_patch;
    patch_filter.from_ts = Some(from_ts);
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

#[cfg(test)]
mod tests {
    use super::{apply_patch_range_override, patch_range_end_ts};
    use crate::plugins::api::{Patch, PatchFilterState};
    use chrono::NaiveDate;
    use fishystuff_api::ids::PatchId;
    use fishystuff_api::models::meta::PatchInfo;

    fn patch(patch_id: &str, start_ts_utc: i64, patch_name: Option<&str>) -> Patch {
        PatchInfo {
            patch_id: PatchId(patch_id.to_string()),
            start_ts_utc,
            patch_name: patch_name.map(str::to_string),
        }
    }

    fn unix_day_start(value: &str) -> i64 {
        NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp()
    }

    #[test]
    fn apply_patch_range_override_supports_manual_iso_date_bounds() {
        let mut patch_filter = PatchFilterState {
            patches: vec![
                patch("2026-03-12", unix_day_start("2026-03-12"), Some("New Era")),
                patch(
                    "2026-04-24",
                    unix_day_start("2026-04-24"),
                    Some("Silver Tide"),
                ),
            ],
            ..PatchFilterState::default()
        };

        apply_patch_range_override(&mut patch_filter, Some("2026-04-16"), Some("2026-04-20"));

        assert_eq!(patch_filter.selected_patch, None);
        assert_eq!(patch_filter.from_ts, Some(unix_day_start("2026-04-16")));
        assert_eq!(
            patch_filter.to_ts,
            Some(unix_day_start("2026-04-21").saturating_sub(1))
        );
    }

    #[test]
    fn apply_patch_range_override_supports_mixed_patch_and_manual_date_bounds() {
        let mut patch_filter = PatchFilterState {
            patches: vec![
                patch("2026-03-12", unix_day_start("2026-03-12"), Some("New Era")),
                patch(
                    "2026-04-24",
                    unix_day_start("2026-04-24"),
                    Some("Silver Tide"),
                ),
            ],
            ..PatchFilterState::default()
        };
        let ordered = patch_filter.patches.iter().collect::<Vec<_>>();

        apply_patch_range_override(&mut patch_filter, Some("2026-03-12"), Some("2026-04-20"));

        assert_eq!(patch_filter.selected_patch.as_deref(), Some("2026-03-12"));
        assert_eq!(patch_filter.from_ts, Some(unix_day_start("2026-03-12")));
        assert_eq!(
            patch_filter.to_ts,
            Some(unix_day_start("2026-04-21").saturating_sub(1))
        );

        apply_patch_range_override(&mut patch_filter, Some("2026-04-16"), Some("2026-04-24"));

        assert_eq!(patch_filter.selected_patch, None);
        assert_eq!(patch_filter.from_ts, Some(unix_day_start("2026-04-16")));
        assert_eq!(patch_filter.to_ts, Some(patch_range_end_ts(&ordered, 1)));
    }

    #[test]
    fn apply_patch_range_override_clears_existing_selection_when_bounds_are_removed() {
        let mut patch_filter = PatchFilterState {
            patches: vec![
                patch("2026-03-12", unix_day_start("2026-03-12"), Some("New Era")),
                patch(
                    "2026-04-24",
                    unix_day_start("2026-04-24"),
                    Some("Silver Tide"),
                ),
            ],
            from_ts: Some(unix_day_start("2026-03-12")),
            to_ts: Some(unix_day_start("2026-04-25").saturating_sub(1)),
            selected_patch: Some("2026-03-12".to_string()),
        };

        apply_patch_range_override(&mut patch_filter, None, None);

        assert_eq!(patch_filter.selected_patch, None);
        assert_eq!(patch_filter.from_ts, None);
        assert_eq!(patch_filter.to_ts, None);
    }
}
