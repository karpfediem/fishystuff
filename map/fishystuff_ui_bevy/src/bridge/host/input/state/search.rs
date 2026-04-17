use std::collections::{BTreeMap, HashSet};

use crate::bridge::contract::{
    normalize_fish_filter_terms, FishyMapSearchExpressionNode, FishyMapSearchExpressionOperator,
    FishyMapSearchProjection, FishyMapSearchTerm, FishyMapSharedFishState,
};
use crate::bridge::host::persistence::apply_patch_range_override;
use crate::bridge::host::BrowserBridgeState;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::layers::LayerRegistry;
use crate::plugins::api::{
    FishCatalog, FishEntry, FishFilterState, PatchFilterState, SemanticFieldFilterState,
};
use crate::prelude::*;

const FISH_FILTER_NO_MATCH_SENTINEL_ID: i32 = -1;

#[derive(Clone)]
struct ProjectedSearchExpression {
    representable: bool,
    expression: Option<FishyMapSearchExpressionNode>,
    orthogonal: bool,
}

#[derive(Clone, Copy)]
enum SearchExpressionProjectionMode {
    Fish,
    Zone,
}

pub(super) fn resolve_browser_search_filters(
    bridge: Res<BrowserBridgeState>,
    fish_catalog: Res<FishCatalog>,
    field_metadata: Res<FieldMetadataCache>,
    layer_registry: Res<LayerRegistry>,
    mut fish_filter: ResMut<FishFilterState>,
    mut semantic_filter: ResMut<SemanticFieldFilterState>,
    mut patch_filter: ResMut<PatchFilterState>,
) {
    if !bridge.is_changed()
        && !fish_catalog.is_changed()
        && !field_metadata.is_changed()
        && !layer_registry.is_changed()
        && !patch_filter.is_changed()
    {
        return;
    }

    let raw_projection = raw_projection_from_input(&bridge.input);
    let expression_projection = (!bridge.input.filters.search_expression.is_empty()).then(|| {
        FishyMapSearchProjection::from_expression(&bridge.input.filters.search_expression)
    });

    let effective_fish_ids = resolve_effective_fish_ids(
        &raw_projection.fish_ids,
        &raw_projection.fish_filter_terms,
        &bridge.input.filters.search_expression,
        &bridge.input.ui.shared_fish_state,
        &fish_catalog.entries,
    );
    if fish_filter.selected_fish_ids != effective_fish_ids {
        fish_filter.selected_fish_ids = effective_fish_ids;
    }

    let effective_zone_rgbs = resolve_effective_zone_rgbs(
        &raw_projection.zone_rgbs,
        &bridge.input.filters.search_expression,
        &zone_catalog_rgbs(&layer_registry, &field_metadata),
    );
    let mut effective_semantic_field_ids = raw_projection.semantic_field_ids_by_layer.clone();
    if let Some(expression_projection) = expression_projection.as_ref() {
        for (layer_id, field_ids) in &expression_projection.semantic_field_ids_by_layer {
            if layer_id != SemanticFieldFilterState::ZONE_MASK_LAYER_ID {
                effective_semantic_field_ids.insert(layer_id.clone(), field_ids.clone());
            }
        }
    }
    if effective_zone_rgbs.is_empty() {
        effective_semantic_field_ids.remove(SemanticFieldFilterState::ZONE_MASK_LAYER_ID);
    } else {
        effective_semantic_field_ids.insert(
            SemanticFieldFilterState::ZONE_MASK_LAYER_ID.to_string(),
            effective_zone_rgbs,
        );
    }
    if semantic_filter.selected_field_ids_by_layer != effective_semantic_field_ids {
        semantic_filter.selected_field_ids_by_layer = effective_semantic_field_ids;
    }

    let projected_patch_id = expression_projection
        .as_ref()
        .and_then(|projection| projection.patch_id.clone())
        .or_else(|| raw_projection.patch_id.clone());
    let projected_from_patch_id = expression_projection
        .as_ref()
        .and_then(|projection| projection.from_patch_id.clone())
        .or_else(|| raw_projection.from_patch_id.clone())
        .or_else(|| projected_patch_id.clone());
    let projected_to_patch_id = expression_projection
        .as_ref()
        .and_then(|projection| projection.to_patch_id.clone())
        .or_else(|| raw_projection.to_patch_id.clone())
        .or_else(|| projected_patch_id);
    if projected_from_patch_id.is_some() || projected_to_patch_id.is_some() {
        apply_patch_range_override(
            &mut patch_filter,
            projected_from_patch_id.as_deref(),
            projected_to_patch_id.as_deref(),
        );
    }
}

fn raw_projection_from_input(
    input: &crate::bridge::contract::FishyMapInputState,
) -> FishyMapSearchProjection {
    FishyMapSearchProjection {
        fish_ids: crate::bridge::contract::normalize_i32_list(input.filters.fish_ids.clone()),
        zone_rgbs: crate::bridge::contract::normalize_u32_list(input.filters.zone_rgbs.clone()),
        semantic_field_ids_by_layer: crate::bridge::contract::normalize_u32_map(
            input.filters.semantic_field_ids_by_layer.clone(),
        ),
        fish_filter_terms: normalize_fish_filter_terms(input.filters.fish_filter_terms.clone()),
        patch_id: input.filters.patch_id.clone(),
        from_patch_id: input.filters.from_patch_id.clone(),
        to_patch_id: input.filters.to_patch_id.clone(),
    }
}

fn zone_catalog_rgbs(
    layer_registry: &LayerRegistry,
    field_metadata: &FieldMetadataCache,
) -> Vec<u32> {
    let Some(layer) = layer_registry.get_by_key(SemanticFieldFilterState::ZONE_MASK_LAYER_ID)
    else {
        return Vec::new();
    };
    let Some(metadata_url) = layer.field_metadata_url() else {
        return Vec::new();
    };
    let Some(metadata) = field_metadata.get(layer.id, &metadata_url) else {
        return Vec::new();
    };
    metadata.entries.keys().copied().collect()
}

fn resolve_effective_fish_ids(
    selected_fish_ids: &[i32],
    filter_terms: &[String],
    expression: &FishyMapSearchExpressionNode,
    shared_fish_state: &FishyMapSharedFishState,
    catalog_fish: &[FishEntry],
) -> Vec<i32> {
    let selected_fish_ids = crate::bridge::contract::normalize_i32_list(selected_fish_ids.to_vec());
    let filter_terms = normalize_fish_filter_terms(filter_terms.to_vec());
    let fish_search_expression =
        project_search_expression(expression, SearchExpressionProjectionMode::Fish);
    let can_use_search_expression =
        fish_search_expression.representable && fish_search_expression.expression.is_some();
    if filter_terms.is_empty() && !can_use_search_expression {
        return selected_fish_ids;
    }

    if catalog_fish.is_empty() {
        if can_use_search_expression {
            return vec![FISH_FILTER_NO_MATCH_SENTINEL_ID];
        }
        return if selected_fish_ids.is_empty() {
            vec![FISH_FILTER_NO_MATCH_SENTINEL_ID]
        } else {
            selected_fish_ids
        };
    }

    let matching_fish_ids = catalog_fish
        .iter()
        .filter_map(|fish| {
            let fish_id = fish.id;
            if fish_id <= 0 {
                return None;
            }
            let matches = if let Some(fish_expression) = fish_search_expression.expression.as_ref()
            {
                fish_matches_search_expression(fish, fish_expression, shared_fish_state)
            } else {
                fish_matches_shared_filter_terms(fish, &filter_terms, shared_fish_state)
            };
            matches.then_some(fish_id)
        })
        .collect::<Vec<_>>();

    if can_use_search_expression {
        return if matching_fish_ids.is_empty() {
            vec![FISH_FILTER_NO_MATCH_SENTINEL_ID]
        } else {
            matching_fish_ids
        };
    }

    if selected_fish_ids.is_empty() {
        return if matching_fish_ids.is_empty() {
            vec![FISH_FILTER_NO_MATCH_SENTINEL_ID]
        } else {
            matching_fish_ids
        };
    }

    if filter_terms
        .iter()
        .any(|term| term == "favourite" || term == "missing")
    {
        let mut combined = selected_fish_ids;
        combined.extend(matching_fish_ids);
        let combined = crate::bridge::contract::normalize_i32_list(combined);
        return if combined.is_empty() {
            vec![FISH_FILTER_NO_MATCH_SENTINEL_ID]
        } else {
            combined
        };
    }

    let matching_set = matching_fish_ids.into_iter().collect::<HashSet<_>>();
    let effective_fish_ids = selected_fish_ids
        .into_iter()
        .filter(|fish_id| matching_set.contains(fish_id))
        .collect::<Vec<_>>();
    if effective_fish_ids.is_empty() {
        vec![FISH_FILTER_NO_MATCH_SENTINEL_ID]
    } else {
        effective_fish_ids
    }
}

fn resolve_effective_zone_rgbs(
    selected_zone_rgbs: &[u32],
    expression: &FishyMapSearchExpressionNode,
    zone_catalog_rgbs: &[u32],
) -> Vec<u32> {
    let selected_zone_rgbs =
        crate::bridge::contract::normalize_u32_list(selected_zone_rgbs.to_vec());
    let zone_search_expression =
        project_search_expression(expression, SearchExpressionProjectionMode::Zone);
    let can_use_search_expression =
        zone_search_expression.representable && zone_search_expression.expression.is_some();
    if !can_use_search_expression || zone_catalog_rgbs.is_empty() {
        return selected_zone_rgbs;
    }
    crate::bridge::contract::normalize_u32_list(
        zone_catalog_rgbs
            .iter()
            .copied()
            .filter(|zone_rgb| {
                zone_matches_search_expression(
                    *zone_rgb,
                    zone_search_expression
                        .expression
                        .as_ref()
                        .expect("zone expression"),
                )
            })
            .collect(),
    )
}

fn fish_matches_shared_filter_terms(
    fish: &FishEntry,
    filter_terms: &[String],
    shared_fish_state: &FishyMapSharedFishState,
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
    let caught_set = shared_fish_state
        .caught_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let favourite_set = shared_fish_state
        .favourite_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    for term in filter_terms {
        match term.as_str() {
            "red" | "yellow" | "blue" | "green" | "white" => {}
            "favourite" => {
                if !fish_identity_ids
                    .iter()
                    .any(|fish_id| favourite_set.contains(fish_id))
                {
                    return false;
                }
            }
            "missing" => {
                if fish_identity_ids
                    .iter()
                    .any(|fish_id| caught_set.contains(fish_id))
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

fn fish_matches_search_expression(
    fish: &FishEntry,
    expression: &FishyMapSearchExpressionNode,
    shared_fish_state: &FishyMapSharedFishState,
) -> bool {
    match expression {
        FishyMapSearchExpressionNode::Term { term, negated } => {
            let result = fish_matches_search_term(fish, term, shared_fish_state);
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
                    .all(|child| fish_matches_search_expression(fish, child, shared_fish_state)),
                FishyMapSearchExpressionOperator::Or => children
                    .iter()
                    .any(|child| fish_matches_search_expression(fish, child, shared_fish_state)),
            };
            if *negated {
                !result
            } else {
                result
            }
        }
    }
}

fn fish_matches_search_term(
    fish: &FishEntry,
    term: &FishyMapSearchTerm,
    shared_fish_state: &FishyMapSharedFishState,
) -> bool {
    match term {
        FishyMapSearchTerm::Fish { fish_id } => fish_identity_ids(fish).contains(fish_id),
        FishyMapSearchTerm::FishFilter { term } => {
            fish_matches_shared_filter_terms(fish, std::slice::from_ref(term), shared_fish_state)
        }
        FishyMapSearchTerm::PatchBound { .. }
        | FishyMapSearchTerm::Zone { .. }
        | FishyMapSearchTerm::Semantic { .. } => false,
    }
}

fn zone_matches_search_expression(
    zone_rgb: u32,
    expression: &FishyMapSearchExpressionNode,
) -> bool {
    match expression {
        FishyMapSearchExpressionNode::Term { term, negated } => {
            let result = matches!(term, FishyMapSearchTerm::Zone { zone_rgb: target } if *target == zone_rgb);
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
                    .all(|child| zone_matches_search_expression(zone_rgb, child)),
                FishyMapSearchExpressionOperator::Or => children
                    .iter()
                    .any(|child| zone_matches_search_expression(zone_rgb, child)),
            };
            if *negated {
                !result
            } else {
                result
            }
        }
    }
}

fn project_search_expression(
    expression: &FishyMapSearchExpressionNode,
    mode: SearchExpressionProjectionMode,
) -> ProjectedSearchExpression {
    match expression {
        FishyMapSearchExpressionNode::Term { term, negated } => match term {
            FishyMapSearchTerm::PatchBound { .. } => match mode {
                SearchExpressionProjectionMode::Fish | SearchExpressionProjectionMode::Zone => {
                    ProjectedSearchExpression {
                        representable: true,
                        expression: None,
                        orthogonal: true,
                    }
                }
            },
            FishyMapSearchTerm::Fish { .. } | FishyMapSearchTerm::FishFilter { .. } => match mode {
                SearchExpressionProjectionMode::Fish => ProjectedSearchExpression {
                    representable: true,
                    expression: Some(expression.clone()),
                    orthogonal: false,
                },
                SearchExpressionProjectionMode::Zone => ProjectedSearchExpression {
                    representable: true,
                    expression: None,
                    orthogonal: true,
                },
            },
            FishyMapSearchTerm::Zone { .. } => match mode {
                SearchExpressionProjectionMode::Fish => ProjectedSearchExpression {
                    representable: !*negated,
                    expression: None,
                    orthogonal: false,
                },
                SearchExpressionProjectionMode::Zone => ProjectedSearchExpression {
                    representable: true,
                    expression: Some(expression.clone()),
                    orthogonal: false,
                },
            },
            FishyMapSearchTerm::Semantic { .. } => match mode {
                SearchExpressionProjectionMode::Fish => ProjectedSearchExpression {
                    representable: !*negated,
                    expression: None,
                    orthogonal: false,
                },
                SearchExpressionProjectionMode::Zone => ProjectedSearchExpression {
                    representable: true,
                    expression: None,
                    orthogonal: true,
                },
            },
        },
        FishyMapSearchExpressionNode::Group {
            operator,
            children,
            negated,
        } => {
            let projected_children = children
                .iter()
                .map(|child| project_search_expression(child, mode))
                .collect::<Vec<_>>();
            if projected_children.iter().any(|child| !child.representable) {
                return ProjectedSearchExpression {
                    representable: false,
                    expression: None,
                    orthogonal: false,
                };
            }
            let non_orthogonal_children = projected_children
                .iter()
                .filter(|child| !child.orthogonal)
                .cloned()
                .collect::<Vec<_>>();
            let kept_children = non_orthogonal_children
                .iter()
                .filter_map(|child| child.expression.clone())
                .collect::<Vec<_>>();
            if *negated && kept_children.len() != non_orthogonal_children.len() {
                return ProjectedSearchExpression {
                    representable: false,
                    expression: None,
                    orthogonal: false,
                };
            }
            if matches!(operator, FishyMapSearchExpressionOperator::Or)
                && !kept_children.is_empty()
                && kept_children.len() != non_orthogonal_children.len()
            {
                return ProjectedSearchExpression {
                    representable: false,
                    expression: None,
                    orthogonal: false,
                };
            }
            ProjectedSearchExpression {
                representable: true,
                expression: build_projected_expression_node(*operator, kept_children, *negated),
                orthogonal: false,
            }
        }
    }
}

fn build_projected_expression_node(
    operator: FishyMapSearchExpressionOperator,
    children: Vec<FishyMapSearchExpressionNode>,
    negated: bool,
) -> Option<FishyMapSearchExpressionNode> {
    let mut children = children;
    if children.is_empty() {
        return None;
    }
    if children.len() == 1 {
        let mut child = children.remove(0);
        if negated {
            match &mut child {
                FishyMapSearchExpressionNode::Term { negated, .. }
                | FishyMapSearchExpressionNode::Group { negated, .. } => {
                    *negated = !*negated;
                }
            }
        }
        return Some(child);
    }
    Some(FishyMapSearchExpressionNode::Group {
        operator,
        children,
        negated,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        project_search_expression, resolve_effective_fish_ids, resolve_effective_zone_rgbs,
        SearchExpressionProjectionMode,
    };
    use crate::bridge::contract::{
        FishyMapSearchExpressionNode, FishyMapSearchExpressionOperator, FishyMapSearchTerm,
        FishyMapSharedFishState,
    };
    use crate::plugins::api::FishEntry;

    fn fish(id: i32, item_id: i32, name: &str, grade: Option<&str>, is_prize: bool) -> FishEntry {
        FishEntry {
            id,
            item_id,
            encyclopedia_key: None,
            encyclopedia_id: None,
            name: name.to_string(),
            name_lower: name.to_lowercase(),
            grade: grade.map(str::to_string),
            is_prize,
        }
    }

    #[test]
    fn fish_resolution_matches_boolean_expression_semantics() {
        let expression = FishyMapSearchExpressionNode::Group {
            operator: FishyMapSearchExpressionOperator::Or,
            children: vec![
                FishyMapSearchExpressionNode::Group {
                    operator: FishyMapSearchExpressionOperator::And,
                    children: vec![
                        FishyMapSearchExpressionNode::Term {
                            term: FishyMapSearchTerm::FishFilter {
                                term: "favourite".to_string(),
                            },
                            negated: false,
                        },
                        FishyMapSearchExpressionNode::Term {
                            term: FishyMapSearchTerm::FishFilter {
                                term: "missing".to_string(),
                            },
                            negated: false,
                        },
                    ],
                    negated: false,
                },
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::FishFilter {
                        term: "red".to_string(),
                    },
                    negated: false,
                },
            ],
            negated: false,
        };

        assert_eq!(
            resolve_effective_fish_ids(
                &[],
                &[
                    "favourite".to_string(),
                    "missing".to_string(),
                    "red".to_string()
                ],
                &expression,
                &FishyMapSharedFishState {
                    caught_ids: vec![912],
                    favourite_ids: vec![77],
                },
                &[
                    fish(61, 6100, "Ancient Relic Crystal Shard", Some("Prize"), true),
                    fish(77, 77, "Serendia Carp", Some("General"), false),
                    fish(912, 912, "Cron Dart", Some("Rare"), false),
                ],
            ),
            vec![61, 77]
        );
    }

    #[test]
    fn zone_resolution_matches_boolean_expression_semantics() {
        let expression = FishyMapSearchExpressionNode::Group {
            operator: FishyMapSearchExpressionOperator::And,
            children: vec![
                FishyMapSearchExpressionNode::Group {
                    operator: FishyMapSearchExpressionOperator::Or,
                    children: vec![
                        FishyMapSearchExpressionNode::Term {
                            term: FishyMapSearchTerm::Zone { zone_rgb: 0x123456 },
                            negated: false,
                        },
                        FishyMapSearchExpressionNode::Term {
                            term: FishyMapSearchTerm::Zone { zone_rgb: 0x654321 },
                            negated: false,
                        },
                    ],
                    negated: false,
                },
                FishyMapSearchExpressionNode::Group {
                    operator: FishyMapSearchExpressionOperator::Or,
                    children: vec![FishyMapSearchExpressionNode::Term {
                        term: FishyMapSearchTerm::Zone { zone_rgb: 0x654321 },
                        negated: false,
                    }],
                    negated: true,
                },
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::FishFilter {
                        term: "red".to_string(),
                    },
                    negated: false,
                },
            ],
            negated: false,
        };

        assert_eq!(
            resolve_effective_zone_rgbs(&[], &expression, &[0x123456, 0x654321, 0x777777],),
            vec![0x123456]
        );
    }

    #[test]
    fn fish_projection_rejects_negated_zone_terms() {
        let expression = FishyMapSearchExpressionNode::Group {
            operator: FishyMapSearchExpressionOperator::Or,
            children: vec![FishyMapSearchExpressionNode::Term {
                term: FishyMapSearchTerm::Zone { zone_rgb: 123 },
                negated: true,
            }],
            negated: false,
        };

        let projected =
            project_search_expression(&expression, SearchExpressionProjectionMode::Fish);

        assert!(!projected.representable);
        assert!(projected.expression.is_none());
    }
}
