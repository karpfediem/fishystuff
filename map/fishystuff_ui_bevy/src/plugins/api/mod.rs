mod fish;
mod remote_images;
mod requests;
mod state;

use crate::map::layers::AvailableLayerCatalog;
use crate::plugins::local_layers;
use crate::prelude::*;

pub(crate) use fish::fish_item_icon_url;
pub(crate) use remote_images::{
    remote_image_handle, RemoteImageCache, RemoteImageEpoch, RemoteImageStatus,
};
#[cfg(target_arch = "wasm32")]
pub(crate) use requests::resolve_api_request_url;
pub use requests::{
    build_zone_stats_request, default_from_patch_id, default_from_ts, now_utc_seconds,
    pick_map_version, spawn_zone_stats_request,
};
pub use state::{
    ApiBootstrapState, CommunityFishZoneSupportIndex, FishCatalog, FishEntry, FishFilterState,
    HoverInfo, HoverState, LayerEffectiveFilterState, LayerFilterBindingOverrideState,
    MapDisplayState, Patch, PatchFilterState, PendingRequests, SearchExpressionState, SelectedInfo,
    SelectionState, SemanticFieldFilterState, ZoneMembershipFilter, POINT_ICON_SCALE_MAX,
    POINT_ICON_SCALE_MIN,
};

pub struct ApiPlugin;

impl Plugin for ApiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ApiBootstrapState>()
            .init_resource::<PatchFilterState>()
            .init_resource::<FishFilterState>()
            .init_resource::<SemanticFieldFilterState>()
            .init_resource::<SearchExpressionState>()
            .init_resource::<LayerFilterBindingOverrideState>()
            .init_resource::<MapDisplayState>()
            .init_resource::<AvailableLayerCatalog>()
            .init_resource::<PendingRequests>()
            .init_resource::<HoverState>()
            .init_resource::<SelectionState>()
            .init_resource::<FishCatalog>()
            .init_resource::<CommunityFishZoneSupportIndex>()
            .init_resource::<RemoteImageCache>()
            .init_resource::<RemoteImageEpoch>()
            .add_systems(
                Update,
                (
                    requests::ensure_meta_request,
                    requests::ensure_zones_request,
                    requests::ensure_fish_catalog_request,
                    requests::ensure_community_fish_zone_support_request,
                    requests::poll_requests,
                    local_layers::sync_local_layers,
                    remote_images::poll_remote_image_requests,
                )
                    .chain(),
            );
    }
}
