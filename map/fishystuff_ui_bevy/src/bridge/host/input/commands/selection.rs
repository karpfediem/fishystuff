use super::super::super::persistence::rgb_u32_to_tuple;
use crate::plugins::api::{
    build_zone_stats_request, spawn_zone_stats_request, ApiBootstrapState, PatchFilterState,
    PendingRequests, SelectionState,
};

pub(super) fn apply_zone_selection_command(
    bootstrap: &ApiBootstrapState,
    patch_filter: &PatchFilterState,
    selection: &mut SelectionState,
    pending: &mut PendingRequests,
    zone_rgb: u32,
) {
    let rgb = rgb_u32_to_tuple(zone_rgb);
    selection.info = Some(crate::plugins::api::SelectedInfo {
        map_px: 0,
        map_py: 0,
        rgb,
        rgb_u32: zone_rgb,
        zone_name: bootstrap.zones.get(&zone_rgb).cloned().unwrap_or(None),
        world_x: 0.0,
        world_z: 0.0,
    });
    selection.zone_stats = None;
    selection.zone_stats_status = "zone stats: loading".to_string();
    if let Some(request) = build_zone_stats_request(bootstrap, patch_filter, rgb) {
        pending.zone_stats = Some((zone_rgb, spawn_zone_stats_request(request)));
    } else {
        selection.zone_stats_status = "zone stats: missing defaults".to_string();
    }
}
