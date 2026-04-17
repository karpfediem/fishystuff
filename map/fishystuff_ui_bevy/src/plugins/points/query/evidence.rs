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

    for layer in layer_registry.ordered() {
        let zone_support = zone_membership_binding_support(layer, &binding_overrides);
        let (zone_active, next_zone_rgbs) = if let Some(projected) =
            project_expression_for_zone_membership(&expression, zone_support)
        {
            let mut candidates = zone_catalog.clone();
            candidates.extend(evaluator.collect_zone_candidates(&projected));
            if candidates.is_empty() && expression_contains_negation(&projected) {
                (false, HashSet::new())
            } else {
                let next_zone_rgbs = candidates
                    .into_iter()
                    .filter(|zone_rgb| evaluator.zone_matches_expression(*zone_rgb, &projected))
                    .collect::<HashSet<_>>();
                (true, next_zone_rgbs)
            }
        } else {
            (false, HashSet::new())
        };
        effective_filters.sync_zone_membership_filter_for_layer(
            layer.key.clone(),
            zone_active,
            next_zone_rgbs,
        );
        let semantic_support = semantic_binding_support(layer, &binding_overrides);
        let (semantic_active, next_field_ids) = if let Some(projected) =
            project_expression_for_semantic_layer(&expression, semantic_support, layer.key.as_str())
        {
            let candidates =
                semantic_field_candidates_for_layer(layer, &field_metadata, &projected);
            if candidates.is_empty() && expression_contains_negation(&projected) {
                (false, Vec::new())
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
                (true, next_field_ids)
            }
        } else {
            (false, Vec::new())
        };
        effective_filters.sync_semantic_field_filter_for_layer(
            layer.key.clone(),
            semantic_active,
            next_field_ids,
        );
    }
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
