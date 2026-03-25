use crate::prelude::*;

use super::super::state::{ApiBootstrapState, FishCatalog, PendingRequests};
use super::spawn::{spawn_fish_catalog_request, spawn_meta_request, spawn_zones_request};

pub(super) fn ensure_meta_request(
    mut pending: ResMut<PendingRequests>,
    bootstrap: Res<ApiBootstrapState>,
) {
    if bootstrap.meta.is_some() || pending.meta.is_some() {
        return;
    }
    pending.meta = Some(spawn_meta_request());
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
