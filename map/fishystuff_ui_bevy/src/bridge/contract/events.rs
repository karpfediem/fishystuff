use serde::Serialize;
use serde_json::Value;

use super::{FishyMapCameraSnapshot, FishyMapHoverLayerSampleSnapshot, FishyMapViewMode};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum FishyMapOutputEvent {
    Ready {
        version: u8,
        capabilities: Vec<String>,
    },
    ViewChanged {
        version: u8,
        view_mode: FishyMapViewMode,
        camera: FishyMapCameraSnapshot,
    },
    SelectionChanged {
        version: u8,
        zone_rgb: Option<u32>,
    },
    HoverChanged {
        version: u8,
        world_x: Option<f64>,
        world_z: Option<f64>,
        zone_rgb: Option<u32>,
        layer_samples: Vec<FishyMapHoverLayerSampleSnapshot>,
    },
    Diagnostic {
        version: u8,
        payload: Value,
    },
}
