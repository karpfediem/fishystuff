use std::collections::HashSet;

use bevy::prelude::*;
use fishystuff_api::models::events::EventPointCompact;

use crate::map::events::EventsSnapshotState;
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_view::{loaded_field_layer, FieldLayerView, LoadedFieldLayer};
use crate::map::layers::LayerRegistry;
use crate::plugins::api::{
    FishFilterState, LayerEffectiveFilterState, LayerFilterBindingOverrideState, PatchFilterState,
    SemanticFieldFilterState,
};

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

pub(super) fn collect_evidence_zone_rgbs(
    events: &[EventPointCompact],
    from_ts_utc: i64,
    to_ts_utc: i64,
    fish_ids: &[i32],
    zone_mask_field: Option<LoadedFieldLayer<'_>>,
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
        if let Some(zone_rgb) = resolved_event_zone_rgb(event, zone_mask_field) {
            has_zone_data = true;
            zones.insert(zone_rgb);
        }
    }

    (zones, has_zone_data, matched_events)
}

pub(super) fn resolved_event_zone_rgb(
    event: &EventPointCompact,
    zone_mask_field: Option<LoadedFieldLayer<'_>>,
) -> Option<u32> {
    if let Some(zone_mask_field) = zone_mask_field {
        return zone_mask_field
            .field_id_at_map_px(event.map_px_x, event.map_px_y)
            .filter(|zone_rgb| *zone_rgb != 0);
    }
    event.zone_rgb_u32
}

pub(crate) fn sync_evidence_zone_filter(
    patch_filter: Res<PatchFilterState>,
    fish_filter: Res<FishFilterState>,
    snapshot: Res<EventsSnapshotState>,
    layer_registry: Res<LayerRegistry>,
    exact_lookups: Res<ExactLookupCache>,
    mut filter: ResMut<EvidenceZoneFilter>,
) {
    let active_terms = !fish_filter.selected_fish_ids.is_empty();
    if !active_terms {
        apply_zone_filter_state(filter.as_mut(), false, HashSet::new());
        return;
    }

    let Some((from_ts_utc, to_ts_utc, mut fish_ids)) =
        normalized_time_and_fish_filters(&patch_filter, &fish_filter)
    else {
        apply_zone_filter_state(filter.as_mut(), false, HashSet::new());
        return;
    };
    fish_ids.sort_unstable();
    fish_ids.dedup();

    if !snapshot.loaded {
        apply_zone_filter_state(filter.as_mut(), false, HashSet::new());
        return;
    }

    let zone_mask_field = layer_registry
        .get_by_key(SemanticFieldFilterState::ZONE_MASK_LAYER_ID)
        .and_then(|layer| loaded_field_layer(layer, exact_lookups.as_ref()));

    let (next_zone_rgbs, _, _) = collect_evidence_zone_rgbs(
        &snapshot.events,
        from_ts_utc,
        to_ts_utc,
        &fish_ids,
        zone_mask_field,
    );
    apply_zone_filter_state(filter.as_mut(), !next_zone_rgbs.is_empty(), next_zone_rgbs);
}

pub(crate) fn sync_layer_effective_filters(
    layer_registry: Res<LayerRegistry>,
    binding_overrides: Res<LayerFilterBindingOverrideState>,
    semantic_filter: Res<SemanticFieldFilterState>,
    fish_selection_filter: Res<EvidenceZoneFilter>,
    mut effective_filters: ResMut<LayerEffectiveFilterState>,
) {
    effective_filters.sync_to_registry(&layer_registry);
    for layer in layer_registry.ordered() {
        effective_filters.resolve_zone_membership_filter_for_layer(
            layer,
            &binding_overrides,
            fish_selection_filter.as_ref(),
            &semantic_filter,
        );
        effective_filters.resolve_semantic_field_filter_for_layer(
            layer,
            &binding_overrides,
            &semantic_filter,
        );
    }
}

#[cfg(test)]
mod tests {}
