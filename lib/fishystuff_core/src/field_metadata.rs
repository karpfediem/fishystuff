use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::field::DiscreteFieldRows;
use crate::gamecommondata::OriginalRegionLayerContext;

pub const FIELD_DETAIL_FACT_KEY_ZONE: &str = "zone";
pub const FIELD_DETAIL_FACT_KEY_REGION: &str = "region";
pub const FIELD_DETAIL_FACT_KEY_REGION_NODE: &str = "region_node";
pub const FIELD_DETAIL_FACT_KEY_RESOURCE_GROUP: &str = "resource_group";
pub const FIELD_DETAIL_FACT_KEY_RESOURCE_WAYPOINT: &str = "resource_waypoint";
pub const FIELD_DETAIL_FACT_KEY_RESOURCE_REGION: &str = "resource_region";
pub const FIELD_DETAIL_FACT_KEY_ORIGIN_REGION: &str = "origin_region";
pub const FIELD_DETAIL_FACT_KEY_ORIGIN_NODE: &str = "origin_node";
pub const FIELD_DETAIL_PRIMARY_FACT_KEYS: [&str; 6] = [
    FIELD_DETAIL_FACT_KEY_ZONE,
    FIELD_DETAIL_FACT_KEY_RESOURCE_GROUP,
    FIELD_DETAIL_FACT_KEY_RESOURCE_REGION,
    FIELD_DETAIL_FACT_KEY_ORIGIN_REGION,
    FIELD_DETAIL_FACT_KEY_ORIGIN_NODE,
    FIELD_DETAIL_FACT_KEY_RESOURCE_WAYPOINT,
];

pub const FIELD_HOVER_TARGET_KEY_RESOURCE_NODE: &str = "resource_node";
pub const FIELD_HOVER_TARGET_KEY_ORIGIN_NODE: &str = "origin_node";
pub const FIELD_HOVER_TARGET_KEY_REGION_NODE: &str = "region_node";

pub const FIELD_DETAIL_PANE_ID_ZONE_MASK: &str = "zone_mask";
pub const FIELD_DETAIL_PANE_ID_TERRITORY: &str = "territory";
pub const FIELD_DETAIL_SECTION_KIND_FACTS: &str = "facts";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FieldHoverMetadataAsset {
    pub entries: BTreeMap<u32, FieldHoverMetadataEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FieldHoverMetadataEntry {
    pub targets: Vec<FieldHoverTarget>,
    pub detail_pane: Option<FieldDetailPaneRef>,
    pub detail_sections: Vec<FieldDetailSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FieldHoverTarget {
    pub key: String,
    pub label: String,
    pub world_x: f64,
    pub world_z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct FieldDetailPaneRef {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FieldDetailSection {
    pub id: String,
    pub kind: String,
    pub title: Option<String>,
    pub facts: Vec<FieldDetailFact>,
    pub targets: Vec<FieldHoverTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct FieldDetailFact {
    pub key: String,
    pub label: String,
    pub value: String,
    pub icon: Option<String>,
    pub status_icon: Option<String>,
    pub status_icon_tone: Option<String>,
}

impl FieldHoverMetadataAsset {
    pub fn entry(&self, id: u32) -> Option<&FieldHoverMetadataEntry> {
        self.entries.get(&id)
    }
}

impl FieldHoverMetadataEntry {
    pub fn has_value(&self) -> bool {
        !self.targets.is_empty() || !self.detail_sections.is_empty()
    }

    pub fn fact_value(&self, key: &str) -> Option<&str> {
        detail_fact_value(detail_facts(&self.detail_sections), key)
    }
}

pub fn detail_fact_is_visible(fact: &FieldDetailFact) -> bool {
    !fact.value.trim().is_empty() && !fact.label.trim().is_empty()
}

pub fn detail_facts<'a>(
    sections: &'a [FieldDetailSection],
) -> impl Iterator<Item = &'a FieldDetailFact> + 'a {
    sections.iter().flat_map(|section| section.facts.iter())
}

pub fn preferred_detail_fact<'a>(
    facts: impl IntoIterator<Item = &'a FieldDetailFact>,
) -> Option<&'a FieldDetailFact> {
    let facts = facts.into_iter().collect::<Vec<_>>();
    for key in FIELD_DETAIL_PRIMARY_FACT_KEYS {
        if let Some(fact) = facts
            .iter()
            .copied()
            .find(|fact| fact.key == key && detail_fact_is_visible(fact))
        {
            return Some(fact);
        }
    }
    facts.into_iter().find(|fact| detail_fact_is_visible(fact))
}

pub fn preferred_detail_fact_value<'a>(
    facts: impl IntoIterator<Item = &'a FieldDetailFact>,
) -> Option<&'a str> {
    preferred_detail_fact(facts)
        .map(|fact| fact.value.trim())
        .filter(|value| !value.is_empty())
}

pub fn detail_fact_value<'a>(
    facts: impl IntoIterator<Item = &'a FieldDetailFact>,
    key: &str,
) -> Option<&'a str> {
    facts
        .into_iter()
        .find(|fact| fact.key == key && detail_fact_is_visible(fact))
        .map(|fact| fact.value.trim())
        .filter(|value| !value.is_empty())
}

pub fn build_regions_hover_metadata(
    field: &DiscreteFieldRows,
    context: &OriginalRegionLayerContext,
) -> FieldHoverMetadataAsset {
    build_hover_metadata(field, |id| context.resolve_region_hover_metadata(id))
}

pub fn build_region_groups_hover_metadata(
    field: &DiscreteFieldRows,
    regions_field: &DiscreteFieldRows,
    context: &OriginalRegionLayerContext,
) -> FieldHoverMetadataAsset {
    build_hover_metadata(field, |id| {
        context.resolve_region_group_hover_metadata(id, regions_field)
    })
}

fn build_hover_metadata(
    field: &DiscreteFieldRows,
    resolve: impl Fn(u32) -> Option<FieldHoverMetadataEntry>,
) -> FieldHoverMetadataAsset {
    let mut entries = BTreeMap::new();
    for id in field.unique_nonzero_ids() {
        let Some(entry) = resolve(id) else {
            continue;
        };
        if entry.has_value() {
            entries.insert(id, entry);
        }
    }
    FieldHoverMetadataAsset { entries }
}

#[cfg(test)]
mod tests {
    use super::{
        detail_fact_is_visible, preferred_detail_fact, preferred_detail_fact_value,
        FieldDetailFact, FIELD_DETAIL_FACT_KEY_ORIGIN_REGION, FIELD_DETAIL_FACT_KEY_RESOURCE_GROUP,
        FIELD_DETAIL_FACT_KEY_RESOURCE_REGION, FIELD_DETAIL_FACT_KEY_ZONE,
    };

    fn fact(key: &str, label: &str, value: &str) -> FieldDetailFact {
        FieldDetailFact {
            key: key.to_string(),
            label: label.to_string(),
            value: value.to_string(),
            icon: Some("hover".to_string()),
            status_icon: None,
            status_icon_tone: None,
        }
    }

    #[test]
    fn detail_fact_visibility_requires_value_and_visible_label() {
        assert!(detail_fact_is_visible(&fact(
            FIELD_DETAIL_FACT_KEY_ZONE,
            "Zone",
            "Olvia"
        )));
        assert!(!detail_fact_is_visible(&fact(
            FIELD_DETAIL_FACT_KEY_ZONE,
            "Zone",
            ""
        )));
        assert!(!detail_fact_is_visible(&fact(
            FIELD_DETAIL_FACT_KEY_ZONE,
            "",
            "Olvia"
        )));
    }

    #[test]
    fn preferred_detail_fact_uses_primary_keys_before_first_visible_row() {
        let facts = vec![
            fact("custom", "Custom", "Alpha"),
            fact(
                FIELD_DETAIL_FACT_KEY_ORIGIN_REGION,
                "Origin region",
                "Castle Ruins",
            ),
            fact(
                FIELD_DETAIL_FACT_KEY_RESOURCE_GROUP,
                "Resource group",
                "Olvia (RG12)",
            ),
            fact(
                FIELD_DETAIL_FACT_KEY_RESOURCE_REGION,
                "Region",
                "Olvia (R76)",
            ),
        ];
        assert_eq!(
            preferred_detail_fact(facts.iter()).map(|fact| fact.key.as_str()),
            Some(FIELD_DETAIL_FACT_KEY_RESOURCE_GROUP)
        );
    }

    #[test]
    fn preferred_detail_fact_value_falls_back_to_first_visible_fact() {
        let facts = vec![
            fact("custom", "Custom", "Alpha"),
            fact("other", "Other", "Beta"),
        ];
        assert_eq!(preferred_detail_fact_value(facts.iter()), Some("Alpha"));
    }
}
