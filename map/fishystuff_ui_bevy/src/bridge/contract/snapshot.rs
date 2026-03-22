use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    default_contract_version, FishyMapFiltersState, FishyMapThemeState, FishyMapUiState,
    FishyMapViewMode, FISHYMAP_CONTRACT_VERSION,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapCameraSnapshot {
    pub center_world_x: Option<f64>,
    pub center_world_z: Option<f64>,
    pub zoom: Option<f64>,
    pub pivot_world_x: Option<f64>,
    pub pivot_world_y: Option<f64>,
    pub pivot_world_z: Option<f64>,
    pub yaw: Option<f64>,
    pub pitch: Option<f64>,
    pub distance: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapViewSnapshot {
    pub view_mode: FishyMapViewMode,
    pub camera: FishyMapCameraSnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapSelectionSnapshot {
    pub zone_rgb: Option<u32>,
    pub zone_name: Option<String>,
    pub world_x: Option<f64>,
    pub world_z: Option<f64>,
    pub zone_stats: Option<FishyMapZoneStatsSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapHoverLayerSampleSnapshot {
    pub layer_id: String,
    pub layer_name: String,
    pub kind: String,
    pub rgb: [u8; 3],
    pub rgb_u32: u32,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapHoverSnapshot {
    pub world_x: Option<f64>,
    pub world_z: Option<f64>,
    pub zone_rgb: Option<u32>,
    pub zone_name: Option<String>,
    pub layer_samples: Vec<FishyMapHoverLayerSampleSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapZoneStatsSnapshot {
    pub zone_rgb: u32,
    pub zone_name: Option<String>,
    pub window: FishyMapZoneWindowSnapshot,
    pub confidence: FishyMapZoneConfidenceSnapshot,
    pub distribution: Vec<FishyMapZoneEvidenceEntrySnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapZoneWindowSnapshot {
    pub from_ts_utc: i64,
    pub to_ts_utc: i64,
    pub half_life_days: Option<f64>,
    pub fish_norm: bool,
    pub tile_px: u32,
    pub sigma_tiles: f64,
    pub alpha0: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapZoneConfidenceSnapshot {
    pub ess: f64,
    pub total_weight: f64,
    pub last_seen_ts_utc: Option<i64>,
    pub age_days_last: Option<f64>,
    pub status: String,
    pub notes: Vec<String>,
    pub drift: Option<FishyMapZoneDriftSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapZoneDriftSnapshot {
    pub boundary_ts_utc: i64,
    pub jsd_mean: f64,
    pub p_drift: f64,
    pub ess_old: f64,
    pub ess_new: f64,
    pub samples: usize,
    pub jsd_threshold: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapZoneEvidenceEntrySnapshot {
    pub fish_id: i32,
    pub item_id: i32,
    pub encyclopedia_key: Option<i32>,
    pub encyclopedia_id: Option<i32>,
    pub fish_name: Option<String>,
    pub evidence_weight: f64,
    pub p_mean: f64,
    pub ci_low: Option<f64>,
    pub ci_high: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapLayerSummary {
    pub layer_id: String,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub opacity_default: f32,
    pub display_order: i32,
    pub kind: String,
    pub visible_tile_count: u32,
    pub resident_tile_count: u32,
    pub pending_count: u32,
    pub inflight_count: u32,
    pub manifest_status: String,
    pub vector_status: String,
    pub vector_progress: f32,
    pub vector_feature_count: u32,
    pub vector_vertex_count: u32,
    pub vector_triangle_count: u32,
    pub vector_mesh_count: u32,
    pub vector_chunked_bucket_count: u32,
    pub vector_build_ms: f32,
    pub vector_last_frame_build_ms: f32,
    pub vector_cache_entries: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapPatchSummary {
    pub patch_id: String,
    pub patch_name: Option<String>,
    pub start_ts_utc: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapFishSummary {
    pub fish_id: i32,
    pub item_id: i32,
    pub encyclopedia_key: Option<i32>,
    pub encyclopedia_id: Option<i32>,
    pub name: String,
    pub grade: Option<String>,
    pub is_prize: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapCatalogSnapshot {
    pub capabilities: Vec<String>,
    pub layers: Vec<FishyMapLayerSummary>,
    pub patches: Vec<FishyMapPatchSummary>,
    pub fish: Vec<FishyMapFishSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapStatusSnapshot {
    pub meta_status: String,
    pub layers_status: String,
    pub zones_status: String,
    pub points_status: String,
    pub fish_status: String,
    pub zone_stats_status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapStateSnapshot {
    #[serde(default = "default_contract_version")]
    pub version: u8,
    pub ready: bool,
    pub theme: FishyMapThemeState,
    pub filters: FishyMapFiltersState,
    pub ui: FishyMapUiState,
    pub view: FishyMapViewSnapshot,
    pub selection: FishyMapSelectionSnapshot,
    pub hover: FishyMapHoverSnapshot,
    pub catalog: FishyMapCatalogSnapshot,
    pub statuses: FishyMapStatusSnapshot,
    pub last_diagnostic: Option<Value>,
}

impl Default for FishyMapStateSnapshot {
    fn default() -> Self {
        Self {
            version: FISHYMAP_CONTRACT_VERSION,
            ready: false,
            theme: FishyMapThemeState::default(),
            filters: FishyMapFiltersState::default(),
            ui: FishyMapUiState::default(),
            view: FishyMapViewSnapshot::default(),
            selection: FishyMapSelectionSnapshot::default(),
            hover: FishyMapHoverSnapshot::default(),
            catalog: FishyMapCatalogSnapshot::default(),
            statuses: FishyMapStatusSnapshot::default(),
            last_diagnostic: None,
        }
    }
}
