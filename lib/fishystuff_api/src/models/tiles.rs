use serde::{Deserialize, Serialize};

use crate::ids::TileSetId;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TilesetManifest {
    pub tileset_id: TileSetId,
    pub tile_px: u32,
    pub root_url: String,
    #[serde(default)]
    pub levels: Vec<TilesetLevelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TilesetLevelInfo {
    pub level: u8,
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
    pub tile_count: usize,
}
