mod hit;
mod loading;
mod query;
mod render;

use bevy::prelude::*;

use crate::bridge::BrowserInputStateSet;
use crate::map::events::EventsSnapshotState;
use crate::plugins::api::LayerEffectiveFilterState;

pub use hit::{point_hover_samples_at_world_point, point_samples_at_world_point};
pub use query::{PointsState, RenderPoint};
#[cfg(target_arch = "wasm32")]
pub(crate) use render::PointIconCache;
pub use render::{EventPointIconMarker, EventPointRingMarker};

pub struct PointsPlugin;

impl Plugin for PointsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PointsState>()
            .init_resource::<query::PointRenderState>()
            .init_resource::<EventsSnapshotState>()
            .init_resource::<LayerEffectiveFilterState>()
            .init_resource::<render::PointRingAssets>()
            .init_resource::<render::PointMarkerPool>()
            .init_resource::<render::PointIconCache>()
            .add_systems(
                PreUpdate,
                query::sync_layer_effective_filters.after(BrowserInputStateSet),
            )
            .add_systems(
                Update,
                (
                    loading::ensure_point_ring_assets,
                    loading::ensure_events_snapshot_loaded,
                    loading::poll_events_snapshot_requests,
                    query::refresh_points_from_local_snapshot,
                    render::mark_points_dirty_on_remote_image_update,
                    render::sync_point_markers,
                    render::request_redraw_for_point_updates,
                )
                    .chain(),
            );
    }
}
