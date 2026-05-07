use serde::Serialize;
use serde_json::Value;

use super::{
    FishyMapCameraSnapshot, FishyMapDetailsTargetSnapshot, FishyMapHoverLayerSampleSnapshot,
    FishyMapPointSampleSnapshot, FishyMapSelectionPointKind, FishyMapViewMode,
};

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
    FirstFrame {
        version: u8,
    },
    ViewChanged {
        version: u8,
        view_mode: FishyMapViewMode,
        camera: FishyMapCameraSnapshot,
    },
    SelectionChanged {
        version: u8,
        details_generation: u64,
        details_target: Option<FishyMapDetailsTargetSnapshot>,
        world_x: Option<f64>,
        world_z: Option<f64>,
        point_kind: Option<FishyMapSelectionPointKind>,
        point_label: Option<String>,
        layer_samples: Vec<FishyMapHoverLayerSampleSnapshot>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        point_samples: Vec<FishyMapPointSampleSnapshot>,
    },
    HoverChanged {
        version: u8,
        world_x: Option<f64>,
        world_z: Option<f64>,
        layer_samples: Vec<FishyMapHoverLayerSampleSnapshot>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        point_samples: Vec<FishyMapPointSampleSnapshot>,
    },
    Diagnostic {
        version: u8,
        payload: Value,
    },
}
