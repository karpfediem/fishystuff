use crate::prelude::*;

use super::super::state::{ApiBootstrapState, FishCatalog, PendingRequests};
use super::spawn::{
    spawn_fish_catalog_request, spawn_layers_request, spawn_meta_request, spawn_zones_request,
};
use super::util::now_utc_seconds;

pub(super) fn ensure_meta_request(
    mut pending: ResMut<PendingRequests>,
    bootstrap: Res<ApiBootstrapState>,
) {
    if bootstrap.meta.is_some() || pending.meta.is_some() {
        return;
    }
    pending.meta = Some(spawn_meta_request());
}

pub(super) fn ensure_layers_request(
    mut pending: ResMut<PendingRequests>,
    mut bootstrap: ResMut<ApiBootstrapState>,
) {
    if pending.layers.is_some() {
        return;
    }
    let now = now_utc_seconds();
    if now < bootstrap.layers_next_retry_at_utc {
        return;
    }
    let Some(map_version) = bootstrap.map_version.clone() else {
        return;
    };
    if bootstrap.layers_loaded_map_version.as_deref() == Some(map_version.as_str()) {
        return;
    }
    bootstrap.layers_status = "layers: loading".to_string();
    pending.layers = Some(spawn_layers_request(Some(map_version)));
}

pub(super) fn ensure_zones_request(
    mut pending: ResMut<PendingRequests>,
    bootstrap: Res<ApiBootstrapState>,
) {
    if !bootstrap.zones.is_empty() || pending.zones.is_some() {
        return;
    }
    pending.zones = Some(spawn_zones_request());
}

pub(super) fn ensure_fish_catalog_request(
    mut pending: ResMut<PendingRequests>,
    fish: Res<FishCatalog>,
    bootstrap: Res<ApiBootstrapState>,
) {
    if bootstrap.meta.is_none() || !fish.entries.is_empty() || pending.fish_catalog.is_some() {
        return;
    }
    pending.fish_catalog = Some(spawn_fish_catalog_request());
}
