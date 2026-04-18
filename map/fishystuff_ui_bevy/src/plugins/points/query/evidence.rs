use std::collections::HashSet;

use bevy::prelude::*;

use crate::map::events::EventsSnapshotState;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::layers::{LayerRegistry, LayerSpec};
use crate::map::search_filters::{
    effective_search_expression, expression_contains_negation,
    project_expression_for_semantic_layer, project_expression_for_zone_membership,
    semantic_field_candidates_for_layer, zone_catalog_rgbs, LayerSearchEvaluator,
    SearchBindingSupport,
};
use crate::plugins::api::{
    CommunityFishZoneSupportIndex, FishCatalog, FishFilterState, LayerEffectiveFilterState,
    LayerFilterBindingOverrideState, PatchFilterState, SearchExpressionState,
    SemanticFieldFilterState,
};

#[cfg(test)]
use crate::map::events::EventZoneSetResolver;

#[cfg(test)]
use fishystuff_api::models::events::EventPointCompact;

#[cfg(test)]
pub(super) fn collect_evidence_zone_rgbs(
    events: &[EventPointCompact],
    from_ts_utc: Option<i64>,
    to_ts_utc: Option<i64>,
    fish_ids: &[i32],
    resolver: &mut EventZoneSetResolver,
) -> (HashSet<u32>, bool, usize) {
    let mut zones = HashSet::new();
    let mut has_zone_data = false;
    let mut matched_events = 0usize;

    for event in events {
        if from_ts_utc.is_some_and(|from_ts_utc| event.ts_utc < from_ts_utc)
            || to_ts_utc.is_some_and(|to_ts_utc| event.ts_utc >= to_ts_utc)
        {
            continue;
        }
        if !fish_ids.is_empty() && fish_ids.binary_search(&event.fish_id).is_err() {
            continue;
        }
        matched_events = matched_events.saturating_add(1);
        let event_zone_rgbs = resolver.full_zone_rgbs(event);
        if !event_zone_rgbs.is_empty() {
            has_zone_data = true;
            zones.extend(event_zone_rgbs.iter().copied());
        }
    }

    (zones, has_zone_data, matched_events)
}

pub(crate) fn sync_layer_effective_filters(
    layer_registry: Res<LayerRegistry>,
    binding_overrides: Res<LayerFilterBindingOverrideState>,
    patch_filter: Res<PatchFilterState>,
    fish_filter: Res<FishFilterState>,
    semantic_filter: Res<SemanticFieldFilterState>,
    search_expression: Res<SearchExpressionState>,
    fish_catalog: Res<FishCatalog>,
    community: Res<CommunityFishZoneSupportIndex>,
    snapshot: Res<EventsSnapshotState>,
    field_metadata: Res<FieldMetadataCache>,
    mut effective_filters: ResMut<LayerEffectiveFilterState>,
) {
    crate::perf_scope!("filters.layer_effective.sync");
    if !layer_registry.is_changed()
        && !binding_overrides.is_changed()
        && !patch_filter.is_changed()
        && !fish_filter.is_changed()
        && !semantic_filter.is_changed()
        && !search_expression.is_changed()
        && !fish_catalog.is_changed()
        && !community.is_changed()
        && !snapshot.is_changed()
        && !field_metadata.is_changed()
    {
        return;
    }

    effective_filters.sync_to_registry(&layer_registry);
    let expression = effective_search_expression(
        &crate::bridge::contract::FishyMapInputState {
            filters: crate::bridge::contract::FishyMapFiltersState {
                search_expression: search_expression.expression.clone(),
                ..Default::default()
            },
            ui: crate::bridge::contract::FishyMapUiState {
                shared_fish_state: search_expression.shared_fish_state.clone(),
                ..Default::default()
            },
            ..Default::default()
        },
        &fish_filter.selected_fish_ids,
        &semantic_filter.selected_field_ids_by_layer,
    );
    crate::perf_last!("filters.expression.nodes", expression.node_count());
    crate::perf_last!("filters.expression.terms", expression.term_count());
    crate::perf_last!("filters.expression.max_depth", expression.max_depth());
    let zone_catalog = zone_catalog_rgbs(&layer_registry, &field_metadata);
    let mut evaluator = LayerSearchEvaluator::new(
        &fish_catalog,
        &community,
        &snapshot,
        patch_filter.from_ts,
        patch_filter.to_ts,
        &search_expression.shared_fish_state.caught_ids,
        &search_expression.shared_fish_state.favourite_ids,
    );
    let mut total_layers = 0usize;
    let mut zone_projected_layers = 0usize;
    let mut zone_active_layers = 0usize;
    let mut zone_candidate_zones = 0usize;
    let mut zone_matched_zones = 0usize;
    let mut semantic_projected_layers = 0usize;
    let mut semantic_active_layers = 0usize;
    let mut semantic_candidate_fields = 0usize;
    let mut semantic_matched_fields = 0usize;

    for layer in layer_registry.ordered() {
        total_layers = total_layers.saturating_add(1);
        let zone_support = zone_membership_binding_support(layer, &binding_overrides);
        let (zone_active, next_zone_rgbs, zone_candidate_count) = if let Some(projected) =
            project_expression_for_zone_membership(&expression, zone_support)
        {
            zone_projected_layers = zone_projected_layers.saturating_add(1);
            let mut candidates = zone_catalog.clone();
            candidates.extend(evaluator.collect_zone_candidates(&projected));
            let candidate_count = candidates.len();
            if candidates.is_empty() && expression_contains_negation(&projected) {
                (false, HashSet::new(), candidate_count)
            } else {
                let next_zone_rgbs = candidates
                    .into_iter()
                    .filter(|zone_rgb| evaluator.zone_matches_expression(*zone_rgb, &projected))
                    .collect::<HashSet<_>>();
                (true, next_zone_rgbs, candidate_count)
            }
        } else {
            (false, HashSet::new(), 0)
        };
        zone_candidate_zones = zone_candidate_zones.saturating_add(zone_candidate_count);
        if zone_active {
            zone_active_layers = zone_active_layers.saturating_add(1);
        }
        zone_matched_zones = zone_matched_zones.saturating_add(next_zone_rgbs.len());
        effective_filters.sync_zone_membership_filter_for_layer(
            layer.key.clone(),
            zone_active,
            next_zone_rgbs,
        );
        let semantic_support = semantic_binding_support(layer, &binding_overrides);
        let (semantic_active, next_field_ids, semantic_candidate_count) = if let Some(projected) =
            project_expression_for_semantic_layer(&expression, semantic_support, layer.key.as_str())
        {
            semantic_projected_layers = semantic_projected_layers.saturating_add(1);
            let candidates =
                semantic_field_candidates_for_layer(layer, &field_metadata, &projected);
            let candidate_count = candidates.len();
            if candidates.is_empty() && expression_contains_negation(&projected) {
                (false, Vec::new(), candidate_count)
            } else {
                let next_field_ids = candidates
                    .into_iter()
                    .filter(|field_id| {
                        evaluator.semantic_field_matches_expression(
                            layer.key.as_str(),
                            *field_id,
                            &projected,
                        )
                    })
                    .collect::<Vec<_>>();
                (true, next_field_ids, candidate_count)
            }
        } else {
            (false, Vec::new(), 0)
        };
        semantic_candidate_fields =
            semantic_candidate_fields.saturating_add(semantic_candidate_count);
        if semantic_active {
            semantic_active_layers = semantic_active_layers.saturating_add(1);
        }
        semantic_matched_fields = semantic_matched_fields.saturating_add(next_field_ids.len());
        effective_filters.sync_semantic_field_filter_for_layer(
            layer.key.clone(),
            semantic_active,
            next_field_ids,
        );
    }

    crate::perf_last!("filters.layer_effective.layers", total_layers);
    crate::perf_last!(
        "filters.layer_effective.zone_projected_layers",
        zone_projected_layers
    );
    crate::perf_last!(
        "filters.layer_effective.zone_active_layers",
        zone_active_layers
    );
    crate::perf_last!(
        "filters.layer_effective.zone_candidate_zones",
        zone_candidate_zones
    );
    crate::perf_last!(
        "filters.layer_effective.zone_matched_zones",
        zone_matched_zones
    );
    crate::perf_last!(
        "filters.layer_effective.semantic_projected_layers",
        semantic_projected_layers
    );
    crate::perf_last!(
        "filters.layer_effective.semantic_active_layers",
        semantic_active_layers
    );
    crate::perf_last!(
        "filters.layer_effective.semantic_candidate_fields",
        semantic_candidate_fields
    );
    crate::perf_last!(
        "filters.layer_effective.semantic_matched_fields",
        semantic_matched_fields
    );
}

pub(super) fn zone_membership_binding_support(
    layer: &LayerSpec,
    overrides: &LayerFilterBindingOverrideState,
) -> SearchBindingSupport {
    let mut support = SearchBindingSupport::default();
    for binding in layer.zone_membership_filter_bindings() {
        if !overrides.is_binding_enabled(layer, binding) {
            continue;
        }
        match binding.source {
            crate::map::layers::LayerFilterSourceKind::FishSelection => {
                support.fish_selection = true;
            }
            crate::map::layers::LayerFilterSourceKind::ZoneSelection => {
                support.zone_selection = true;
            }
            crate::map::layers::LayerFilterSourceKind::SemanticSelection => {
                support.semantic_selection = true;
            }
        }
    }
    support
}

fn semantic_binding_support(
    layer: &LayerSpec,
    overrides: &LayerFilterBindingOverrideState,
) -> SearchBindingSupport {
    let mut support = SearchBindingSupport::default();
    for binding in layer.semantic_selection_filter_bindings() {
        if !overrides.is_binding_enabled(layer, binding) {
            continue;
        }
        if matches!(
            binding.source,
            crate::map::layers::LayerFilterSourceKind::SemanticSelection
        ) {
            support.semantic_selection = true;
        }
    }
    support
}

#[cfg(test)]
mod tests {
    // Coverage for community and ranking support now lives on the layer-effective path.
}
