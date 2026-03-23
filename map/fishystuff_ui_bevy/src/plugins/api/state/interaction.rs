use fishystuff_api::models::zone_stats::ZoneStatsResponse;
use fishystuff_api::Rgb;

use crate::map::layer_query::LayerQuerySample;
use crate::prelude::*;

fn zone_layer_sample(layer_samples: &[LayerQuerySample]) -> Option<&LayerQuerySample> {
    layer_samples
        .iter()
        .find(|sample| sample.layer_id == "zone_mask")
}

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

#[derive(Debug, Clone, PartialEq)]
pub struct HoverInfo {
    pub map_px: i32,
    pub map_py: i32,
    pub rgb: Option<Rgb>,
    pub rgb_u32: Option<u32>,
    pub world_x: f64,
    pub world_z: f64,
    pub layer_samples: Vec<LayerQuerySample>,
}

impl HoverInfo {
    pub fn zone_layer_sample(&self) -> Option<&LayerQuerySample> {
        zone_layer_sample(&self.layer_samples)
    }

    pub fn zone_rgb(&self) -> Option<Rgb> {
        self.zone_layer_sample().map(|sample| sample.rgb)
    }

    pub fn zone_rgb_u32(&self) -> Option<u32> {
        self.zone_layer_sample().map(|sample| sample.rgb_u32)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectedInfo {
    pub map_px: i32,
    pub map_py: i32,
    pub rgb: Option<Rgb>,
    pub rgb_u32: Option<u32>,
    pub world_x: f64,
    pub world_z: f64,
    pub sampled_world_point: bool,
    pub layer_samples: Vec<LayerQuerySample>,
}

impl SelectedInfo {
    pub fn zone_layer_sample(&self) -> Option<&LayerQuerySample> {
        zone_layer_sample(&self.layer_samples)
    }

    pub fn zone_rgb(&self) -> Option<Rgb> {
        self.zone_layer_sample().map(|sample| sample.rgb)
    }

    pub fn zone_rgb_u32(&self) -> Option<u32> {
        self.zone_layer_sample().map(|sample| sample.rgb_u32)
    }

    pub fn effective_world_point(&self) -> Option<(f64, f64)> {
        if !self.sampled_world_point {
            return None;
        }
        if !self.world_x.is_finite() || !self.world_z.is_finite() {
            return None;
        }
        Some((self.world_x, self.world_z))
    }
}
