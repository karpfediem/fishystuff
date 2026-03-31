use bevy::image::Image;
use bevy::prelude::*;

use crate::map::events::{EventsSnapshotState, SnapshotMetaAction};

use super::render::{build_ring_texture, PointRingAssets};

pub(super) fn ensure_point_ring_assets(
    mut ring_assets: ResMut<PointRingAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    if ring_assets.texture.is_some() {
        return;
    }

    let texture = build_ring_texture();
    ring_assets.texture = Some(images.add(texture));
    ring_assets.diameter_map_px = super::render::RING_RADIUS_GAME_UNITS * 2.0;
}

pub(super) fn ensure_events_snapshot_loaded(
    time: Res<Time>,
    mut snapshot: ResMut<EventsSnapshotState>,
) {
    let now = time.elapsed_secs_f64();
    if snapshot.should_poll_meta(now) {
        snapshot.start_meta_poll(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_preload_no_longer_depends_on_layer_visibility() {
        let snapshot = EventsSnapshotState::default();
        assert!(snapshot.should_poll_meta(0.0));
    }
}

pub(super) fn poll_events_snapshot_requests(
    time: Res<Time>,
    mut snapshot: ResMut<EventsSnapshotState>,
) {
    let meta_result = snapshot
        .pending_meta
        .as_ref()
        .and_then(|receiver| receiver.try_recv().ok());
    if let Some(result) = meta_result {
        snapshot.pending_meta = None;
        match result {
            Ok(meta) => match snapshot.apply_meta(&meta) {
                SnapshotMetaAction::ReuseLoaded | SnapshotMetaAction::IgnorePending => {}
                SnapshotMetaAction::FetchSnapshot { revision } => {
                    snapshot.start_snapshot_fetch(revision);
                }
            },
            Err(err) => {
                snapshot.mark_failure(time.elapsed_secs_f64(), err);
            }
        }
    }

    let snapshot_result = snapshot
        .pending_snapshot
        .as_ref()
        .and_then(|receiver| receiver.try_recv().ok());
    if let Some(result) = snapshot_result {
        snapshot.pending_snapshot = None;
        match result {
            Ok(response) => snapshot.apply_snapshot(response),
            Err(err) => snapshot.mark_failure(time.elapsed_secs_f64(), err),
        }
    }
}
