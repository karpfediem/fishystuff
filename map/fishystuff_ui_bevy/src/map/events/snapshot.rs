use async_channel::Receiver;
use bevy::prelude::Resource;
use bevy::tasks::IoTaskPool;
use fishystuff_api::models::events::{
    EventPointCompact, EventsSnapshotMetaResponse, EventsSnapshotResponse,
};
use fishystuff_client::{ClientError, FishyClient};

use super::index::{LocalEventQuery, SpatialIndex, ViewSelection, SPATIAL_BUCKET_PX};

pub const META_RECHECK_INTERVAL_SECS: f64 = 20.0;
pub const RETRY_BACKOFF_SECS: f64 = 2.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SnapshotLoadKind {
    #[default]
    None,
    NetworkInitial,
    NetworkRefetch,
    CacheReuse,
}

impl SnapshotLoadKind {
    pub fn label(self) -> &'static str {
        match self {
            SnapshotLoadKind::None => "none",
            SnapshotLoadKind::NetworkInitial => "network-initial",
            SnapshotLoadKind::NetworkRefetch => "network-refetch",
            SnapshotLoadKind::CacheReuse => "cache-reuse",
        }
    }
}

#[derive(Debug, Clone)]
pub enum SnapshotMetaAction {
    ReuseLoaded,
    FetchSnapshot { revision: String },
    IgnorePending,
}

#[derive(Resource)]
pub struct EventsSnapshotState {
    pub revision: Option<String>,
    pub loading: bool,
    pub loaded: bool,
    pub failed: bool,
    pub event_count: usize,
    pub events: Vec<EventPointCompact>,
    pub spatial_index: SpatialIndex,
    pub last_error: Option<String>,
    pub last_load_kind: SnapshotLoadKind,
    pub meta_requests_started: u64,
    pub snapshot_requests_started: u64,
    pub snapshot_rebuilds: u64,
    pub pending_snapshot_revision: Option<String>,
    pub snapshot_refresh_reason: String,
    pub last_meta_poll_at_secs: f64,
    pub next_retry_at_secs: f64,
    pub pending_meta: Option<Receiver<Result<EventsSnapshotMetaResponse, String>>>,
    pub pending_snapshot: Option<Receiver<Result<EventsSnapshotResponse, String>>>,
}

impl Default for EventsSnapshotState {
    fn default() -> Self {
        Self {
            revision: None,
            loading: false,
            loaded: false,
            failed: false,
            event_count: 0,
            events: Vec::new(),
            spatial_index: SpatialIndex::new(SPATIAL_BUCKET_PX),
            last_error: None,
            last_load_kind: SnapshotLoadKind::None,
            meta_requests_started: 0,
            snapshot_requests_started: 0,
            snapshot_rebuilds: 0,
            pending_snapshot_revision: None,
            snapshot_refresh_reason: "not-loaded".to_string(),
            last_meta_poll_at_secs: -1.0,
            next_retry_at_secs: 0.0,
            pending_meta: None,
            pending_snapshot: None,
        }
    }
}

impl EventsSnapshotState {
    pub fn should_poll_meta(&self, now_secs: f64) -> bool {
        if self.pending_meta.is_some() || self.pending_snapshot.is_some() {
            return false;
        }
        if now_secs < self.next_retry_at_secs {
            return false;
        }
        if !self.loaded {
            return true;
        }
        self.last_meta_poll_at_secs < 0.0
            || (now_secs - self.last_meta_poll_at_secs) >= META_RECHECK_INTERVAL_SECS
    }

    pub fn start_meta_poll(&mut self, now_secs: f64) {
        self.loading = true;
        self.last_meta_poll_at_secs = now_secs;
        self.meta_requests_started = self.meta_requests_started.saturating_add(1);
        self.pending_meta = Some(spawn_events_snapshot_meta_request());
    }

    pub fn apply_meta(&mut self, meta: &EventsSnapshotMetaResponse) -> SnapshotMetaAction {
        if self.pending_snapshot_revision.as_deref() == Some(meta.revision.as_str()) {
            return SnapshotMetaAction::IgnorePending;
        }
        if self.loaded && self.revision.as_deref() == Some(meta.revision.as_str()) {
            self.loading = false;
            self.failed = false;
            self.last_error = None;
            self.last_load_kind = SnapshotLoadKind::CacheReuse;
            self.snapshot_refresh_reason = "meta-unchanged".to_string();
            return SnapshotMetaAction::ReuseLoaded;
        }

        let reason = if self.loaded {
            "revision-changed"
        } else {
            "initial-load"
        };
        self.pending_snapshot_revision = Some(meta.revision.clone());
        self.snapshot_refresh_reason = reason.to_string();
        SnapshotMetaAction::FetchSnapshot {
            revision: meta.revision.clone(),
        }
    }

    pub fn start_snapshot_fetch(&mut self, revision: String) {
        self.loading = true;
        self.snapshot_requests_started = self.snapshot_requests_started.saturating_add(1);
        self.pending_snapshot = Some(spawn_events_snapshot_request(revision));
    }

    pub fn apply_snapshot(&mut self, response: EventsSnapshotResponse) {
        let last_revision = self.revision.clone();
        let next_count = response.event_count.max(response.events.len());
        self.revision = Some(response.revision);
        self.event_count = next_count;
        self.events = response.events;
        self.spatial_index.rebuild(&self.events);
        self.snapshot_rebuilds = self.snapshot_rebuilds.saturating_add(1);
        self.loading = false;
        self.loaded = true;
        self.failed = false;
        self.last_error = None;
        self.pending_snapshot_revision = None;
        self.next_retry_at_secs = 0.0;
        self.last_load_kind = if last_revision.is_some() {
            SnapshotLoadKind::NetworkRefetch
        } else {
            SnapshotLoadKind::NetworkInitial
        };
    }

    pub fn mark_failure(&mut self, now_secs: f64, error: String) {
        self.loading = false;
        self.failed = true;
        self.last_error = Some(error);
        self.next_retry_at_secs = now_secs + RETRY_BACKOFF_SECS;
        self.pending_snapshot_revision = None;
    }

    pub fn select_for_view(&self, query: &LocalEventQuery<'_>) -> ViewSelection {
        if !self.loaded {
            return ViewSelection::default();
        }
        let candidate_indices = {
            crate::perf_scope!("events.spatial_index_query");
            self.spatial_index.query_bbox(query.bbox, &self.events)
        };
        let candidate_count = candidate_indices.len();
        if candidate_indices.is_empty() {
            return ViewSelection {
                candidate_count: 0,
                filtered_indices: Vec::new(),
            };
        }

        let mut filtered_indices = {
            crate::perf_scope!("events.filter_application");
            let mut filtered_indices = Vec::with_capacity(candidate_indices.len());
            for idx in candidate_indices {
                let Some(event) = self.events.get(idx) else {
                    continue;
                };
                if query
                    .from_ts_utc
                    .is_some_and(|from_ts_utc| event.ts_utc < from_ts_utc)
                    || query
                        .to_ts_utc
                        .is_some_and(|to_ts_utc| event.ts_utc >= to_ts_utc)
                {
                    continue;
                }
                if !query.fish_ids.is_empty()
                    && query.fish_ids.binary_search(&event.fish_id).is_err()
                {
                    continue;
                }
                if let Some(zone_rgbs) = query.zone_rgbs.as_ref() {
                    let Some(zone_rgb) = event.zone_rgb_u32 else {
                        continue;
                    };
                    if !zone_rgbs.contains(&zone_rgb) {
                        continue;
                    }
                }
                if let Some(scope) = query.tile_scope.as_ref() {
                    if !scope.contains(event.map_px_x, event.map_px_y) {
                        continue;
                    }
                }
                filtered_indices.push(idx);
            }
            filtered_indices
        };
        crate::perf_gauge!("events.filtered_count", filtered_indices.len());

        ViewSelection {
            candidate_count,
            filtered_indices: std::mem::take(&mut filtered_indices),
        }
    }
}

fn spawn_events_snapshot_meta_request() -> Receiver<Result<EventsSnapshotMetaResponse, String>> {
    let (sender, receiver) = async_channel::bounded(1);
    IoTaskPool::get()
        .spawn_local(async move {
            let client = FishyClient::new("");
            let result = client.events_snapshot_meta().await;
            let result = result.map_err(client_error_to_string);
            let _ = sender.send(result).await;
        })
        .detach();
    receiver
}

fn spawn_events_snapshot_request(
    revision: String,
) -> Receiver<Result<EventsSnapshotResponse, String>> {
    let (sender, receiver) = async_channel::bounded(1);
    IoTaskPool::get()
        .spawn_local(async move {
            let client = FishyClient::new("");
            let result = client.events_snapshot(revision.as_str()).await;
            let result = result.map_err(client_error_to_string);
            let _ = sender.send(result).await;
        })
        .detach();
    receiver
}

fn client_error_to_string(error: ClientError) -> String {
    match error {
        ClientError::Transport(message) => message,
        ClientError::Decode(message) => message,
        ClientError::Api(error) => error.message,
        ClientError::HttpStatus(status, body) => format!("http {status}: {body}"),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::map::events::{VisibleTileScope, VISIBLE_TILE_SCOPE_PX};
    use fishystuff_api::models::events::MapBboxPx;

    fn sample_events() -> Vec<EventPointCompact> {
        vec![
            EventPointCompact {
                event_id: 1,
                fish_id: 101,
                ts_utc: 100,
                map_px_x: 100,
                map_px_y: 100,
                length_milli: 1000,
                world_x: Some(1000),
                world_z: Some(2000),
                zone_rgb_u32: Some(0x112233),
                source_kind: None,
                source_id: None,
            },
            EventPointCompact {
                event_id: 2,
                fish_id: 202,
                ts_utc: 200,
                map_px_x: 130,
                map_px_y: 110,
                length_milli: 1200,
                world_x: Some(1300),
                world_z: Some(2200),
                zone_rgb_u32: Some(0x445566),
                source_kind: None,
                source_id: None,
            },
            EventPointCompact {
                event_id: 3,
                fish_id: 101,
                ts_utc: 300,
                map_px_x: 3200,
                map_px_y: 4200,
                length_milli: 1500,
                world_x: None,
                world_z: None,
                zone_rgb_u32: Some(0x112233),
                source_kind: None,
                source_id: None,
            },
        ]
    }

    #[test]
    fn local_selection_applies_bbox_time_fish_and_tile_scope() {
        let mut state = EventsSnapshotState::default();
        state.loaded = true;
        state.events = sample_events();
        state.event_count = state.events.len();
        state.spatial_index.rebuild(&state.events);

        let bbox = MapBboxPx {
            min_x: 0,
            min_y: 0,
            max_x: 5000,
            max_y: 5000,
        };
        let query = LocalEventQuery {
            bbox: &bbox,
            from_ts_utc: Some(150),
            to_ts_utc: Some(350),
            fish_ids: &[101],
            zone_rgbs: None,
            tile_scope: Some(VisibleTileScope::from_bbox(
                &MapBboxPx {
                    min_x: 3000,
                    min_y: 4000,
                    max_x: 3500,
                    max_y: 4500,
                },
                VISIBLE_TILE_SCOPE_PX,
            )),
        };

        let selected = state.select_for_view(&query);
        assert_eq!(selected.filtered_indices, vec![2]);
    }

    #[test]
    fn local_selection_applies_zone_filter() {
        let mut state = EventsSnapshotState::default();
        state.loaded = true;
        state.events = sample_events();
        state.event_count = state.events.len();
        state.spatial_index.rebuild(&state.events);

        let bbox = MapBboxPx {
            min_x: 0,
            min_y: 0,
            max_x: 5000,
            max_y: 5000,
        };
        let zones = HashSet::from([0x445566]);
        let query = LocalEventQuery {
            bbox: &bbox,
            from_ts_utc: Some(0),
            to_ts_utc: Some(1000),
            fish_ids: &[],
            zone_rgbs: Some(&zones),
            tile_scope: None,
        };

        let selected = state.select_for_view(&query);
        assert_eq!(selected.filtered_indices, vec![1]);
    }

    #[test]
    fn revision_change_marks_one_pending_refetch_until_snapshot_applied() {
        let mut state = EventsSnapshotState::default();
        state.loaded = true;
        state.revision = Some("rev-1".to_string());
        state.snapshot_rebuilds = 1;

        let same_meta = EventsSnapshotMetaResponse {
            revision: "rev-1".to_string(),
            event_count: 10,
            source_kind: Default::default(),
            last_updated_utc: None,
            snapshot_url: "/api/v1/events_snapshot?revision=rev-1".to_string(),
        };
        assert!(matches!(
            state.apply_meta(&same_meta),
            SnapshotMetaAction::ReuseLoaded
        ));
        assert_eq!(state.snapshot_rebuilds, 1);

        let new_meta = EventsSnapshotMetaResponse {
            revision: "rev-2".to_string(),
            event_count: 11,
            source_kind: Default::default(),
            last_updated_utc: None,
            snapshot_url: "/api/v1/events_snapshot?revision=rev-2".to_string(),
        };
        assert!(matches!(
            state.apply_meta(&new_meta),
            SnapshotMetaAction::FetchSnapshot { .. }
        ));
        assert!(matches!(
            state.apply_meta(&new_meta),
            SnapshotMetaAction::IgnorePending
        ));
        assert_eq!(state.snapshot_rebuilds, 1);
    }

    #[test]
    fn pan_zoom_local_queries_do_not_trigger_new_network_fetches_once_loaded() {
        let mut state = EventsSnapshotState::default();
        state.loaded = true;
        state.revision = Some("rev-1".to_string());
        state.events = sample_events();
        state.event_count = state.events.len();
        state.spatial_index.rebuild(&state.events);
        state.meta_requests_started = 1;
        state.snapshot_requests_started = 1;

        let q1 = MapBboxPx {
            min_x: 0,
            min_y: 0,
            max_x: 512,
            max_y: 512,
        };
        let q2 = MapBboxPx {
            min_x: 2500,
            min_y: 3500,
            max_x: 5000,
            max_y: 5000,
        };
        let _ = state.select_for_view(&LocalEventQuery {
            bbox: &q1,
            from_ts_utc: Some(0),
            to_ts_utc: Some(10_000),
            fish_ids: &[],
            zone_rgbs: None,
            tile_scope: None,
        });
        let _ = state.select_for_view(&LocalEventQuery {
            bbox: &q2,
            from_ts_utc: Some(0),
            to_ts_utc: Some(10_000),
            fish_ids: &[],
            zone_rgbs: None,
            tile_scope: None,
        });

        assert_eq!(state.meta_requests_started, 1);
        assert_eq!(state.snapshot_requests_started, 1);
    }
}
