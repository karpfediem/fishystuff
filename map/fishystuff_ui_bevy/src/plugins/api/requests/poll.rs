use async_channel::TryRecvError;
use std::time::Duration;

use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::prelude::*;
use bevy::ecs::system::SystemParam;

use super::super::fish::build_fish_catalog_entries;
use super::super::state::{
    ApiBootstrapState, CommunityFishZoneSupportIndex, FishCatalog, MapDisplayState,
    PatchFilterState, PendingRequests, SelectionState,
};
use super::apply::apply_meta_response;
use crate::plugins::local_layers::sync_display_layer_controls;

pub(super) fn poll_requests(mut state: RequestPollState<'_, '_>) {
    let now_secs = state.time.elapsed_secs_f64();

    if let Some(receiver) = state.pending.meta.as_ref() {
        match receiver.try_recv() {
            Ok(result) => {
                state.pending.meta = None;
                match result {
                    Ok(meta) => {
                        state.pending.record_meta_success();
                        apply_meta_response(&mut state.bootstrap, &mut state.patch_filter, meta)
                    }
                    Err(err) => {
                        let delay = state.pending.record_meta_failure(now_secs);
                        state.bootstrap.meta_status = api_retry_status("meta", &err, delay);
                        state.bootstrap.layers_status = "layers: waiting for API".to_string();
                    }
                }
            }
            Err(TryRecvError::Closed) => {
                state.pending.meta = None;
                let delay = state.pending.record_meta_failure(now_secs);
                state.bootstrap.meta_status = api_retry_status("meta", "request closed", delay);
                state.bootstrap.layers_status = "layers: waiting for API".to_string();
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
                        state.pending.record_zones_success();
                        state.bootstrap.zones.clear();
                        for zone in zones.zones {
                            state.bootstrap.zones.insert(zone.rgb_u32, zone.name);
                        }
                        state.bootstrap.zones_status =
                            format!("zones: {}", state.bootstrap.zones.len());
                    }
                    Err(err) => {
                        let delay = state.pending.record_zones_failure(now_secs);
                        state.bootstrap.zones_status = api_retry_status("zones", &err, delay);
                    }
                }
            }
            Err(TryRecvError::Closed) => {
                state.pending.zones = None;
                let delay = state.pending.record_zones_failure(now_secs);
                state.bootstrap.zones_status = api_retry_status("zones", "request closed", delay);
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
                        state.pending.record_fish_catalog_success();
                        let entries = build_fish_catalog_entries(response.fish);
                        state.fish.status = format!("fish: {}", entries.len());
                        state.fish.replace(entries);
                    }
                    Err(err) => {
                        let delay = state.pending.record_fish_catalog_failure(now_secs);
                        state.fish.status = api_retry_status("fish", &err, delay);
                    }
                }
            }
            Err(TryRecvError::Closed) => {
                state.pending.fish_catalog = None;
                let delay = state.pending.record_fish_catalog_failure(now_secs);
                state.fish.status = api_retry_status("fish", "request closed", delay);
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    if let Some(receiver) = state.pending.community_fish_zone_support.as_ref() {
        match receiver.try_recv() {
            Ok(result) => {
                state.pending.community_fish_zone_support = None;
                match result {
                    Ok(response) => {
                        state.pending.record_community_fish_zone_support_success();
                        state.community.replace_from_response(response);
                    }
                    Err(err) => {
                        let delay = state
                            .pending
                            .record_community_fish_zone_support_failure(now_secs);
                        state.community.status =
                            api_retry_status("fish community support", &err, delay);
                    }
                }
            }
            Err(TryRecvError::Closed) => {
                state.pending.community_fish_zone_support = None;
                let delay = state
                    .pending
                    .record_community_fish_zone_support_failure(now_secs);
                state.community.status =
                    api_retry_status("fish community support", "request closed", delay);
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

fn api_retry_status(kind: &str, error: &str, delay: Duration) -> String {
    let error = compact_api_error(error);
    format!(
        "{kind}: request failed; retrying in {} ({error})",
        format_retry_delay(delay)
    )
}

fn format_retry_delay(delay: Duration) -> String {
    let seconds = delay.as_secs().max(1);
    if seconds >= 60 {
        let minutes = seconds.div_ceil(60);
        return format!("{minutes}m");
    }
    format!("{seconds}s")
}

fn compact_api_error(error: &str) -> String {
    const MAX_ERROR_CHARS: usize = 120;
    let single_line = error.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.is_empty() {
        return "unknown error".to_string();
    }
    if single_line.chars().count() <= MAX_ERROR_CHARS {
        return single_line;
    }
    let mut compact = single_line
        .chars()
        .take(MAX_ERROR_CHARS.saturating_sub(3))
        .collect::<String>();
    compact.push_str("...");
    compact
}

#[derive(SystemParam)]
pub(crate) struct RequestPollState<'w, 's> {
    bootstrap: ResMut<'w, ApiBootstrapState>,
    patch_filter: ResMut<'w, PatchFilterState>,
    display_state: ResMut<'w, MapDisplayState>,
    pending: ResMut<'w, PendingRequests>,
    layer_registry: Res<'w, LayerRegistry>,
    layer_runtime: Res<'w, LayerRuntime>,
    selection: ResMut<'w, SelectionState>,
    fish: ResMut<'w, FishCatalog>,
    community: ResMut<'w, CommunityFishZoneSupportIndex>,
    time: Res<'w, Time>,
    _marker: std::marker::PhantomData<&'s ()>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_retry_status_includes_kind_delay_and_compact_error() {
        assert_eq!(
            api_retry_status("meta", "Failed to fetch", Duration::from_secs(2)),
            "meta: request failed; retrying in 2s (Failed to fetch)"
        );
    }

    #[test]
    fn compact_api_error_truncates_long_messages() {
        let message = "x".repeat(160);
        let compact = compact_api_error(&message);

        assert_eq!(compact.chars().count(), 120);
        assert!(compact.ends_with("..."));
    }
}
