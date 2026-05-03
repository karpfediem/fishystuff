use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use fishystuff_api::models::events::EventPointCompact;

use crate::bridge::contract::{
    normalize_fish_filter_terms, normalize_i32_list, normalize_u32_list, FishyMapInputState,
    FishyMapPatchBound, FishyMapSearchExpressionNode, FishyMapSearchExpressionOperator,
    FishyMapSearchTerm,
};
use crate::map::events::{EventZoneSetResolver, EventsSnapshotState};
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::layers::{LayerRegistry, LayerSpec};
use crate::plugins::api::{CommunityFishZoneSupportIndex, FishCatalog, FishEntry};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SearchBindingSupport {
    pub fish_selection: bool,
    pub zone_selection: bool,
    pub semantic_selection: bool,
}

pub(crate) fn search_expression_key(expression: &FishyMapSearchExpressionNode) -> String {
    serde_json::to_string(expression).unwrap_or_default()
}

pub(crate) fn input_has_search_terms(input: &FishyMapInputState) -> bool {
    !input.filters.search_expression.is_empty()
        || !input.filters.fish_ids.is_empty()
        || !input.filters.zone_rgbs.is_empty()
        || !input.filters.semantic_field_ids_by_layer.is_empty()
        || !input.filters.fish_filter_terms.is_empty()
        || input.filters.patch_id.is_some()
        || input.filters.from_patch_id.is_some()
        || input.filters.to_patch_id.is_some()
}

pub(crate) fn effective_search_expression(
    input: &FishyMapInputState,
    fallback_fish_ids: &[i32],
    fallback_semantic_field_ids_by_layer: &BTreeMap<String, Vec<u32>>,
) -> FishyMapSearchExpressionNode {
    if !input.filters.search_expression.is_empty() {
        return input.filters.search_expression.clone();
    }
    if input_has_search_terms(input) {
        return search_expression_from_terms(
            &collect_search_terms_from_input(input),
            FishyMapSearchExpressionOperator::Or,
        );
    }
    search_expression_from_terms(
        &collect_search_terms_from_state(fallback_fish_ids, fallback_semantic_field_ids_by_layer),
        FishyMapSearchExpressionOperator::Or,
    )
}

pub(crate) fn project_expression_for_zone_membership(
    expression: &FishyMapSearchExpressionNode,
    support: SearchBindingSupport,
) -> Option<FishyMapSearchExpressionNode> {
    project_expression(expression, &|term| match term {
        FishyMapSearchTerm::Fish { .. } | FishyMapSearchTerm::FishFilter { .. } => {
            support.fish_selection
        }
        FishyMapSearchTerm::Zone { .. } => support.zone_selection,
        FishyMapSearchTerm::PatchBound { .. } | FishyMapSearchTerm::Semantic { .. } => false,
    })
}

pub(crate) fn project_expression_for_semantic_layer(
    expression: &FishyMapSearchExpressionNode,
    support: SearchBindingSupport,
    layer_id: &str,
) -> Option<FishyMapSearchExpressionNode> {
    let normalized_layer_id = layer_id.trim();
    project_expression(expression, &|term| match term {
        FishyMapSearchTerm::Semantic {
            layer_id,
            field_id: _,
        } => support.semantic_selection && layer_id == normalized_layer_id,
        FishyMapSearchTerm::PatchBound { .. }
        | FishyMapSearchTerm::Fish { .. }
        | FishyMapSearchTerm::FishFilter { .. }
        | FishyMapSearchTerm::Zone { .. } => false,
    })
}

pub(crate) fn project_expression_for_fish_selection(
    expression: &FishyMapSearchExpressionNode,
) -> Option<FishyMapSearchExpressionNode> {
    project_expression(expression, &|term| {
        matches!(
            term,
            FishyMapSearchTerm::Fish { .. } | FishyMapSearchTerm::FishFilter { .. }
        )
    })
}

pub(crate) fn fish_id_matches_search_expression(
    fish_catalog: &FishCatalog,
    fish_id: i32,
    expression: &FishyMapSearchExpressionNode,
    shared_fish_state: &crate::bridge::contract::FishyMapSharedFishState,
) -> bool {
    let caught_ids = shared_fish_state.caught_ids.iter().copied().collect();
    let favourite_ids = shared_fish_state.favourite_ids.iter().copied().collect();
    evaluate_expression(expression, &mut |term| {
        fish_id_matches_search_term(fish_catalog, fish_id, term, &caught_ids, &favourite_ids)
    })
}

pub(crate) fn expression_contains_negation(expression: &FishyMapSearchExpressionNode) -> bool {
    match expression {
        FishyMapSearchExpressionNode::Term { negated, .. } => *negated,
        FishyMapSearchExpressionNode::Group {
            negated, children, ..
        } => *negated || children.iter().any(expression_contains_negation),
    }
}

pub(crate) fn collect_zone_terms(expression: &FishyMapSearchExpressionNode) -> HashSet<u32> {
    let mut zones = HashSet::new();
    collect_zone_terms_into(expression, &mut zones);
    zones
}

pub(crate) fn semantic_field_candidates_for_layer(
    layer: &LayerSpec,
    field_metadata: &FieldMetadataCache,
    expression: &FishyMapSearchExpressionNode,
) -> HashSet<u32> {
    let mut field_ids = HashSet::new();
    if let Some(metadata_url) = layer.field_metadata_url() {
        if let Some(metadata) = field_metadata.get(layer.id, &metadata_url) {
            field_ids.extend(metadata.entries.keys().copied());
        }
    }
    collect_semantic_terms_for_layer_into(expression, layer.key.as_str(), &mut field_ids);
    field_ids
}

pub(crate) fn zone_catalog_rgbs(
    layer_registry: &LayerRegistry,
    field_metadata: &FieldMetadataCache,
) -> HashSet<u32> {
    let Some(layer) = layer_registry.get_by_key("zone_mask") else {
        return HashSet::new();
    };
    let Some(metadata_url) = layer.field_metadata_url() else {
        return HashSet::new();
    };
    let Some(metadata) = field_metadata.get(layer.id, &metadata_url) else {
        return HashSet::new();
    };
    metadata.entries.keys().copied().collect()
}

pub(crate) struct LayerSearchEvaluator<'a> {
    fish_catalog: &'a FishCatalog,
    community: &'a CommunityFishZoneSupportIndex,
    snapshot: &'a EventsSnapshotState,
    from_ts_utc: Option<i64>,
    to_ts_utc: Option<i64>,
    caught_ids: HashSet<i32>,
    favourite_ids: HashSet<i32>,
    zone_term_cache: HashMap<String, HashSet<u32>>,
    fish_filter_cache: HashMap<String, Vec<i32>>,
    zone_resolver: EventZoneSetResolver,
}

impl<'a> LayerSearchEvaluator<'a> {
    pub(crate) fn new(
        fish_catalog: &'a FishCatalog,
        community: &'a CommunityFishZoneSupportIndex,
        snapshot: &'a EventsSnapshotState,
        from_ts_utc: Option<i64>,
        to_ts_utc: Option<i64>,
        caught_ids: &[i32],
        favourite_ids: &[i32],
    ) -> Self {
        Self {
            fish_catalog,
            community,
            snapshot,
            from_ts_utc,
            to_ts_utc,
            caught_ids: caught_ids.iter().copied().collect(),
            favourite_ids: favourite_ids.iter().copied().collect(),
            zone_term_cache: HashMap::new(),
            fish_filter_cache: HashMap::new(),
            zone_resolver: EventZoneSetResolver::new(),
        }
    }

    pub(crate) fn collect_zone_candidates(
        &mut self,
        expression: &FishyMapSearchExpressionNode,
    ) -> HashSet<u32> {
        let mut zones = collect_zone_terms(expression);
        self.collect_fish_term_zone_candidates(expression, &mut zones);
        zones
    }

    pub(crate) fn zone_matches_expression(
        &mut self,
        zone_rgb: u32,
        expression: &FishyMapSearchExpressionNode,
    ) -> bool {
        evaluate_expression(expression, &mut |term| {
            self.zone_matches_term(zone_rgb, term)
        })
    }

    pub(crate) fn event_matches_expression(
        &mut self,
        event: &EventPointCompact,
        expression: &FishyMapSearchExpressionNode,
    ) -> bool {
        evaluate_expression(expression, &mut |term| self.event_matches_term(event, term))
    }

    pub(crate) fn semantic_field_matches_expression(
        &self,
        layer_id: &str,
        field_id: u32,
        expression: &FishyMapSearchExpressionNode,
    ) -> bool {
        evaluate_expression(expression, &mut |term| match term {
            FishyMapSearchTerm::Semantic {
                layer_id: target_layer_id,
                field_id: target_field_id,
            } => target_layer_id == layer_id && *target_field_id == field_id,
            FishyMapSearchTerm::PatchBound { .. }
            | FishyMapSearchTerm::Fish { .. }
            | FishyMapSearchTerm::FishFilter { .. }
            | FishyMapSearchTerm::Zone { .. } => false,
        })
    }

    fn zone_matches_term(&mut self, zone_rgb: u32, term: &FishyMapSearchTerm) -> bool {
        match term {
            FishyMapSearchTerm::Fish { .. } | FishyMapSearchTerm::FishFilter { .. } => {
                self.zone_support_for_term(term).contains(&zone_rgb)
            }
            FishyMapSearchTerm::Zone { zone_rgb: target } => *target == zone_rgb,
            FishyMapSearchTerm::PatchBound { .. } | FishyMapSearchTerm::Semantic { .. } => false,
        }
    }

    fn event_matches_term(&mut self, event: &EventPointCompact, term: &FishyMapSearchTerm) -> bool {
        match term {
            FishyMapSearchTerm::Fish { .. } | FishyMapSearchTerm::FishFilter { .. } => {
                self.event_matches_fish_term(event.fish_id, term)
            }
            FishyMapSearchTerm::Zone { zone_rgb } => {
                self.zone_resolver.zone_rgbs(event).contains(zone_rgb)
            }
            FishyMapSearchTerm::PatchBound { .. } | FishyMapSearchTerm::Semantic { .. } => false,
        }
    }

    fn event_matches_fish_term(&self, fish_id: i32, term: &FishyMapSearchTerm) -> bool {
        if let Some(fish) = self.fish_catalog.entry_for_fish(fish_id) {
            return fish_matches_search_term(fish, term, &self.caught_ids, &self.favourite_ids);
        }
        match term {
            FishyMapSearchTerm::Fish { fish_id: target } => *target == fish_id,
            FishyMapSearchTerm::FishFilter { term } => {
                let normalized = term.trim().to_lowercase();
                if normalized == "favourite" {
                    return self.favourite_ids.contains(&fish_id);
                }
                if normalized == "missing" {
                    return !self.caught_ids.contains(&fish_id);
                }
                false
            }
            FishyMapSearchTerm::PatchBound { .. }
            | FishyMapSearchTerm::Zone { .. }
            | FishyMapSearchTerm::Semantic { .. } => false,
        }
    }

    fn zone_support_for_term(&mut self, term: &FishyMapSearchTerm) -> &HashSet<u32> {
        let key = serde_json::to_string(term).unwrap_or_default();
        if !self.zone_term_cache.contains_key(&key) {
            crate::perf_counter_add!("filters.zone_term_cache.miss", 1);
            let matching_fish_ids = self.fish_ids_for_term(term);
            let zones = self.collect_zone_support_for_fish_ids(&matching_fish_ids);
            self.zone_term_cache.insert(key.clone(), zones);
        } else {
            crate::perf_counter_add!("filters.zone_term_cache.hit", 1);
        }
        self.zone_term_cache.get(&key).expect("term cache entry")
    }

    fn fish_ids_for_term(&mut self, term: &FishyMapSearchTerm) -> Vec<i32> {
        match term {
            FishyMapSearchTerm::Fish { fish_id } => self.identity_ids_for_fish(*fish_id),
            FishyMapSearchTerm::FishFilter { term } => {
                let key = term.trim().to_lowercase();
                if let Some(ids) = self.fish_filter_cache.get(&key) {
                    return ids.clone();
                }
                let search_term = FishyMapSearchTerm::FishFilter { term: key.clone() };
                let mut fish_ids = BTreeSet::new();
                for fish in &self.fish_catalog.entries {
                    if fish_matches_search_term(
                        fish,
                        &search_term,
                        &self.caught_ids,
                        &self.favourite_ids,
                    ) {
                        for fish_id in fish_identity_ids(fish) {
                            fish_ids.insert(fish_id);
                        }
                    }
                }
                let resolved = fish_ids.into_iter().collect::<Vec<_>>();
                self.fish_filter_cache.insert(key, resolved.clone());
                resolved
            }
            FishyMapSearchTerm::PatchBound { .. }
            | FishyMapSearchTerm::Zone { .. }
            | FishyMapSearchTerm::Semantic { .. } => Vec::new(),
        }
    }

    fn identity_ids_for_fish(&self, fish_id: i32) -> Vec<i32> {
        if let Some(fish) = self.fish_catalog.entry_for_fish(fish_id) {
            return fish_identity_ids(fish);
        }
        if fish_id > 0 {
            return vec![fish_id];
        }
        Vec::new()
    }

    fn collect_zone_support_for_fish_ids(&mut self, fish_ids: &[i32]) -> HashSet<u32> {
        let _profiling_scope = crate::profiling::scope("filters.zone_support.collect");
        let mut normalized_fish_ids = normalize_i32_list(fish_ids.to_vec());
        if normalized_fish_ids.is_empty() {
            return HashSet::new();
        }
        normalized_fish_ids.sort_unstable();
        crate::perf_counter_add!("filters.zone_support.lookups", 1);
        crate::perf_gauge!(
            "filters.zone_support.lookup_fish_ids",
            normalized_fish_ids.len()
        );
        crate::perf_last!(
            "filters.zone_support.lookup_fish_ids",
            normalized_fish_ids.len()
        );

        let mut zones = HashSet::new();
        if self.snapshot.loaded {
            crate::perf_counter_add!(
                "filters.zone_support.snapshot_events_scanned",
                self.snapshot.events.len()
            );
            let mut matched_events = 0usize;
            for event in &self.snapshot.events {
                if self
                    .from_ts_utc
                    .is_some_and(|from_ts_utc| event.ts_utc < from_ts_utc)
                    || self
                        .to_ts_utc
                        .is_some_and(|to_ts_utc| event.ts_utc >= to_ts_utc)
                {
                    continue;
                }
                if normalized_fish_ids.binary_search(&event.fish_id).is_err() {
                    continue;
                }
                matched_events = matched_events.saturating_add(1);
                zones.extend(self.zone_resolver.full_zone_rgbs(event).iter().copied());
            }
            crate::perf_counter_add!(
                "filters.zone_support.snapshot_events_matched",
                matched_events
            );
        }

        let mut community_item_ids = 0usize;
        for fish_id in normalized_fish_ids {
            if let Some(item_id) = self.fish_catalog.item_id_for_fish(fish_id) {
                community_item_ids = community_item_ids.saturating_add(1);
                zones.extend(self.community.zone_rgbs_for_item(item_id).iter().copied());
            }
        }
        crate::perf_counter_add!(
            "filters.zone_support.community_item_ids",
            community_item_ids
        );
        crate::perf_gauge!("filters.zone_support.matched_zones", zones.len());
        crate::perf_last!("filters.zone_support.matched_zones", zones.len());
        zones
    }

    fn collect_fish_term_zone_candidates(
        &mut self,
        expression: &FishyMapSearchExpressionNode,
        zones: &mut HashSet<u32>,
    ) {
        match expression {
            FishyMapSearchExpressionNode::Term { term, .. } => match term {
                FishyMapSearchTerm::Fish { .. } | FishyMapSearchTerm::FishFilter { .. } => {
                    zones.extend(self.zone_support_for_term(term).iter().copied());
                }
                FishyMapSearchTerm::PatchBound { .. }
                | FishyMapSearchTerm::Zone { .. }
                | FishyMapSearchTerm::Semantic { .. } => {}
            },
            FishyMapSearchExpressionNode::Group { children, .. } => {
                for child in children {
                    self.collect_fish_term_zone_candidates(child, zones);
                }
            }
        }
    }
}

fn fish_id_matches_search_term(
    fish_catalog: &FishCatalog,
    fish_id: i32,
    term: &FishyMapSearchTerm,
    caught_ids: &HashSet<i32>,
    favourite_ids: &HashSet<i32>,
) -> bool {
    if let Some(fish) = fish_catalog.entry_for_fish(fish_id) {
        return fish_matches_search_term(fish, term, caught_ids, favourite_ids);
    }
    match term {
        FishyMapSearchTerm::Fish { fish_id: target } => *target == fish_id,
        FishyMapSearchTerm::FishFilter { term } => match term.as_str() {
            "favourite" => favourite_ids.contains(&fish_id),
            "missing" => !caught_ids.contains(&fish_id),
            _ => false,
        },
        FishyMapSearchTerm::PatchBound { .. }
        | FishyMapSearchTerm::Zone { .. }
        | FishyMapSearchTerm::Semantic { .. } => false,
    }
}

fn collect_search_terms_from_input(input: &FishyMapInputState) -> Vec<FishyMapSearchTerm> {
    let mut terms = Vec::new();
    let mut seen = BTreeSet::new();
    let mut push_term = |term: FishyMapSearchTerm| {
        let key = serde_json::to_string(&term).unwrap_or_default();
        if !key.is_empty() && seen.insert(key) {
            terms.push(term);
        }
    };

    let exact_patch_id = input.filters.patch_id.clone();
    let from_patch_id = input
        .filters
        .from_patch_id
        .clone()
        .or_else(|| exact_patch_id.clone());
    let to_patch_id = input.filters.to_patch_id.clone().or_else(|| exact_patch_id);
    if let Some(patch_id) = from_patch_id.filter(|value| !value.trim().is_empty()) {
        push_term(FishyMapSearchTerm::PatchBound {
            bound: FishyMapPatchBound::From,
            patch_id: Some(patch_id),
        });
    }
    if let Some(patch_id) = to_patch_id.filter(|value| !value.trim().is_empty()) {
        push_term(FishyMapSearchTerm::PatchBound {
            bound: FishyMapPatchBound::To,
            patch_id: Some(patch_id),
        });
    }
    for term in normalize_fish_filter_terms(input.filters.fish_filter_terms.clone()) {
        push_term(FishyMapSearchTerm::FishFilter { term });
    }
    for fish_id in normalize_i32_list(input.filters.fish_ids.clone()) {
        if fish_id > 0 {
            push_term(FishyMapSearchTerm::Fish { fish_id });
        }
    }

    let mut zone_rgbs = input.filters.zone_rgbs.clone();
    if let Some(zone_mask_zone_rgbs) = input
        .filters
        .semantic_field_ids_by_layer
        .get("zone_mask")
        .cloned()
    {
        zone_rgbs.extend(zone_mask_zone_rgbs);
    }
    for zone_rgb in normalize_u32_list(zone_rgbs) {
        if zone_rgb > 0 {
            push_term(FishyMapSearchTerm::Zone { zone_rgb });
        }
    }

    for (layer_id_raw, field_ids) in &input.filters.semantic_field_ids_by_layer {
        let layer_id = layer_id_raw.trim();
        if layer_id.is_empty() || layer_id == "zone_mask" {
            continue;
        }
        for field_id in normalize_u32_list(field_ids.clone()) {
            if field_id == 0 {
                continue;
            }
            push_term(FishyMapSearchTerm::Semantic {
                layer_id: layer_id.to_string(),
                field_id,
            });
        }
    }

    terms
}

fn collect_search_terms_from_state(
    fish_ids: &[i32],
    semantic_field_ids_by_layer: &BTreeMap<String, Vec<u32>>,
) -> Vec<FishyMapSearchTerm> {
    let mut terms = Vec::new();
    let mut seen = BTreeSet::new();
    let mut push_term = |term: FishyMapSearchTerm| {
        let key = serde_json::to_string(&term).unwrap_or_default();
        if !key.is_empty() && seen.insert(key) {
            terms.push(term);
        }
    };

    for fish_id in normalize_i32_list(fish_ids.to_vec()) {
        if fish_id > 0 {
            push_term(FishyMapSearchTerm::Fish { fish_id });
        }
    }

    let mut zone_rgbs = semantic_field_ids_by_layer
        .get("zone_mask")
        .cloned()
        .unwrap_or_default();
    for zone_rgb in normalize_u32_list(std::mem::take(&mut zone_rgbs)) {
        if zone_rgb > 0 {
            push_term(FishyMapSearchTerm::Zone { zone_rgb });
        }
    }

    for (layer_id_raw, field_ids) in semantic_field_ids_by_layer {
        let layer_id = layer_id_raw.trim();
        if layer_id.is_empty() || layer_id == "zone_mask" {
            continue;
        }
        for field_id in normalize_u32_list(field_ids.clone()) {
            if field_id == 0 {
                continue;
            }
            push_term(FishyMapSearchTerm::Semantic {
                layer_id: layer_id.to_string(),
                field_id,
            });
        }
    }

    terms
}

fn search_expression_from_terms(
    terms: &[FishyMapSearchTerm],
    operator: FishyMapSearchExpressionOperator,
) -> FishyMapSearchExpressionNode {
    FishyMapSearchExpressionNode::Group {
        operator,
        children: terms
            .iter()
            .cloned()
            .map(|term| FishyMapSearchExpressionNode::Term {
                term,
                negated: false,
            })
            .collect(),
        negated: false,
    }
}

fn project_expression(
    expression: &FishyMapSearchExpressionNode,
    keep_term: &impl Fn(&FishyMapSearchTerm) -> bool,
) -> Option<FishyMapSearchExpressionNode> {
    match expression {
        FishyMapSearchExpressionNode::Term { term, negated } => {
            keep_term(term).then(|| FishyMapSearchExpressionNode::Term {
                term: term.clone(),
                negated: *negated,
            })
        }
        FishyMapSearchExpressionNode::Group {
            operator,
            children,
            negated,
        } => {
            let mut projected_children = children
                .iter()
                .filter_map(|child| project_expression(child, keep_term))
                .collect::<Vec<_>>();
            if projected_children.is_empty() {
                return None;
            }
            if projected_children.len() == 1 {
                let mut child = projected_children.remove(0);
                if *negated {
                    toggle_node_negation(&mut child);
                }
                return Some(child);
            }
            Some(FishyMapSearchExpressionNode::Group {
                operator: *operator,
                children: projected_children,
                negated: *negated,
            })
        }
    }
}

fn toggle_node_negation(node: &mut FishyMapSearchExpressionNode) {
    match node {
        FishyMapSearchExpressionNode::Term { negated, .. }
        | FishyMapSearchExpressionNode::Group { negated, .. } => {
            *negated = !*negated;
        }
    }
}

fn evaluate_expression(
    expression: &FishyMapSearchExpressionNode,
    eval_term: &mut impl FnMut(&FishyMapSearchTerm) -> bool,
) -> bool {
    match expression {
        FishyMapSearchExpressionNode::Term { term, negated } => {
            let result = eval_term(term);
            if *negated {
                !result
            } else {
                result
            }
        }
        FishyMapSearchExpressionNode::Group {
            operator,
            children,
            negated,
        } => {
            let result = match operator {
                FishyMapSearchExpressionOperator::And => children
                    .iter()
                    .all(|child| evaluate_expression(child, eval_term)),
                FishyMapSearchExpressionOperator::Or => children
                    .iter()
                    .any(|child| evaluate_expression(child, eval_term)),
            };
            if *negated {
                !result
            } else {
                result
            }
        }
    }
}

fn collect_zone_terms_into(expression: &FishyMapSearchExpressionNode, zones: &mut HashSet<u32>) {
    match expression {
        FishyMapSearchExpressionNode::Term { term, .. } => {
            if let FishyMapSearchTerm::Zone { zone_rgb } = term {
                zones.insert(*zone_rgb);
            }
        }
        FishyMapSearchExpressionNode::Group { children, .. } => {
            for child in children {
                collect_zone_terms_into(child, zones);
            }
        }
    }
}

fn collect_semantic_terms_for_layer_into(
    expression: &FishyMapSearchExpressionNode,
    layer_id: &str,
    field_ids: &mut HashSet<u32>,
) {
    match expression {
        FishyMapSearchExpressionNode::Term { term, .. } => {
            if let FishyMapSearchTerm::Semantic {
                layer_id: target_layer_id,
                field_id,
            } = term
            {
                if target_layer_id == layer_id {
                    field_ids.insert(*field_id);
                }
            }
        }
        FishyMapSearchExpressionNode::Group { children, .. } => {
            for child in children {
                collect_semantic_terms_for_layer_into(child, layer_id, field_ids);
            }
        }
    }
}

fn fish_matches_search_term(
    fish: &FishEntry,
    term: &FishyMapSearchTerm,
    caught_ids: &HashSet<i32>,
    favourite_ids: &HashSet<i32>,
) -> bool {
    match term {
        FishyMapSearchTerm::Fish { fish_id } => fish_identity_ids(fish).contains(fish_id),
        FishyMapSearchTerm::FishFilter { term } => fish_matches_shared_filter_terms(
            fish,
            std::slice::from_ref(term),
            caught_ids,
            favourite_ids,
        ),
        FishyMapSearchTerm::PatchBound { .. }
        | FishyMapSearchTerm::Zone { .. }
        | FishyMapSearchTerm::Semantic { .. } => false,
    }
}

fn fish_matches_shared_filter_terms(
    fish: &FishEntry,
    filter_terms: &[String],
    caught_ids: &HashSet<i32>,
    favourite_ids: &HashSet<i32>,
) -> bool {
    let fish_identity_ids = fish_identity_ids(fish);
    if fish_identity_ids.is_empty() {
        return false;
    }
    let selected_grade_terms = filter_terms
        .iter()
        .filter(|term| matches!(term.as_str(), "red" | "yellow" | "blue" | "green" | "white"))
        .cloned()
        .collect::<Vec<_>>();
    if !selected_grade_terms.is_empty() {
        let grade_term = resolve_fish_grade_filter_term(fish);
        if !selected_grade_terms.iter().any(|term| term == grade_term) {
            return false;
        }
    }
    for term in filter_terms {
        match term.as_str() {
            "red" | "yellow" | "blue" | "green" | "white" => {}
            "favourite" => {
                if !fish_identity_ids
                    .iter()
                    .any(|fish_id| favourite_ids.contains(fish_id))
                {
                    return false;
                }
            }
            "missing" => {
                if fish_identity_ids
                    .iter()
                    .any(|fish_id| caught_ids.contains(fish_id))
                {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

fn fish_identity_ids(fish: &FishEntry) -> Vec<i32> {
    let mut ids = Vec::new();
    for candidate in [fish.id, fish.item_id] {
        if candidate > 0 && !ids.contains(&candidate) {
            ids.push(candidate);
        }
    }
    ids
}

fn resolve_fish_grade_filter_term(fish: &FishEntry) -> &'static str {
    let grade = fish
        .grade
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    if fish.is_prize || grade == "prize" || grade == "red" {
        return "red";
    }
    match grade.as_str() {
        "rare" | "yellow" => "yellow",
        "highquality" | "high_quality" | "high-quality" | "blue" => "blue",
        "general" | "green" => "green",
        "trash" | "white" => "white",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashSet};

    use super::{
        effective_search_expression, fish_id_matches_search_expression,
        project_expression_for_fish_selection, project_expression_for_semantic_layer,
        project_expression_for_zone_membership, search_expression_key, zone_catalog_rgbs,
        LayerSearchEvaluator, SearchBindingSupport,
    };
    use crate::bridge::contract::FishyMapInputState;
    use crate::bridge::contract::{
        FishyMapSearchExpressionNode, FishyMapSearchExpressionOperator, FishyMapSearchTerm,
        FishyMapSharedFishState,
    };
    use crate::map::events::EventsSnapshotState;
    use crate::map::field_metadata::FieldMetadataCache;
    use crate::map::layers::{build_local_layer_specs, LayerRegistry};
    use crate::plugins::api::{CommunityFishZoneSupportIndex, FishCatalog, FishEntry};
    use fishystuff_api::models::events::EventPointCompact;
    use fishystuff_core::field_metadata::{FieldHoverMetadataAsset, FieldHoverMetadataEntry};

    fn fish(id: i32, item_id: i32, grade: Option<&str>, is_prize: bool) -> FishEntry {
        FishEntry {
            id,
            item_id,
            encyclopedia_key: None,
            encyclopedia_id: None,
            name: format!("fish-{id}"),
            name_lower: format!("fish-{id}"),
            grade: grade.map(str::to_string),
            is_prize,
        }
    }

    #[test]
    fn effective_search_expression_falls_back_to_current_state_when_input_is_empty() {
        let mut semantic = BTreeMap::new();
        semantic.insert("zone_mask".to_string(), vec![0x123456]);
        semantic.insert("regions".to_string(), vec![77]);

        let expression =
            effective_search_expression(&FishyMapInputState::default(), &[240], &semantic);

        assert_eq!(
            expression,
            FishyMapSearchExpressionNode::Group {
                operator: FishyMapSearchExpressionOperator::Or,
                children: vec![
                    FishyMapSearchExpressionNode::Term {
                        term: FishyMapSearchTerm::Fish { fish_id: 240 },
                        negated: false,
                    },
                    FishyMapSearchExpressionNode::Term {
                        term: FishyMapSearchTerm::Zone { zone_rgb: 0x123456 },
                        negated: false,
                    },
                    FishyMapSearchExpressionNode::Term {
                        term: FishyMapSearchTerm::Semantic {
                            layer_id: "regions".to_string(),
                            field_id: 77,
                        },
                        negated: false,
                    },
                ],
                negated: false,
            }
        );
    }

    #[test]
    fn semantic_projection_keeps_supported_branch_under_mixed_or() {
        let expression = FishyMapSearchExpressionNode::Group {
            operator: FishyMapSearchExpressionOperator::Or,
            children: vec![
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::Fish { fish_id: 240 },
                    negated: false,
                },
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::Semantic {
                        layer_id: "regions".to_string(),
                        field_id: 77,
                    },
                    negated: false,
                },
            ],
            negated: false,
        };

        let projected = project_expression_for_semantic_layer(
            &expression,
            SearchBindingSupport {
                semantic_selection: true,
                ..SearchBindingSupport::default()
            },
            "regions",
        );

        assert_eq!(
            projected,
            Some(FishyMapSearchExpressionNode::Term {
                term: FishyMapSearchTerm::Semantic {
                    layer_id: "regions".to_string(),
                    field_id: 77,
                },
                negated: false,
            })
        );
    }

    #[test]
    fn zone_projection_keeps_fish_and_zone_terms() {
        let expression = FishyMapSearchExpressionNode::Group {
            operator: FishyMapSearchExpressionOperator::Or,
            children: vec![
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::Fish { fish_id: 240 },
                    negated: false,
                },
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::Zone { zone_rgb: 0x123456 },
                    negated: false,
                },
            ],
            negated: false,
        };

        let projected = project_expression_for_zone_membership(
            &expression,
            SearchBindingSupport {
                fish_selection: true,
                zone_selection: true,
                ..SearchBindingSupport::default()
            },
        );

        assert_eq!(
            search_expression_key(projected.as_ref().expect("projected")),
            search_expression_key(&expression)
        );
    }

    #[test]
    fn fish_projection_keeps_fish_terms_only() {
        let expression = FishyMapSearchExpressionNode::Group {
            operator: FishyMapSearchExpressionOperator::And,
            children: vec![
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::Fish { fish_id: 240 },
                    negated: false,
                },
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::FishFilter {
                        term: "yellow".to_string(),
                    },
                    negated: false,
                },
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::Zone { zone_rgb: 0x123456 },
                    negated: false,
                },
            ],
            negated: false,
        };

        let projected = project_expression_for_fish_selection(&expression).expect("projected");

        assert_eq!(
            projected,
            FishyMapSearchExpressionNode::Group {
                operator: FishyMapSearchExpressionOperator::And,
                children: vec![
                    FishyMapSearchExpressionNode::Term {
                        term: FishyMapSearchTerm::Fish { fish_id: 240 },
                        negated: false,
                    },
                    FishyMapSearchExpressionNode::Term {
                        term: FishyMapSearchTerm::FishFilter {
                            term: "yellow".to_string(),
                        },
                        negated: false,
                    },
                ],
                negated: false,
            }
        );
    }

    #[test]
    fn fish_id_expression_match_resolves_item_aliases_and_grade_filters() {
        let mut fish_catalog = FishCatalog::default();
        fish_catalog.replace(vec![fish(240, 820240, Some("Rare"), false)]);
        let expression = FishyMapSearchExpressionNode::Group {
            operator: FishyMapSearchExpressionOperator::And,
            children: vec![
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::Fish { fish_id: 240 },
                    negated: false,
                },
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::FishFilter {
                        term: "yellow".to_string(),
                    },
                    negated: false,
                },
            ],
            negated: false,
        };

        assert!(fish_id_matches_search_expression(
            &fish_catalog,
            820240,
            &expression,
            &FishyMapSharedFishState::default(),
        ));
        assert!(!fish_id_matches_search_expression(
            &fish_catalog,
            820777,
            &expression,
            &FishyMapSharedFishState::default(),
        ));
    }

    #[test]
    fn zone_catalog_uses_zone_mask_metadata() {
        let (revision, layers) = build_local_layer_specs(
            crate::map::layers::AvailableLayerCatalog::default().entries(),
            None,
        );
        let mut registry = LayerRegistry::default();
        registry.apply_layer_specs(revision, None, layers);
        let zone_mask = registry.get_by_key("zone_mask").expect("zone mask");

        let mut field_metadata = FieldMetadataCache::default();
        field_metadata.insert_ready(
            zone_mask.id,
            zone_mask.field_metadata_url().expect("zone metadata url"),
            FieldHoverMetadataAsset {
                entries: BTreeMap::from([
                    (0x111111, FieldHoverMetadataEntry::default()),
                    (0x222222, FieldHoverMetadataEntry::default()),
                ]),
            },
        );

        assert_eq!(
            zone_catalog_rgbs(&registry, &field_metadata),
            HashSet::from([0x111111, 0x222222])
        );
    }

    #[test]
    fn evaluator_matches_mixed_point_expression() {
        let expression = FishyMapSearchExpressionNode::Group {
            operator: FishyMapSearchExpressionOperator::Or,
            children: vec![
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::Fish { fish_id: 240 },
                    negated: false,
                },
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::Zone { zone_rgb: 0xabcdef },
                    negated: false,
                },
            ],
            negated: false,
        };
        let mut fish_catalog = FishCatalog::default();
        fish_catalog.replace(vec![
            fish(240, 820240, Some("Rare"), false),
            fish(777, 820777, Some("General"), false),
        ]);
        let community = CommunityFishZoneSupportIndex::default();
        let snapshot = EventsSnapshotState::default();
        let mut evaluator =
            LayerSearchEvaluator::new(&fish_catalog, &community, &snapshot, None, None, &[], &[]);

        let fish_match = EventPointCompact {
            event_id: 1,
            fish_id: 240,
            ts_utc: 100,
            map_px_x: 0,
            map_px_y: 0,
            length_milli: 1,
            world_x: None,
            world_z: None,
            zone_rgb_u32: Some(0x111111),
            zone_rgbs: vec![0x111111],
            full_zone_rgbs: vec![0x111111],
            source_kind: None,
            source_id: None,
        };
        let zone_match = EventPointCompact {
            event_id: 2,
            fish_id: 777,
            ts_utc: 100,
            map_px_x: 0,
            map_px_y: 0,
            length_milli: 1,
            world_x: None,
            world_z: None,
            zone_rgb_u32: Some(0xabcdef),
            zone_rgbs: vec![0xabcdef],
            full_zone_rgbs: vec![0xabcdef],
            source_kind: None,
            source_id: None,
        };
        let miss = EventPointCompact {
            event_id: 3,
            fish_id: 777,
            ts_utc: 100,
            map_px_x: 0,
            map_px_y: 0,
            length_milli: 1,
            world_x: None,
            world_z: None,
            zone_rgb_u32: Some(0x555555),
            zone_rgbs: vec![0x555555],
            full_zone_rgbs: vec![0x555555],
            source_kind: None,
            source_id: None,
        };

        assert!(evaluator.event_matches_expression(&fish_match, &expression));
        assert!(evaluator.event_matches_expression(&zone_match, &expression));
        assert!(!evaluator.event_matches_expression(&miss, &expression));
    }
}
