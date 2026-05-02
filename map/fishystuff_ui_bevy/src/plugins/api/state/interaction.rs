use fishystuff_api::models::zone_stats::ZoneStatsResponse;

use crate::bridge::contract::{FishyMapSelectionHistoryBehavior, FishyMapSelectionPointKind};
use crate::map::layer_query::LayerQuerySample;
use crate::map::spaces::WorldPoint;
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
    pub details_generation: u64,
    pub details_target: Option<DetailsSelectionTarget>,
    pub info: Option<SelectedInfo>,
    pub zone_stats: Option<ZoneStatsResponse>,
    pub zone_stats_status: String,
}

impl SelectionState {
    pub fn begin_details_selection(
        &mut self,
        element_kind: impl Into<String>,
        world_point: Option<WorldPoint>,
        point_kind: Option<FishyMapSelectionPointKind>,
        point_label: Option<String>,
        history_behavior: FishyMapSelectionHistoryBehavior,
    ) -> u64 {
        self.details_generation = self.details_generation.saturating_add(1);
        let element_kind = element_kind.into();
        self.details_target = world_point.map(|world_point| DetailsSelectionTarget {
            element_kind,
            world_x: world_point.x,
            world_z: world_point.z,
            point_kind,
            point_label,
            history_behavior,
        });
        self.details_generation
    }
}

impl Default for SelectionState {
    fn default() -> Self {
        Self {
            details_generation: 0,
            details_target: None,
            info: None,
            zone_stats: None,
            zone_stats_status: "zone stats: idle".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetailsSelectionTarget {
    pub element_kind: String,
    pub world_x: f64,
    pub world_z: f64,
    pub point_kind: Option<FishyMapSelectionPointKind>,
    pub point_label: Option<String>,
    pub history_behavior: FishyMapSelectionHistoryBehavior,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoverInfo {
    pub map_px: i32,
    pub map_py: i32,
    pub world_x: f64,
    pub world_z: f64,
    pub layer_samples: Vec<LayerQuerySample>,
    pub point_samples: Vec<PointSampleSummary>,
}

impl HoverInfo {
    pub fn zone_layer_sample(&self) -> Option<&LayerQuerySample> {
        zone_layer_sample(&self.layer_samples)
    }

    pub fn zone_rgb(&self) -> Option<fishystuff_api::Rgb> {
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
    pub world_x: f64,
    pub world_z: f64,
    pub sampled_world_point: bool,
    pub point_kind: Option<FishyMapSelectionPointKind>,
    pub point_label: Option<String>,
    pub layer_samples: Vec<LayerQuerySample>,
    pub point_samples: Vec<PointSampleSummary>,
}

impl SelectedInfo {
    pub fn zone_layer_sample(&self) -> Option<&LayerQuerySample> {
        zone_layer_sample(&self.layer_samples)
    }

    pub fn zone_rgb(&self) -> Option<fishystuff_api::Rgb> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointSampleSummary {
    pub fish_id: i32,
    pub sample_count: u32,
    pub last_ts_utc: i64,
    pub sample_id: Option<i64>,
    pub zone_rgbs: Vec<u32>,
    pub full_zone_rgbs: Vec<u32>,
}
