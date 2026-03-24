use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::field::DiscreteFieldRows;
use crate::gamecommondata::OriginalRegionLayerContext;

pub const FIELD_HOVER_ROW_KEY_ZONE: &str = "zone";
pub const FIELD_HOVER_ROW_KEY_RESOURCES: &str = "resources";
pub const FIELD_HOVER_ROW_KEY_ORIGIN: &str = "origin";
pub const FIELD_HOVER_PRIMARY_ROW_KEYS: [&str; 3] = [
    FIELD_HOVER_ROW_KEY_ZONE,
    FIELD_HOVER_ROW_KEY_RESOURCES,
    FIELD_HOVER_ROW_KEY_ORIGIN,
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
    pub rows: Vec<FieldHoverRow>,
    pub targets: Vec<FieldHoverTarget>,
    pub detail_pane: Option<FieldDetailPaneRef>,
    pub detail_sections: Vec<FieldDetailSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct FieldHoverRow {
    pub key: String,
    pub icon: String,
    pub label: String,
    pub value: String,
    pub hide_label: bool,
    pub status_icon: Option<String>,
    pub status_icon_tone: Option<String>,
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
        !self.rows.is_empty() || !self.targets.is_empty() || !self.detail_sections.is_empty()
    }

    pub fn row_value(&self, key: &str) -> Option<&str> {
        self.rows
            .iter()
            .find(|row| row.key == key)
            .map(|row| row.value.as_str())
    }
}

pub fn hover_row_is_visible(row: &FieldHoverRow) -> bool {
    let value = row.value.trim();
    !value.is_empty() && (row.hide_label || !row.label.trim().is_empty())
}

pub fn preferred_hover_row<'a>(
    rows: impl IntoIterator<Item = &'a FieldHoverRow>,
) -> Option<&'a FieldHoverRow> {
    let rows = rows.into_iter().collect::<Vec<_>>();
    for key in FIELD_HOVER_PRIMARY_ROW_KEYS {
        if let Some(row) = rows
            .iter()
            .copied()
            .find(|row| row.key == key && hover_row_is_visible(row))
        {
            return Some(row);
        }
    }
    rows.into_iter().find(|row| hover_row_is_visible(row))
}

pub fn preferred_hover_row_value<'a>(
    rows: impl IntoIterator<Item = &'a FieldHoverRow>,
) -> Option<&'a str> {
    preferred_hover_row(rows)
        .map(|row| row.value.trim())
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
        hover_row_is_visible, preferred_hover_row, preferred_hover_row_value, FieldHoverRow,
        FIELD_HOVER_ROW_KEY_ORIGIN, FIELD_HOVER_ROW_KEY_RESOURCES, FIELD_HOVER_ROW_KEY_ZONE,
    };

    fn row(key: &str, label: &str, value: &str) -> FieldHoverRow {
        FieldHoverRow {
            key: key.to_string(),
            icon: "hover".to_string(),
            label: label.to_string(),
            value: value.to_string(),
            hide_label: false,
            status_icon: None,
            status_icon_tone: None,
        }
    }

    #[test]
    fn hover_row_visibility_requires_value_and_visible_label() {
        assert!(hover_row_is_visible(&row(
            FIELD_HOVER_ROW_KEY_ZONE,
            "Zone",
            "Olvia"
        )));
        assert!(!hover_row_is_visible(&row(
            FIELD_HOVER_ROW_KEY_ZONE,
            "Zone",
            ""
        )));
        assert!(!hover_row_is_visible(&row(
            FIELD_HOVER_ROW_KEY_ZONE,
            "",
            "Olvia"
        )));
    }

    #[test]
    fn preferred_hover_row_uses_primary_keys_before_first_visible_row() {
        let rows = vec![
            row("custom", "Custom", "Alpha"),
            row(FIELD_HOVER_ROW_KEY_ORIGIN, "Origin", "Castle Ruins"),
            row(FIELD_HOVER_ROW_KEY_RESOURCES, "Resources", "Olvia"),
        ];
        assert_eq!(
            preferred_hover_row(rows.iter()).map(|row| row.key.as_str()),
            Some(FIELD_HOVER_ROW_KEY_RESOURCES)
        );
    }

    #[test]
    fn preferred_hover_row_value_falls_back_to_first_visible_row() {
        let rows = vec![
            row("custom", "Custom", "Alpha"),
            row("other", "Other", "Beta"),
        ];
        assert_eq!(preferred_hover_row_value(rows.iter()), Some("Alpha"));
    }
}
