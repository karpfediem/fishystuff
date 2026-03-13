use serde::{Deserialize, Serialize};

use crate::ids::MapVersionId;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegionGroupsResponse {
    #[serde(default)]
    pub revision: String,
    pub map_version_id: Option<MapVersionId>,
    #[serde(default)]
    pub groups: Vec<RegionGroupDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegionGroupDescriptor {
    pub region_group_id: u32,
    #[serde(default)]
    pub feature_count: u32,
    #[serde(default)]
    pub region_count: u32,
    #[serde(default)]
    pub accessible_region_count: u32,
    pub color_rgb_u32: Option<u32>,
    pub bbox_min_x: Option<f64>,
    pub bbox_min_y: Option<f64>,
    pub bbox_max_x: Option<f64>,
    pub bbox_max_y: Option<f64>,
    pub graph_world_x: Option<f64>,
    pub graph_world_z: Option<f64>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub region_ids: Vec<u32>,
}
