use fishystuff_api::models::zone_stats::ZoneStatsResponse;
use fishystuff_api::Rgb;

use crate::prelude::*;

#[derive(Resource, Default)]
pub struct HoverState {
    pub info: Option<HoverInfo>,
}

#[derive(Resource)]
pub struct SelectionState {
    pub info: Option<SelectedInfo>,
    pub zone_stats: Option<ZoneStatsResponse>,
    pub zone_stats_status: String,
}

impl Default for SelectionState {
    fn default() -> Self {
        Self {
            info: None,
            zone_stats: None,
            zone_stats_status: "zone stats: idle".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HoverLayerSample {
    pub layer_id: String,
    pub layer_name: String,
    pub kind: String,
    pub rgb: Rgb,
    pub rgb_u32: u32,
    pub region_group: Option<u32>,
    pub region_name: Option<String>,
    pub resource_bar_waypoint: Option<u32>,
    pub resource_bar_world_x: Option<f64>,
    pub resource_bar_world_z: Option<f64>,
    pub origin_waypoint: Option<u32>,
    pub origin_world_x: Option<f64>,
    pub origin_world_z: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub map_px: i32,
    pub map_py: i32,
    pub rgb: Option<Rgb>,
    pub rgb_u32: Option<u32>,
    pub zone_name: Option<String>,
    pub world_x: f64,
    pub world_z: f64,
    pub layer_samples: Vec<HoverLayerSample>,
}

#[derive(Debug, Clone)]
pub struct SelectedInfo {
    pub map_px: i32,
    pub map_py: i32,
    pub rgb: Rgb,
    pub rgb_u32: u32,
    pub zone_name: Option<String>,
    pub world_x: f64,
    pub world_z: f64,
}
