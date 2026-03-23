use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::field::DiscreteFieldRows;
use crate::gamecommondata::OriginalRegionLayerContext;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FieldHoverMetadataAsset {
    pub entries: BTreeMap<u32, FieldHoverMetadataEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FieldHoverMetadataEntry {
    pub region_id: Option<u32>,
    pub region_group: Option<u32>,
    pub region_name: Option<String>,
    pub resource_bar_waypoint: Option<u32>,
    pub resource_bar_world_x: Option<f64>,
    pub resource_bar_world_z: Option<f64>,
    pub origin_waypoint: Option<u32>,
    pub origin_world_x: Option<f64>,
    pub origin_world_z: Option<f64>,
}

impl FieldHoverMetadataAsset {
    pub fn entry(&self, id: u32) -> Option<&FieldHoverMetadataEntry> {
        self.entries.get(&id)
    }
}

impl FieldHoverMetadataEntry {
    pub fn has_value(&self) -> bool {
        self.region_id.is_some()
            || self.region_group.is_some()
            || self.region_name.is_some()
            || self.resource_bar_waypoint.is_some()
            || self.resource_bar_world_x.is_some()
            || self.resource_bar_world_z.is_some()
            || self.origin_waypoint.is_some()
            || self.origin_world_x.is_some()
            || self.origin_world_z.is_some()
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
    context: &OriginalRegionLayerContext,
) -> FieldHoverMetadataAsset {
    build_hover_metadata(field, |id| context.resolve_region_group_hover_metadata(id))
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
