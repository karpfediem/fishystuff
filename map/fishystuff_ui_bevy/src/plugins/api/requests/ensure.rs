use crate::prelude::*;

use super::super::state::{
    ApiBootstrapState, CommunityFishZoneSupportIndex, FishCatalog, PendingRequests,
};
use super::spawn::{
    spawn_community_fish_zone_support_request, spawn_fish_catalog_request, spawn_meta_request,
    spawn_zones_request,
};

pub(super) fn ensure_meta_request(
    mut pending: ResMut<PendingRequests>,
    bootstrap: Res<ApiBootstrapState>,
    time: Res<Time>,
) {
    let now_secs = time.elapsed_secs_f64();
    if bootstrap.meta.is_some() || pending.meta.is_some() || !pending.can_request_meta(now_secs) {
        return;
    }
    pending.meta = Some(spawn_meta_request());
}

pub(super) fn ensure_zones_request(
    mut pending: ResMut<PendingRequests>,
    bootstrap: Res<ApiBootstrapState>,
    time: Res<Time>,
) {
    let now_secs = time.elapsed_secs_f64();
    if !bootstrap.zones.is_empty()
        || pending.zones.is_some()
        || !pending.can_request_zones(now_secs)
    {
        return;
    }
    pending.zones = Some(spawn_zones_request());
}

pub(super) fn ensure_fish_catalog_request(
    mut pending: ResMut<PendingRequests>,
    fish: Res<FishCatalog>,
    bootstrap: Res<ApiBootstrapState>,
    time: Res<Time>,
) {
    let now_secs = time.elapsed_secs_f64();
    if bootstrap.meta.is_none()
        || !fish.entries.is_empty()
        || pending.fish_catalog.is_some()
        || !pending.can_request_fish_catalog(now_secs)
    {
        return;
    }
    pending.fish_catalog = Some(spawn_fish_catalog_request());
}

pub(super) fn ensure_community_fish_zone_support_request(
    mut pending: ResMut<PendingRequests>,
    community: Res<CommunityFishZoneSupportIndex>,
    bootstrap: Res<ApiBootstrapState>,
    time: Res<Time>,
) {
    let now_secs = time.elapsed_secs_f64();
    if bootstrap.meta.is_none()
        || community.revision.is_some()
        || pending.community_fish_zone_support.is_some()
        || !pending.can_request_community_fish_zone_support(now_secs)
    {
        return;
    }
    pending.community_fish_zone_support = Some(spawn_community_fish_zone_support_request());
}
