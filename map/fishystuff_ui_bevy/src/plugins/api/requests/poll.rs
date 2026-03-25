use async_channel::TryRecvError;

use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::map::terrain::Terrain3dConfig;
use crate::prelude::*;
use bevy::ecs::system::SystemParam;

use super::super::fish::build_fish_catalog_entries;
use super::super::state::{
    ApiBootstrapState, FishCatalog, MapDisplayState, PatchFilterState, PendingRequests,
    SelectionState,
};
use super::apply::apply_meta_response;
use crate::plugins::local_layers::sync_display_layer_controls;

pub(super) fn poll_requests(mut state: RequestPollState<'_, '_>) {
    if let Some(receiver) = state.pending.meta.as_ref() {
        match receiver.try_recv() {
            Ok(result) => {
                state.pending.meta = None;
                match result {
                    Ok(meta) => apply_meta_response(
                        &mut state.bootstrap,
                        &mut state.patch_filter,
                        &mut state.terrain_config,
                        meta,
                    ),
                    Err(err) => {
                        state.bootstrap.meta_status = format!("meta: {err}");
                        state.bootstrap.layers_status = "layers: blocked".to_string();
                    }
                }
            }
            Err(TryRecvError::Closed) => {
                state.pending.meta = None;
                state.bootstrap.meta_status = "meta: request closed".to_string();
                state.bootstrap.layers_status = "layers: blocked".to_string();
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    if let Some(receiver) = state.pending.zones.as_ref() {
        match receiver.try_recv() {
            Ok(result) => {
                state.pending.zones = None;
                match result {
                    Ok(zones) => {
                        state.bootstrap.zones.clear();
                        for zone in zones.zones {
                            state.bootstrap.zones.insert(zone.rgb_u32, zone.name);
                        }
                        state.bootstrap.zones_status =
                            format!("zones: {}", state.bootstrap.zones.len());
                    }
                    Err(err) => {
                        state.bootstrap.zones_status = format!("zones: {err}");
                    }
                }
            }
            Err(TryRecvError::Closed) => {
                state.pending.zones = None;
                state.bootstrap.zones_status = "zones: request closed".to_string();
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    if let Some((rgb, receiver)) = state.pending.zone_stats.as_ref() {
        match receiver.try_recv() {
            Ok(result) => {
                let rgb = *rgb;
                state.pending.zone_stats = None;
                if state.selection.info.as_ref().and_then(|s| s.zone_rgb_u32()) != Some(rgb) {
                    return;
                }
                match result {
                    Ok(response) => {
                        state.selection.zone_stats = Some(response);
                        state.selection.zone_stats_status = "zone stats: loaded".to_string();
                    }
                    Err(err) => {
                        state.selection.zone_stats = None;
                        state.selection.zone_stats_status = format!("zone stats: {err}");
                    }
                }
            }
            Err(TryRecvError::Closed) => {
                state.pending.zone_stats = None;
                state.selection.zone_stats = None;
                state.selection.zone_stats_status = "zone stats: request closed".to_string();
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    if let Some(receiver) = state.pending.fish_catalog.as_ref() {
        match receiver.try_recv() {
            Ok(result) => {
                state.pending.fish_catalog = None;
                match result {
                    Ok(response) => {
                        let entries = build_fish_catalog_entries(response.fish);
                        state.fish.status = format!("fish: {}", entries.len());
                        state.fish.replace(entries);
                    }
                    Err(err) => {
                        state.fish.status = format!("fish: {err}");
                    }
                }
            }
            Err(TryRecvError::Closed) => {
                state.pending.fish_catalog = None;
                state.fish.status = "fish: request closed".to_string();
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    sync_display_layer_controls(
        &mut state.display_state,
        &state.layer_registry,
        &state.layer_runtime,
    );
}

#[derive(SystemParam)]
pub(crate) struct RequestPollState<'w, 's> {
    bootstrap: ResMut<'w, ApiBootstrapState>,
    patch_filter: ResMut<'w, PatchFilterState>,
    display_state: ResMut<'w, MapDisplayState>,
    pending: ResMut<'w, PendingRequests>,
    layer_registry: ResMut<'w, LayerRegistry>,
    layer_runtime: ResMut<'w, LayerRuntime>,
    terrain_config: ResMut<'w, Terrain3dConfig>,
    selection: ResMut<'w, SelectionState>,
    fish: ResMut<'w, FishCatalog>,
    _marker: std::marker::PhantomData<&'s ()>,
}
