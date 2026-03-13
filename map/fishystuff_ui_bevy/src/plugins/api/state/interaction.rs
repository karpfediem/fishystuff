use fishystuff_api::models::zone_stats::ZoneStatsResponse;

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
    pub rgb: (u8, u8, u8),
    pub rgb_u32: u32,
}

#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub map_px: i32,
    pub map_py: i32,
    pub rgb: Option<(u8, u8, u8)>,
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
    pub rgb: (u8, u8, u8),
    pub rgb_u32: u32,
    pub zone_name: Option<String>,
    pub world_x: f64,
    pub world_z: f64,
}
