use serde::{Deserialize, Serialize};

use crate::ids::{MapVersionId, PatchId, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetaResponse {
    #[serde(default)]
    pub api_version: String,
    #[serde(default)]
    pub terrain_manifest_url: Option<String>,
    #[serde(default)]
    pub terrain_drape_manifest_url: Option<String>,
    #[serde(default)]
    pub terrain_height_tiles_url: Option<String>,
    #[serde(default)]
    pub canonical_map: CanonicalMapInfo,
    #[serde(default)]
    pub patches: Vec<PatchInfo>,
    pub default_patch: Option<PatchInfo>,
    #[serde(default)]
    pub map_versions: Vec<MapVersionInfo>,
    #[serde(default)]
    pub defaults: MetaDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PatchInfo {
    pub patch_id: PatchId,
    pub start_ts_utc: Timestamp,
    pub patch_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MapVersionInfo {
    pub map_version_id: MapVersionId,
    pub name: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaDefaults {
    pub tile_px: u32,
    pub sigma_tiles: f64,
    pub half_life_days: Option<f64>,
    pub alpha0: f64,
    pub top_k: usize,
    pub map_version_id: Option<MapVersionId>,
}

impl Default for MetaDefaults {
    fn default() -> Self {
        Self {
            tile_px: 32,
            sigma_tiles: 3.0,
            half_life_days: None,
            alpha0: 1.0,
            top_k: 30,
            map_version_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalMapInfo {
    pub left: f64,
    pub right: f64,
    pub bottom: f64,
    pub top: f64,
    pub image_size_x: u32,
    pub image_size_y: u32,
    pub sector_per_pixel: f64,
    pub distance_per_pixel: f64,
    pub world_position_factor: f64,
    pub pixel_center_offset: f64,
}

impl Default for CanonicalMapInfo {
    fn default() -> Self {
        Self {
            left: -160.0,
            right: 112.0,
            bottom: -88.0,
            top: 160.0,
            image_size_x: 11_560,
            image_size_y: 10_540,
            sector_per_pixel: 0.023_529_412_224_888_8,
            distance_per_pixel: 301.176_483_154_296_9,
            world_position_factor: 12_800.0,
            pixel_center_offset: 1.0,
        }
    }
}
