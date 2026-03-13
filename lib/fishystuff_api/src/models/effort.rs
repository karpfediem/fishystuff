use serde::{Deserialize, Serialize};

use crate::ids::{MapVersionId, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffortGridRequest {
    pub map_version_id: MapVersionId,
    pub from_ts_utc: Timestamp,
    pub to_ts_utc: Timestamp,
    pub tile_px: u32,
    pub sigma_tiles: f64,
    pub half_life_days: Option<f64>,
    pub ref_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EffortGridResponse {
    pub tile_px: u32,
    pub grid_w: i32,
    pub grid_h: i32,
    pub sigma_tiles: f64,
    #[serde(default)]
    pub values: Vec<f64>,
}
