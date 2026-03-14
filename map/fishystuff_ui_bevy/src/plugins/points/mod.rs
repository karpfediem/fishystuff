mod loading;
mod query;
mod render;

use bevy::prelude::*;

use crate::map::events::EventsSnapshotState;

pub use query::{EvidenceZoneFilter, PointsState, RenderPoint};
pub(crate) use render::PointIconCache;
pub use render::{EventPointIconMarker, EventPointRingMarker};

pub struct PointsPlugin;

impl Plugin for PointsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PointsState>()
            .init_resource::<EventsSnapshotState>()
            .init_resource::<EvidenceZoneFilter>()
            .init_resource::<render::PointRingAssets>()
            .init_resource::<render::PointMarkerPool>()
            .init_resource::<render::PointIconCache>()
            .add_systems(
                Update,
                (
                    loading::ensure_point_ring_assets,
                    loading::ensure_events_snapshot_loaded,
                    loading::poll_events_snapshot_requests,
                    query::refresh_points_from_local_snapshot,
                    query::sync_evidence_zone_filter,
                    render::sync_point_markers,
                    render::track_point_icon_load_states,
                )
                    .chain(),
            );
    }
}
