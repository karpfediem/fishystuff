use serde::{Deserialize, Serialize};

use crate::ids::{MapVersionId, RgbKey, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneStatsRequest {
    #[serde(default)]
    pub layer_revision_id: Option<String>,
    #[serde(default)]
    pub layer_id: Option<String>,
    #[serde(default)]
    pub patch_id: Option<String>,
    #[serde(default)]
    pub at_ts_utc: Option<Timestamp>,
    #[serde(default)]
    pub map_version_id: Option<MapVersionId>,
    pub rgb: RgbKey,
    pub from_ts_utc: Timestamp,
    pub to_ts_utc: Timestamp,
    pub tile_px: u32,
    pub sigma_tiles: f64,
    #[serde(default)]
    pub fish_norm: bool,
    pub alpha0: f64,
    pub top_k: usize,
    pub half_life_days: Option<f64>,
    pub drift_boundary_ts_utc: Option<Timestamp>,
    pub ref_id: Option<String>,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneStatsResponse {
    pub zone_rgb_u32: u32,
    pub zone_rgb: RgbKey,
    pub zone_name: Option<String>,
    pub window: ZoneStatsWindow,
    pub confidence: ZoneConfidence,
    #[serde(default)]
    pub distribution: Vec<ZoneFishEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneStatsWindow {
    pub from_ts_utc: Timestamp,
    pub to_ts_utc: Timestamp,
    pub half_life_days: Option<f64>,
    pub fish_norm: bool,
    pub tile_px: u32,
    pub sigma_tiles: f64,
    pub alpha0: f64,
}

impl Default for ZoneStatsWindow {
    fn default() -> Self {
        Self {
            from_ts_utc: 0,
            to_ts_utc: 0,
            half_life_days: None,
            fish_norm: false,
            tile_px: 32,
            sigma_tiles: 3.0,
            alpha0: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneConfidence {
    pub ess: f64,
    pub total_weight: f64,
    pub last_seen_ts_utc: Option<Timestamp>,
    pub age_days_last: Option<f64>,
    pub status: ZoneStatus,
    #[serde(default)]
    pub notes: Vec<String>,
    pub drift: Option<DriftInfo>,
}

impl Default for ZoneConfidence {
    fn default() -> Self {
        Self {
            ess: 0.0,
            total_weight: 0.0,
            last_seen_ts_utc: None,
            age_days_last: None,
            status: ZoneStatus::Unknown,
            notes: Vec::new(),
            drift: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ZoneStatus {
    #[default]
    Unknown,
    Stale,
    Fresh,
    Drifting,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneFishEvidence {
    pub fish_id: i32,
    pub fish_name: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    pub evidence_weight: f64,
    pub p_mean: f64,
    pub ci_low: Option<f64>,
    pub ci_high: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DriftInfo {
    pub boundary_ts_utc: Timestamp,
    pub jsd_mean: f64,
    pub p_drift: f64,
    pub ess_old: f64,
    pub ess_new: f64,
    pub samples: usize,
    pub jsd_threshold: f64,
}
