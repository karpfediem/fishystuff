mod apply;
mod ensure;
mod poll;
mod spawn;
mod util;

use async_channel::Receiver;
use fishystuff_api::models::meta::MetaResponse;
use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};
use fishystuff_api::Rgb;

use crate::prelude::*;

use super::state::{ApiBootstrapState, FishCatalog, PatchFilterState, PendingRequests};

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

pub(super) fn poll_requests(state: poll::RequestPollState<'_, '_>) {
    poll::poll_requests(state);
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
    rgb: Rgb,
) -> Option<ZoneStatsRequest> {
    util::build_zone_stats_request(bootstrap, patch_filter, rgb)
}
