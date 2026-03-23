use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::field::DiscreteFieldRows;
use crate::gamecommondata::OriginalRegionLayerContext;

pub const FIELD_HOVER_ROW_KEY_ZONE: &str = "zone";
pub const FIELD_HOVER_ROW_KEY_RESOURCES: &str = "resources";
pub const FIELD_HOVER_ROW_KEY_ORIGIN: &str = "origin";

pub const FIELD_HOVER_TARGET_KEY_RESOURCE_NODE: &str = "resource_node";
pub const FIELD_HOVER_TARGET_KEY_ORIGIN_NODE: &str = "origin_node";

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

impl FieldHoverMetadataAsset {
    pub fn entry(&self, id: u32) -> Option<&FieldHoverMetadataEntry> {
        self.entries.get(&id)
    }
}

impl FieldHoverMetadataEntry {
    pub fn has_value(&self) -> bool {
        !self.rows.is_empty() || !self.targets.is_empty()
    }

    pub fn row_value(&self, key: &str) -> Option<&str> {
        self.rows
            .iter()
            .find(|row| row.key == key)
            .map(|row| row.value.as_str())
    }
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
