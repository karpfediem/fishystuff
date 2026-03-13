use fishystuff_api::models::layers::LayersResponse;
use fishystuff_api::models::meta::MetaResponse;

use crate::map::layers::{LayerRegistry, LayerRuntime, PickMode};
use crate::map::terrain::Terrain3dConfig;

use super::super::state::{ApiBootstrapState, MapDisplayState, PatchFilterState};
use super::util::{
    absolutize_layers_response_assets, default_from_patch_id, default_from_ts,
    normalize_public_base_url, now_utc_seconds, pick_map_version, resolve_public_asset_url,
};

const LOCAL_TERRAIN_HEIGHT_TILES_FALLBACK: &str = "/images/terrain_height/v1";

pub(super) fn apply_meta_response(
    bootstrap: &mut ApiBootstrapState,
    patch_filter: &mut PatchFilterState,
    terrain_config: &mut Terrain3dConfig,
    meta: MetaResponse,
) {
    let public_base_url = normalize_public_base_url(None);
    if let Some(url) = resolve_public_asset_url(
        meta.terrain_manifest_url.as_deref(),
        public_base_url.as_deref(),
    ) {
        terrain_config.terrain_manifest_url = url;
    } else {
        terrain_config.terrain_manifest_url.clear();
    }
    if let Some(url) = resolve_public_asset_url(
        meta.terrain_drape_manifest_url.as_deref(),
        public_base_url.as_deref(),
    ) {
        terrain_config.drape_manifest_url = url;
    } else {
        terrain_config.drape_manifest_url.clear();
    }
    if let Some(url) = resolve_public_asset_url(
        meta.terrain_height_tiles_url.as_deref(),
        public_base_url.as_deref(),
    ) {
        terrain_config.height_tile_root_url = url;
    }
    if terrain_config.height_tile_root_url.trim().is_empty() {
        terrain_config.height_tile_root_url = resolve_public_asset_url(
            Some(LOCAL_TERRAIN_HEIGHT_TILES_FALLBACK),
            public_base_url.as_deref(),
        )
        .unwrap_or_else(|| LOCAL_TERRAIN_HEIGHT_TILES_FALLBACK.to_string());
    }
    terrain_config.map_width = meta.canonical_map.image_size_x;
    terrain_config.map_height = meta.canonical_map.image_size_y;

    let map_version = pick_map_version(&meta);
    if map_version != bootstrap.map_version {
        bootstrap.map_version_dirty = true;
        bootstrap.layers_loaded_map_version = None;
        bootstrap.layers_next_retry_at_utc = 0;
    }
    bootstrap.meta_status = "meta: loaded".to_string();
    bootstrap.defaults = Some(meta.defaults.clone());
    bootstrap.map_version = map_version;
    patch_filter.from_ts = Some(default_from_ts(&meta));
    patch_filter.to_ts = Some(now_utc_seconds());
    patch_filter.patches = meta.patches.clone();
    bootstrap.meta = Some(meta);
    if patch_filter.selected_patch.is_none() {
        patch_filter.selected_patch = bootstrap.meta.as_ref().and_then(default_from_patch_id);
    }
}

pub(super) fn apply_layers_response(
    bootstrap: &mut ApiBootstrapState,
    display_state: &mut MapDisplayState,
    layer_registry: &mut LayerRegistry,
    layer_runtime: &mut LayerRuntime,
    mut response: LayersResponse,
) {
    let public_base_url = normalize_public_base_url(None);
    absolutize_layers_response_assets(&mut response, public_base_url.as_deref());

    let layer_count = response.layers.len();
    let revision = response.revision.clone();
    layer_registry.apply_layers_response(response);
    layer_runtime.reset_from_registry(layer_registry);
    bootstrap.layers_loaded_map_version = layer_registry.map_version_id().map(ToString::to_string);
    bootstrap.layers_next_retry_at_utc = 0;
    bootstrap.layers_status = format!("layers: {layer_count} ({revision})");

    sync_zone_mask_controls(display_state, layer_registry, layer_runtime);
    bootstrap.map_version_dirty = true;
}

pub(super) fn sync_zone_mask_controls(
    display_state: &mut MapDisplayState,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) {
    if let Some(mask_layer_id) = layer_registry.first_id_by_pick_mode(PickMode::ExactTilePixel) {
        if let Some(state) = layer_runtime.get(mask_layer_id) {
            if display_state.show_zone_mask != state.visible {
                display_state.show_zone_mask = state.visible;
            }
            if (display_state.zone_mask_opacity - state.opacity).abs() > f32::EPSILON {
                display_state.zone_mask_opacity = state.opacity;
            }
        }
    }
}
