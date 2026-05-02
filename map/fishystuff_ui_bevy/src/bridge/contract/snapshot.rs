use std::collections::BTreeMap;

use fishystuff_core::field_metadata::{FieldDetailPaneRef, FieldDetailSection, FieldHoverTarget};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    default_contract_version, deserialize_search_expression_state, FishyMapFiltersState,
    FishyMapSearchExpressionNode, FishyMapSharedFishState, FishyMapThemeState, FishyMapUiState,
    FishyMapViewMode, FISHYMAP_CONTRACT_VERSION,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum FishyMapSelectionPointKind {
    #[default]
    #[serde(rename = "clicked")]
    Clicked,
    #[serde(rename = "waypoint")]
    Waypoint,
    #[serde(rename = "bookmark")]
    Bookmark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum FishyMapSelectionHistoryBehavior {
    #[default]
    #[serde(rename = "append")]
    Append,
    #[serde(rename = "navigate")]
    Navigate,
}

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
pub struct FishyMapDetailsTargetSnapshot {
    pub element_kind: String,
    pub world_x: f64,
    pub world_z: f64,
    pub point_kind: Option<FishyMapSelectionPointKind>,
    pub point_label: Option<String>,
    pub history_behavior: FishyMapSelectionHistoryBehavior,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapSelectionSnapshot {
    pub details_generation: u64,
    pub details_target: Option<FishyMapDetailsTargetSnapshot>,
    pub world_x: Option<f64>,
    pub world_z: Option<f64>,
    pub point_kind: Option<FishyMapSelectionPointKind>,
    pub point_label: Option<String>,
    pub layer_samples: Vec<FishyMapHoverLayerSampleSnapshot>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub point_samples: Vec<FishyMapPointSampleSnapshot>,
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
    pub field_id: Option<u32>,
    pub targets: Vec<FieldHoverTarget>,
    pub detail_pane: Option<FieldDetailPaneRef>,
    pub detail_sections: Vec<FieldDetailSection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapHoverSnapshot {
    pub world_x: Option<f64>,
    pub world_z: Option<f64>,
    pub layer_samples: Vec<FishyMapHoverLayerSampleSnapshot>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub point_samples: Vec<FishyMapPointSampleSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapPointSampleSnapshot {
    pub fish_id: i32,
    pub sample_count: u32,
    pub last_ts_utc: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_id: Option<i64>,
    pub zone_rgbs: Vec<u32>,
    pub full_zone_rgbs: Vec<u32>,
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
pub struct FishyMapLayerFilterBindingSummary {
    pub binding_id: String,
    pub source: String,
    pub target: String,
    pub enabled: bool,
    pub default_enabled: bool,
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
    pub supports_waypoint_connections: bool,
    pub waypoint_connections_visible: bool,
    pub waypoint_connections_default: bool,
    pub supports_waypoint_labels: bool,
    pub waypoint_labels_visible: bool,
    pub waypoint_labels_default: bool,
    pub supports_point_icons: bool,
    pub point_icons_visible: bool,
    pub point_icons_default: bool,
    pub point_icon_scale: f32,
    pub point_icon_scale_default: f32,
    pub filter_bindings: Vec<FishyMapLayerFilterBindingSummary>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapSemanticTermSummary {
    pub layer_id: String,
    pub layer_name: String,
    pub field_id: u32,
    pub label: String,
    pub description: Option<String>,
    pub search_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapCatalogSnapshot {
    pub capabilities: Vec<String>,
    pub layers: Vec<FishyMapLayerSummary>,
    pub patches: Vec<FishyMapPatchSummary>,
    pub fish: Vec<FishyMapFishSummary>,
    pub semantic_terms: Vec<FishyMapSemanticTermSummary>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapEffectiveZoneMembershipFilterSnapshot {
    pub active: bool,
    pub zone_rgbs: Vec<u32>,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapEffectiveSemanticFieldFilterSnapshot {
    pub active: bool,
    pub field_ids: Vec<u32>,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapEffectiveFiltersSnapshot {
    #[serde(default, deserialize_with = "deserialize_search_expression_state")]
    pub search_expression: FishyMapSearchExpressionNode,
    #[serde(skip_serializing_if = "FishyMapSharedFishState::is_empty")]
    pub shared_fish_state: FishyMapSharedFishState,
    pub zone_membership_by_layer: BTreeMap<String, FishyMapEffectiveZoneMembershipFilterSnapshot>,
    pub semantic_field_filters_by_layer:
        BTreeMap<String, FishyMapEffectiveSemanticFieldFilterSnapshot>,
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
    pub effective_filters: FishyMapEffectiveFiltersSnapshot,
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
            effective_filters: FishyMapEffectiveFiltersSnapshot::default(),
            last_diagnostic: None,
        }
    }
}
