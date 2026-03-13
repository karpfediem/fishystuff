use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::map::terrain::Terrain3dConfig;
use crate::prelude::*;

use super::super::fish::build_fish_catalog_entries;
use super::super::state::{
    ApiBootstrapState, FishCatalog, MapDisplayState, PatchFilterState, PendingRequests,
    SelectionState,
};
use super::apply::{apply_layers_response, apply_meta_response, sync_zone_mask_controls};
use super::util::now_utc_seconds;

pub(super) fn poll_requests(
    mut bootstrap: ResMut<ApiBootstrapState>,
    mut patch_filter: ResMut<PatchFilterState>,
    mut display_state: ResMut<MapDisplayState>,
    mut pending: ResMut<PendingRequests>,
    mut layer_registry: ResMut<LayerRegistry>,
    mut layer_runtime: ResMut<LayerRuntime>,
    mut terrain_config: ResMut<Terrain3dConfig>,
    mut selection: ResMut<SelectionState>,
    mut fish: ResMut<FishCatalog>,
) {
    if let Some(receiver) = pending.meta.as_ref() {
        if let Ok(result) = receiver.try_recv() {
            pending.meta = None;
            match result {
                Ok(meta) => apply_meta_response(
                    &mut bootstrap,
                    &mut patch_filter,
                    &mut terrain_config,
                    meta,
                ),
                Err(err) => {
                    bootstrap.meta_status = format!("meta: {err}");
                    bootstrap.layers_status = "layers: blocked".to_string();
                }
            }
        }
    }

    if let Some(receiver) = pending.layers.as_ref() {
        if let Ok(result) = receiver.try_recv() {
            pending.layers = None;
            match result {
                Ok(response) => apply_layers_response(
                    &mut bootstrap,
                    &mut display_state,
                    &mut layer_registry,
                    &mut layer_runtime,
                    response,
                ),
                Err(err) => {
                    bootstrap.layers_status = format!("layers: {err}");
                    bootstrap.layers_next_retry_at_utc = now_utc_seconds() + 2;
                }
            }
        }
    }

    if let Some(receiver) = pending.zones.as_ref() {
        if let Ok(result) = receiver.try_recv() {
            pending.zones = None;
            match result {
                Ok(zones) => {
                    bootstrap.zones.clear();
                    for zone in zones.zones {
                        bootstrap.zones.insert(zone.rgb_u32, zone.name);
                    }
                    bootstrap.zones_status = format!("zones: {}", bootstrap.zones.len());
                }
                Err(err) => {
                    bootstrap.zones_status = format!("zones: {err}");
                }
            }
        }
    }

    if let Some((rgb, receiver)) = pending.zone_stats.as_ref() {
        if let Ok(result) = receiver.try_recv() {
            let rgb = *rgb;
            pending.zone_stats = None;
            if selection.info.as_ref().map(|s| s.rgb_u32) != Some(rgb) {
                return;
            }
            match result {
                Ok(response) => {
                    selection.zone_stats = Some(response);
                    selection.zone_stats_status = "zone stats: loaded".to_string();
                }
                Err(err) => {
                    selection.zone_stats = None;
                    selection.zone_stats_status = format!("zone stats: {err}");
                }
            }
        }
    }

    if let Some(receiver) = pending.fish_catalog.as_ref() {
        if let Ok(result) = receiver.try_recv() {
            pending.fish_catalog = None;
            match result {
                Ok(response) => {
                    let (entries, icon_by_id) =
                        build_fish_catalog_entries(response.fish, response.fish_table);
                    fish.entries = entries;
                    fish.icon_by_id = icon_by_id;
                    fish.status = format!("fish: {}", fish.entries.len());
                }
                Err(err) => {
                    fish.status = format!("fish: {err}");
                }
            }
        }
    }

    sync_zone_mask_controls(&mut display_state, &layer_registry, &layer_runtime);
}
