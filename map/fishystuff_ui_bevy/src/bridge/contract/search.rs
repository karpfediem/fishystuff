use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use super::normalize::{normalize_i32_list, normalize_u32_list, normalize_u32_map};

const DEFAULT_SEARCH_EXPRESSION_OPERATOR: FishyMapSearchExpressionOperator =
    FishyMapSearchExpressionOperator::Or;
const FISH_FILTER_TERM_ORDER: [&str; 7] = [
    "favourite",
    "missing",
    "red",
    "yellow",
    "blue",
    "green",
    "white",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FishyMapSearchExpressionOperator {
    And,
    #[default]
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FishyMapPatchBound {
    From,
    To,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FishyMapSearchTerm {
    PatchBound {
        bound: FishyMapPatchBound,
        #[serde(rename = "patchId", skip_serializing_if = "Option::is_none")]
        patch_id: Option<String>,
    },
    FishFilter {
        term: String,
    },
    Fish {
        #[serde(rename = "fishId")]
        fish_id: i32,
    },
    Zone {
        #[serde(rename = "zoneRgb")]
        zone_rgb: u32,
    },
    Semantic {
        #[serde(rename = "layerId")]
        layer_id: String,
        #[serde(rename = "fieldId")]
        field_id: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FishyMapSearchExpressionNode {
    Term {
        term: FishyMapSearchTerm,
        #[serde(default, skip_serializing_if = "is_false")]
        negated: bool,
    },
    Group {
        operator: FishyMapSearchExpressionOperator,
        children: Vec<FishyMapSearchExpressionNode>,
        #[serde(default, skip_serializing_if = "is_false")]
        negated: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapSharedFishState {
    pub caught_ids: Vec<i32>,
    pub favourite_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FishyMapSearchProjection {
    pub fish_ids: Vec<i32>,
    pub zone_rgbs: Vec<u32>,
    pub semantic_field_ids_by_layer: BTreeMap<String, Vec<u32>>,
    pub fish_filter_terms: Vec<String>,
    pub patch_id: Option<String>,
    pub from_patch_id: Option<String>,
    pub to_patch_id: Option<String>,
}

impl Default for FishyMapSearchExpressionNode {
    fn default() -> Self {
        Self::Group {
            operator: DEFAULT_SEARCH_EXPRESSION_OPERATOR,
            children: Vec::new(),
            negated: false,
        }
    }
}

impl FishyMapSharedFishState {
    pub fn normalize(mut self) -> Self {
        self.caught_ids = normalize_positive_i32_list(self.caught_ids);
        self.favourite_ids = normalize_positive_i32_list(self.favourite_ids);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.caught_ids.is_empty() && self.favourite_ids.is_empty()
    }
}

impl FishyMapSearchExpressionNode {
    pub fn is_empty(&self) -> bool {
        matches!(
            self,
            FishyMapSearchExpressionNode::Group {
                children,
                negated: false,
                ..
            } if children.is_empty()
        )
    }
}

impl FishyMapSearchProjection {
    pub fn from_expression(expression: &FishyMapSearchExpressionNode) -> Self {
        Self::from_terms(&selected_search_terms_from_expression(expression))
    }

    pub fn from_terms(terms: &[FishyMapSearchTerm]) -> Self {
        let mut projection = Self::default();
        for term in terms {
            match term {
                FishyMapSearchTerm::PatchBound { bound, patch_id } => {
                    let Some(patch_id) = patch_id.as_ref().filter(|value| !value.is_empty()) else {
                        continue;
                    };
                    match bound {
                        FishyMapPatchBound::From => {
                            projection.from_patch_id = Some(patch_id.clone());
                        }
                        FishyMapPatchBound::To => {
                            projection.to_patch_id = Some(patch_id.clone());
                        }
                    }
                }
                FishyMapSearchTerm::FishFilter { term } => {
                    projection.fish_filter_terms.push(term.clone());
                }
                FishyMapSearchTerm::Fish { fish_id } => {
                    projection.fish_ids.push(*fish_id);
                }
                FishyMapSearchTerm::Zone { zone_rgb } => {
                    projection.zone_rgbs.push(*zone_rgb);
                }
                FishyMapSearchTerm::Semantic { layer_id, field_id } => {
                    projection
                        .semantic_field_ids_by_layer
                        .entry(layer_id.clone())
                        .or_default()
                        .push(*field_id);
                }
            }
        }
        if !projection.zone_rgbs.is_empty() {
            projection
                .semantic_field_ids_by_layer
                .insert("zone_mask".to_string(), projection.zone_rgbs.clone());
        }
        projection.fish_ids = normalize_i32_list(projection.fish_ids);
        projection.zone_rgbs = normalize_u32_list(projection.zone_rgbs);
        projection.semantic_field_ids_by_layer =
            normalize_u32_map(projection.semantic_field_ids_by_layer);
        projection.fish_filter_terms = normalize_fish_filter_terms(projection.fish_filter_terms);
        projection.patch_id = match (&projection.from_patch_id, &projection.to_patch_id) {
            (Some(from_patch_id), Some(to_patch_id)) if from_patch_id == to_patch_id => {
                Some(from_patch_id.clone())
            }
            _ => None,
        };
        projection
    }
}

pub fn normalize_fish_filter_term(value: impl AsRef<str>) -> String {
    let normalized = value.as_ref().trim().to_lowercase();
    match normalized.as_str() {
        "favourite" | "favourites" | "favorite" | "favorites" => "favourite".to_string(),
        "missing" | "uncaught" | "not caught" | "not yet caught" => "missing".to_string(),
        "red" | "prize" => "red".to_string(),
        "yellow" | "rare" => "yellow".to_string(),
        "blue" | "highquality" | "high_quality" | "high-quality" => "blue".to_string(),
        "green" | "general" => "green".to_string(),
        "white" | "trash" => "white".to_string(),
        _ => String::new(),
    }
}

pub fn normalize_fish_filter_terms(values: Vec<String>) -> Vec<String> {
    let mut selected = BTreeSet::new();
    for value in values {
        let normalized = normalize_fish_filter_term(value);
        if !normalized.is_empty() {
            selected.insert(normalized);
        }
    }
    FISH_FILTER_TERM_ORDER
        .iter()
        .filter(|term| selected.contains(**term))
        .map(|term| (*term).to_string())
        .collect()
}

pub fn deserialize_search_expression_field<'de, D>(
    deserializer: D,
) -> Result<Option<FishyMapSearchExpressionNode>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value.map(|value| normalize_search_expression_value(&value).unwrap_or_default()))
}

pub fn deserialize_search_expression_state<'de, D>(
    deserializer: D,
) -> Result<FishyMapSearchExpressionNode, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value
        .and_then(|value| normalize_search_expression_value(&value))
        .unwrap_or_default())
}

pub fn selected_search_terms_from_expression(
    expression: &FishyMapSearchExpressionNode,
) -> Vec<FishyMapSearchTerm> {
    let mut selected = Vec::new();
    let mut seen = BTreeSet::new();
    collect_selected_search_terms(expression, &mut selected, &mut seen);
    selected
}

fn collect_selected_search_terms(
    node: &FishyMapSearchExpressionNode,
    selected: &mut Vec<FishyMapSearchTerm>,
    seen: &mut BTreeSet<String>,
) {
    match node {
        FishyMapSearchExpressionNode::Term { term, .. } => {
            let key = search_term_key(term);
            if key.is_empty() || !seen.insert(key) {
                return;
            }
            selected.push(term.clone());
        }
        FishyMapSearchExpressionNode::Group { children, .. } => {
            for child in children {
                collect_selected_search_terms(child, selected, seen);
            }
        }
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn normalize_positive_i32_list(values: Vec<i32>) -> Vec<i32> {
    let mut out = Vec::new();
    for value in normalize_i32_list(values) {
        if value > 0 {
            out.push(value);
        }
    }
    out
}

fn normalize_patch_bound(value: Option<&Value>) -> Option<FishyMapPatchBound> {
    let normalized = value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    match normalized.as_str() {
        "from" | "start" | "since" => Some(FishyMapPatchBound::From),
        "to" | "until" | "end" | "through" => Some(FishyMapPatchBound::To),
        _ => None,
    }
}

fn normalize_patch_id_value(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_search_expression_operator(value: Option<&Value>) -> FishyMapSearchExpressionOperator {
    match value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_lowercase()
        .as_str()
    {
        "and" => FishyMapSearchExpressionOperator::And,
        _ => DEFAULT_SEARCH_EXPRESSION_OPERATOR,
    }
}

fn normalize_negated(value: Option<&Value>) -> bool {
    value.and_then(Value::as_bool).unwrap_or(false)
}

fn normalize_search_term_value(raw: &Value) -> Option<FishyMapSearchTerm> {
    let object = raw.as_object()?;
    let kind = object
        .get("kind")
        .or_else(|| object.get("type"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    match kind.as_str() {
        "patch-bound" | "patch" => {
            let bound = normalize_patch_bound(
                object
                    .get("bound")
                    .or_else(|| object.get("patchBound"))
                    .or_else(|| object.get("side")),
            )?;
            let patch_id = normalize_patch_id_value(
                object
                    .get("patchId")
                    .or_else(|| object.get("value"))
                    .or_else(|| object.get("id")),
            );
            Some(FishyMapSearchTerm::PatchBound { bound, patch_id })
        }
        "fish-filter" => {
            let term = normalize_fish_filter_term(
                object
                    .get("term")
                    .or_else(|| object.get("fishFilterTerm"))
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            );
            (!term.is_empty()).then_some(FishyMapSearchTerm::FishFilter { term })
        }
        "fish" => {
            let fish_id = object
                .get("fishId")
                .or_else(|| object.get("itemId"))
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok())
                .filter(|value| *value > 0)?;
            Some(FishyMapSearchTerm::Fish { fish_id })
        }
        "zone" => {
            let zone_rgb = object
                .get("zoneRgb")
                .or_else(|| object.get("fieldId"))
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .filter(|value| *value > 0)?;
            Some(FishyMapSearchTerm::Zone { zone_rgb })
        }
        "semantic" => {
            let layer_id = object
                .get("layerId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            let field_id = object
                .get("fieldId")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .filter(|value| *value > 0)?;
            if layer_id == "zone_mask" {
                return Some(FishyMapSearchTerm::Zone { zone_rgb: field_id });
            }
            (!layer_id.is_empty()).then_some(FishyMapSearchTerm::Semantic { layer_id, field_id })
        }
        _ => None,
    }
}

fn normalize_search_expression_node_value(raw: &Value) -> Option<FishyMapSearchExpressionNode> {
    let object = raw.as_object()?;
    let node_type = object
        .get("type")
        .or_else(|| object.get("nodeType"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_lowercase();

    if node_type == "term" {
        let term = normalize_search_term_value(
            object
                .get("term")
                .or_else(|| object.get("value"))
                .or_else(|| object.get("searchTerm"))?,
        )?;
        return Some(FishyMapSearchExpressionNode::Term {
            term,
            negated: normalize_negated(
                object
                    .get("negated")
                    .or_else(|| object.get("not"))
                    .or_else(|| object.get("inverted")),
            ),
        });
    }

    let child_values = object
        .get("children")
        .or_else(|| object.get("items"))
        .or_else(|| object.get("nodes"))
        .and_then(Value::as_array);

    if node_type == "group" || child_values.is_some() {
        let mut children = Vec::new();
        let mut seen = BTreeSet::new();
        for child_value in child_values.into_iter().flatten() {
            let Some(child) = normalize_search_expression_node_value(child_value) else {
                continue;
            };
            let key = search_expression_node_key(&child);
            if key.is_empty() || !seen.insert(key) {
                continue;
            }
            children.push(child);
        }
        return Some(FishyMapSearchExpressionNode::Group {
            operator: normalize_search_expression_operator(
                object.get("operator").or_else(|| object.get("op")),
            ),
            children,
            negated: normalize_negated(
                object
                    .get("negated")
                    .or_else(|| object.get("not"))
                    .or_else(|| object.get("inverted")),
            ),
        });
    }

    let term = normalize_search_term_value(raw)?;
    Some(FishyMapSearchExpressionNode::Term {
        term,
        negated: normalize_negated(
            object
                .get("negated")
                .or_else(|| object.get("not"))
                .or_else(|| object.get("inverted")),
        ),
    })
}

fn normalize_search_expression_value(raw: &Value) -> Option<FishyMapSearchExpressionNode> {
    let node = normalize_search_expression_node_value(raw)?;
    match node {
        FishyMapSearchExpressionNode::Group { .. } => Some(node),
        FishyMapSearchExpressionNode::Term { .. } => Some(FishyMapSearchExpressionNode::Group {
            operator: DEFAULT_SEARCH_EXPRESSION_OPERATOR,
            children: vec![node],
            negated: false,
        }),
    }
}

fn search_expression_node_key(node: &FishyMapSearchExpressionNode) -> String {
    match node {
        FishyMapSearchExpressionNode::Term { term, negated } => {
            let prefix = if *negated { "not:" } else { "" };
            format!("term:{}{}", prefix, search_term_key(term))
        }
        FishyMapSearchExpressionNode::Group {
            operator,
            children,
            negated,
        } => {
            let prefix = if *negated { "not:" } else { "" };
            let operator = match operator {
                FishyMapSearchExpressionOperator::And => "and",
                FishyMapSearchExpressionOperator::Or => "or",
            };
            let child_keys = children
                .iter()
                .map(search_expression_node_key)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join("|");
            format!("group:{}{}:{}", prefix, operator, child_keys)
        }
    }
}

fn search_term_key(term: &FishyMapSearchTerm) -> String {
    match term {
        FishyMapSearchTerm::PatchBound { bound, patch_id } => {
            let bound = match bound {
                FishyMapPatchBound::From => "from",
                FishyMapPatchBound::To => "to",
            };
            format!(
                "patch-bound:{}:{}",
                bound,
                patch_id
                    .clone()
                    .unwrap_or_else(|| "__pending__".to_string())
            )
        }
        FishyMapSearchTerm::FishFilter { term } => format!("fish-filter:{term}"),
        FishyMapSearchTerm::Fish { fish_id } => format!("fish:{fish_id}"),
        FishyMapSearchTerm::Zone { zone_rgb } => format!("zone:{zone_rgb}"),
        FishyMapSearchTerm::Semantic { layer_id, field_id } => {
            format!("semantic:{layer_id}:{field_id}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        deserialize_search_expression_field, normalize_fish_filter_terms,
        selected_search_terms_from_expression, FishyMapPatchBound, FishyMapSearchExpressionNode,
        FishyMapSearchProjection, FishyMapSearchTerm,
    };
    use serde::Deserialize;
    use std::collections::BTreeMap;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SearchExpressionHolder {
        #[serde(default, deserialize_with = "deserialize_search_expression_field")]
        search_expression: Option<FishyMapSearchExpressionNode>,
    }

    #[test]
    fn fish_filter_terms_normalize_to_canonical_order() {
        assert_eq!(
            normalize_fish_filter_terms(vec![
                "general".to_string(),
                "favorite".to_string(),
                "red".to_string(),
                "general".to_string(),
            ]),
            vec![
                "favourite".to_string(),
                "red".to_string(),
                "green".to_string(),
            ]
        );
    }

    #[test]
    fn search_expression_deserializer_normalizes_zone_mask_semantic_terms() {
        let holder: SearchExpressionHolder = serde_json::from_str(
            r#"{
                "searchExpression": {
                    "type": "group",
                    "operator": "or",
                    "children": [
                        {
                            "type": "term",
                            "term": {
                                "kind": "semantic",
                                "layerId": "zone_mask",
                                "fieldId": 1193046
                            }
                        }
                    ]
                }
            }"#,
        )
        .expect("holder");

        assert_eq!(
            holder.search_expression,
            Some(FishyMapSearchExpressionNode::Group {
                operator: super::FishyMapSearchExpressionOperator::Or,
                children: vec![FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::Zone { zone_rgb: 1193046 },
                    negated: false,
                }],
                negated: false,
            })
        );
    }

    #[test]
    fn selected_terms_preserve_unique_preorder_traversal() {
        let expression = FishyMapSearchExpressionNode::Group {
            operator: super::FishyMapSearchExpressionOperator::Or,
            children: vec![
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::FishFilter {
                        term: "red".to_string(),
                    },
                    negated: false,
                },
                FishyMapSearchExpressionNode::Group {
                    operator: super::FishyMapSearchExpressionOperator::And,
                    children: vec![
                        FishyMapSearchExpressionNode::Term {
                            term: FishyMapSearchTerm::FishFilter {
                                term: "red".to_string(),
                            },
                            negated: false,
                        },
                        FishyMapSearchExpressionNode::Term {
                            term: FishyMapSearchTerm::PatchBound {
                                bound: FishyMapPatchBound::From,
                                patch_id: Some("2026-02-26".to_string()),
                            },
                            negated: false,
                        },
                    ],
                    negated: false,
                },
            ],
            negated: false,
        };

        assert_eq!(
            selected_search_terms_from_expression(&expression),
            vec![
                FishyMapSearchTerm::FishFilter {
                    term: "red".to_string(),
                },
                FishyMapSearchTerm::PatchBound {
                    bound: FishyMapPatchBound::From,
                    patch_id: Some("2026-02-26".to_string()),
                },
            ]
        );
    }

    #[test]
    fn search_projection_syncs_zone_mask_and_patch_bounds() {
        let projection = FishyMapSearchProjection::from_terms(&[
            FishyMapSearchTerm::Zone { zone_rgb: 123 },
            FishyMapSearchTerm::Semantic {
                layer_id: "regions".to_string(),
                field_id: 22,
            },
            FishyMapSearchTerm::PatchBound {
                bound: FishyMapPatchBound::From,
                patch_id: Some("2026-02-26".to_string()),
            },
            FishyMapSearchTerm::PatchBound {
                bound: FishyMapPatchBound::To,
                patch_id: Some("2026-03-12".to_string()),
            },
        ]);

        assert_eq!(projection.zone_rgbs, vec![123]);
        assert_eq!(
            projection.semantic_field_ids_by_layer,
            BTreeMap::from([
                ("regions".to_string(), vec![22]),
                ("zone_mask".to_string(), vec![123]),
            ])
        );
        assert_eq!(projection.patch_id, None);
        assert_eq!(projection.from_patch_id.as_deref(), Some("2026-02-26"));
        assert_eq!(projection.to_patch_id.as_deref(), Some("2026-03-12"));
    }
}
