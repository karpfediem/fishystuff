use std::collections::HashSet;

use bevy::prelude::*;
use fishystuff_api::models::events::EventPointCompact;

use crate::map::events::EventsSnapshotState;
use crate::plugins::api::{FishFilterState, PatchFilterState};

use super::{normalized_time_and_fish_filters, EvidenceZoneFilter};

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

pub(in crate::plugins::points) fn sync_evidence_zone_filter(
    patch_filter: Res<PatchFilterState>,
    fish_filter: Res<FishFilterState>,
    snapshot: Res<EventsSnapshotState>,
    mut filter: ResMut<EvidenceZoneFilter>,
) {
    let active_terms = !fish_filter.selected_fish_ids.is_empty();
    if !active_terms {
        if filter.active || !filter.zone_rgbs.is_empty() {
            filter.active = false;
            filter.zone_rgbs.clear();
            filter.revision = filter.revision.wrapping_add(1);
        }
        return;
    }

    let Some((from_ts_utc, to_ts_utc, mut fish_ids)) =
        normalized_time_and_fish_filters(&patch_filter, &fish_filter)
    else {
        return;
    };
    fish_ids.sort_unstable();
    fish_ids.dedup();

    if !snapshot.loaded {
        return;
    }

    let (zones, has_zone_data, matched_events) =
        collect_evidence_zone_rgbs(&snapshot.events, from_ts_utc, to_ts_utc, &fish_ids);
    if !has_zone_data && matched_events > 0 {
        if filter.active || !filter.zone_rgbs.is_empty() {
            filter.active = false;
            filter.zone_rgbs.clear();
            filter.revision = filter.revision.wrapping_add(1);
        }
        return;
    }

    if !filter.active || filter.zone_rgbs != zones {
        filter.active = true;
        filter.zone_rgbs = zones;
        filter.revision = filter.revision.wrapping_add(1);
    }
}
