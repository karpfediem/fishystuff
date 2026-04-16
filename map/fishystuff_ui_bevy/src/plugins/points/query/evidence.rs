use std::collections::HashSet;

use bevy::prelude::*;
use fishystuff_api::models::events::EventPointCompact;

use crate::map::events::EventsSnapshotState;
use crate::plugins::api::{FishFilterState, PatchFilterState, SemanticFieldFilterState};

use super::{normalized_time_and_fish_filters, EvidenceZoneFilter};

fn apply_zone_filter_state(
    filter: &mut EvidenceZoneFilter,
    next_active: bool,
    next_zone_rgbs: HashSet<u32>,
) {
    if filter.active != next_active || filter.zone_rgbs != next_zone_rgbs {
        filter.active = next_active;
        filter.zone_rgbs = next_zone_rgbs;
        filter.revision = filter.revision.wrapping_add(1);
    }
}

pub(super) fn merge_zone_terms(
    explicit_zones: &HashSet<u32>,
    evidence_zones: HashSet<u32>,
    has_zone_data: bool,
    matched_events: usize,
) -> HashSet<u32> {
    if explicit_zones.is_empty() {
        return evidence_zones;
    }
    if !has_zone_data && matched_events > 0 {
        return explicit_zones.clone();
    }
    evidence_zones
        .into_iter()
        .filter(|zone_rgb| explicit_zones.contains(zone_rgb))
        .collect()
}

pub(super) fn collect_evidence_zone_rgbs(
    events: &[EventPointCompact],
    from_ts_utc: i64,
    to_ts_utc: i64,
    fish_ids: &[i32],
) -> (HashSet<u32>, bool, usize) {
    let mut zones = HashSet::new();
    let mut has_zone_data = false;
    let mut matched_events = 0usize;

    for event in events {
        if event.ts_utc < from_ts_utc || event.ts_utc >= to_ts_utc {
            continue;
        }
        if !fish_ids.is_empty() && fish_ids.binary_search(&event.fish_id).is_err() {
            continue;
        }
        matched_events = matched_events.saturating_add(1);
        if let Some(zone_rgb) = event.zone_rgb_u32 {
            has_zone_data = true;
            zones.insert(zone_rgb);
        }
    }

    (zones, has_zone_data, matched_events)
}

pub(crate) fn sync_evidence_zone_filter(
    patch_filter: Res<PatchFilterState>,
    fish_filter: Res<FishFilterState>,
    semantic_filter: Res<SemanticFieldFilterState>,
    snapshot: Res<EventsSnapshotState>,
    mut filter: ResMut<EvidenceZoneFilter>,
) {
    let explicit_zones = semantic_filter
        .selected_zone_rgbs()
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let active_terms = !fish_filter.selected_fish_ids.is_empty();
    if !active_terms {
        apply_zone_filter_state(&mut filter, !explicit_zones.is_empty(), explicit_zones);
        return;
    }

    let Some((from_ts_utc, to_ts_utc, mut fish_ids)) =
        normalized_time_and_fish_filters(&patch_filter, &fish_filter)
    else {
        if !explicit_zones.is_empty() {
            apply_zone_filter_state(&mut filter, true, explicit_zones);
        }
        return;
    };
    fish_ids.sort_unstable();
    fish_ids.dedup();

    if !snapshot.loaded {
        if !explicit_zones.is_empty() {
            apply_zone_filter_state(&mut filter, true, explicit_zones);
        }
        return;
    }

    let (evidence_zones, has_zone_data, matched_events) =
        collect_evidence_zone_rgbs(&snapshot.events, from_ts_utc, to_ts_utc, &fish_ids);
    let next_zone_rgbs = merge_zone_terms(
        &explicit_zones,
        evidence_zones,
        has_zone_data,
        matched_events,
    );
    apply_zone_filter_state(&mut filter, !next_zone_rgbs.is_empty(), next_zone_rgbs);
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::merge_zone_terms;

    fn zone_set(values: &[u32]) -> HashSet<u32> {
        values.iter().copied().collect()
    }

    #[test]
    fn merge_zone_terms_intersects_explicit_and_evidence_zones() {
        assert_eq!(
            merge_zone_terms(
                &zone_set(&[0x111111, 0x222222]),
                zone_set(&[0x222222, 0x333333]),
                true,
                2
            ),
            zone_set(&[0x222222]),
        );
    }

    #[test]
    fn merge_zone_terms_keeps_explicit_zones_when_events_lack_zone_data() {
        assert_eq!(
            merge_zone_terms(&zone_set(&[0x111111, 0x222222]), HashSet::new(), false, 3),
            zone_set(&[0x111111, 0x222222]),
        );
    }

    #[test]
    fn merge_zone_terms_returns_evidence_zones_when_no_explicit_zone_filter_exists() {
        assert_eq!(
            merge_zone_terms(&HashSet::new(), zone_set(&[0x222222, 0x333333]), true, 2),
            zone_set(&[0x222222, 0x333333]),
        );
    }
}
