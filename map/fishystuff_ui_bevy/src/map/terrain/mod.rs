use crate::prelude::*;
use crate::public_assets::{normalize_public_base_url, resolve_public_asset_url};

pub mod chunks;
pub mod drape;
pub mod height_tiles;
pub mod materials;
pub mod mesh;
pub mod mode;
pub mod runtime;

const DEFAULT_TERRAIN_MANIFEST_PATH: &str = "/images/terrain/v1/manifest.json";
const DEFAULT_TERRAIN_DRAPE_MANIFEST_PATH: &str = "/images/terrain_drape/minimap/v1/manifest.json";
const DEFAULT_TERRAIN_HEIGHT_TILES_PATH: &str = "/images/terrain_height/v1";

fn default_public_asset_url(path: &str) -> String {
    let public_base = normalize_public_base_url(None);
    resolve_public_asset_url(Some(path), public_base.as_deref()).unwrap_or_else(|| path.to_string())
}

pub(crate) fn default_terrain_manifest_url() -> String {
    default_public_asset_url(DEFAULT_TERRAIN_MANIFEST_PATH)
}

pub(crate) fn default_terrain_drape_manifest_url() -> String {
    default_public_asset_url(DEFAULT_TERRAIN_DRAPE_MANIFEST_PATH)
}

pub(crate) fn default_terrain_height_tiles_url() -> String {
    default_public_asset_url(DEFAULT_TERRAIN_HEIGHT_TILES_PATH)
}

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct Terrain3dConfig {
    pub enabled_default: bool,
    pub terrain_manifest_url: String,
    pub map_width: u32,
    pub map_height: u32,
    pub bbox_y_min: f32,
    pub bbox_y_max: f32,
    pub terrain_chunk_requests_per_frame: usize,
    pub terrain_chunk_max_inflight: usize,
    pub terrain_cache_max_chunks: usize,
    pub terrain_pinned_coarse_levels: u8,
    pub terrain_target_chunks_radius: f32,
    pub use_chunk_aligned_drape: bool,
    pub drape_manifest_url: String,
    pub height_tile_root_url: String,
    pub height_tile_size: u32,
    pub height_tile_source_width: u32,
    pub height_tile_source_height: u32,
    pub height_tile_min_tx: i32,
    pub height_tile_max_tx: i32,
    pub height_tile_min_ty: i32,
    pub height_tile_max_ty: i32,
    pub height_tile_flip_y: bool,
    pub height_tile_world_left: f32,
    pub height_tile_world_top: f32,
    pub height_tile_world_units_per_px: f32,
    pub height_tile_cache_max: usize,
    pub height_tile_loads_per_frame: usize,
    pub chunk_map_px: u32,
    pub verts_per_chunk_edge: u32,
    pub build_chunks_per_frame: usize,
    pub build_ms_per_frame: f32,
    pub show_drape: bool,
    pub drape_subdivisions: u32,
    pub drape_builds_per_frame: usize,
    pub drape_offset_base: f32,
    pub drape_offset_per_layer: f32,
}

impl Default for Terrain3dConfig {
    fn default() -> Self {
        Self {
            enabled_default: false,
            terrain_manifest_url: default_terrain_manifest_url(),
            map_width: 11_560,
            map_height: 10_540,
            bbox_y_min: -9_500.0,
            bbox_y_max: 24_000.0,
            terrain_chunk_requests_per_frame: 10,
            terrain_chunk_max_inflight: 24,
            terrain_cache_max_chunks: 2048,
            terrain_pinned_coarse_levels: 2,
            terrain_target_chunks_radius: 6.0,
            use_chunk_aligned_drape: false,
            drape_manifest_url: default_terrain_drape_manifest_url(),
            height_tile_root_url: default_terrain_height_tiles_url(),
            height_tile_size: 512,
            height_tile_source_width: 32_000,
            height_tile_source_height: 27_904,
            height_tile_min_tx: 0,
            height_tile_max_tx: 62,
            height_tile_min_ty: 0,
            height_tile_max_ty: 54,
            height_tile_flip_y: false,
            height_tile_world_left: -1_715_200.0,
            height_tile_world_top: 1_830_300.0,
            height_tile_world_units_per_px: 100.0,
            height_tile_cache_max: 48,
            height_tile_loads_per_frame: 24,
            chunk_map_px: 512,
            verts_per_chunk_edge: 65,
            build_chunks_per_frame: 3,
            build_ms_per_frame: 3.5,
            show_drape: true,
            drape_subdivisions: 8,
            drape_builds_per_frame: 4,
            drape_offset_base: 20.0,
            drape_offset_per_layer: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        default_terrain_drape_manifest_url, default_terrain_height_tiles_url,
        default_terrain_manifest_url, Terrain3dConfig,
    };

    #[test]
    fn default_terrain_urls_resolve_without_api_metadata() {
        assert_eq!(
            default_terrain_manifest_url(),
            "/images/terrain/v1/manifest.json"
        );
        assert_eq!(
            default_terrain_drape_manifest_url(),
            "/images/terrain_drape/minimap/v1/manifest.json"
        );
        assert_eq!(
            default_terrain_height_tiles_url(),
            "/images/terrain_height/v1"
        );

        let config = Terrain3dConfig::default();
        assert_eq!(config.terrain_manifest_url, default_terrain_manifest_url());
        assert_eq!(
            config.drape_manifest_url,
            default_terrain_drape_manifest_url()
        );
        assert_eq!(
            config.height_tile_root_url,
            default_terrain_height_tiles_url()
        );
    }
}
