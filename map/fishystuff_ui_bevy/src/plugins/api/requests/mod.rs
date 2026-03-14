mod apply;
mod ensure;
mod poll;
mod spawn;
mod util;

use async_channel::Receiver;
use fishystuff_api::models::meta::MetaResponse;
use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};

use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::map::terrain::Terrain3dConfig;
use crate::prelude::*;

use super::state::{
    ApiBootstrapState, FishCatalog, MapDisplayState, PatchFilterState, PendingRequests,
    SelectionState,
};

pub(super) fn ensure_meta_request(
    pending: ResMut<PendingRequests>,
    bootstrap: Res<ApiBootstrapState>,
) {
    ensure::ensure_meta_request(pending, bootstrap);
}

pub(super) fn ensure_layers_request(
    pending: ResMut<PendingRequests>,
    bootstrap: ResMut<ApiBootstrapState>,
) {
    ensure::ensure_layers_request(pending, bootstrap);
}

pub(super) fn ensure_zones_request(
    pending: ResMut<PendingRequests>,
    bootstrap: Res<ApiBootstrapState>,
) {
    ensure::ensure_zones_request(pending, bootstrap);
}

pub(super) fn ensure_fish_catalog_request(
    pending: ResMut<PendingRequests>,
    fish: Res<FishCatalog>,
    bootstrap: Res<ApiBootstrapState>,
) {
    ensure::ensure_fish_catalog_request(pending, fish, bootstrap);
}

pub(super) fn poll_requests(
    bootstrap: ResMut<ApiBootstrapState>,
    patch_filter: ResMut<PatchFilterState>,
    display_state: ResMut<MapDisplayState>,
    pending: ResMut<PendingRequests>,
    layer_registry: ResMut<LayerRegistry>,
    layer_runtime: ResMut<LayerRuntime>,
    terrain_config: ResMut<Terrain3dConfig>,
    selection: ResMut<SelectionState>,
    fish: ResMut<FishCatalog>,
) {
    poll::poll_requests(
        bootstrap,
        patch_filter,
        display_state,
        pending,
        layer_registry,
        layer_runtime,
        terrain_config,
        selection,
        fish,
    );
}

pub fn spawn_zone_stats_request(
    request: ZoneStatsRequest,
) -> Receiver<Result<ZoneStatsResponse, String>> {
    spawn::spawn_zone_stats_request(request)
}

pub fn pick_map_version(meta: &MetaResponse) -> Option<String> {
    util::pick_map_version(meta)
}

pub fn default_from_ts(meta: &MetaResponse) -> i64 {
    util::default_from_ts(meta)
}

pub fn default_from_patch_id(meta: &MetaResponse) -> Option<String> {
    util::default_from_patch_id(meta)
}

pub fn now_utc_seconds() -> i64 {
    util::now_utc_seconds()
}

pub fn build_zone_stats_request(
    bootstrap: &ApiBootstrapState,
    patch_filter: &PatchFilterState,
    rgb: (u8, u8, u8),
) -> Option<ZoneStatsRequest> {
    util::build_zone_stats_request(bootstrap, patch_filter, rgb)
}
