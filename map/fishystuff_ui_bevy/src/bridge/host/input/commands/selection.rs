use fishystuff_api::Rgb;
use fishystuff_core::field_metadata::FIELD_HOVER_ROW_KEY_ZONE;

use crate::map::field_metadata::FieldMetadataCache;
use crate::map::layers::LayerRegistry;
use crate::plugins::api::{
    build_zone_stats_request, spawn_zone_stats_request, ApiBootstrapState, PatchFilterState,
    PendingRequests, SelectionState,
};

pub(super) fn apply_zone_selection_command(
    bootstrap: &ApiBootstrapState,
    patch_filter: &PatchFilterState,
    layer_registry: &LayerRegistry,
    field_metadata: &FieldMetadataCache,
    selection: &mut SelectionState,
    pending: &mut PendingRequests,
    zone_rgb: u32,
) {
    let rgb = Rgb::from_u32(zone_rgb);
    selection.info = Some(crate::plugins::api::SelectedInfo {
        map_px: 0,
        map_py: 0,
        rgb,
        rgb_u32: zone_rgb,
        zone_name: resolve_zone_name(layer_registry, field_metadata, zone_rgb),
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

fn resolve_zone_name(
    layer_registry: &LayerRegistry,
    field_metadata: &FieldMetadataCache,
    zone_rgb: u32,
) -> Option<String> {
    let layer = layer_registry.get_by_key("zone_mask")?;
    let metadata_url = layer.field_metadata_url()?;
    let entry = field_metadata.entry(layer.id, &metadata_url, zone_rgb)?;
    let value = entry.row_value(FIELD_HOVER_ROW_KEY_ZONE)?.trim();
    (!value.is_empty()).then(|| value.to_string())
}
