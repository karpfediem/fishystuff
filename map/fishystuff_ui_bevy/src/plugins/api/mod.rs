mod fish;
mod requests;
mod state;

use crate::prelude::*;

pub(crate) use fish::bevy_public_asset_path;
pub use requests::{
    build_zone_stats_request, default_from_patch_id, default_from_ts, now_utc_seconds,
    pick_map_version, spawn_zone_stats_request,
};
pub(crate) use requests::{normalize_public_base_url, resolve_public_asset_url};
pub use state::{
    ApiBootstrapState, FishCatalog, FishEntry, FishFilterState, HoverInfo, HoverLayerSample,
    HoverState, MapDisplayState, Patch, PatchFilterState, PendingRequests, SelectedInfo,
    SelectionState, POINT_ICON_SCALE_MAX, POINT_ICON_SCALE_MIN,
};

pub struct ApiPlugin;

impl Plugin for ApiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ApiBootstrapState>()
            .init_resource::<PatchFilterState>()
            .init_resource::<FishFilterState>()
            .init_resource::<MapDisplayState>()
            .init_resource::<PendingRequests>()
            .init_resource::<HoverState>()
            .init_resource::<SelectionState>()
            .init_resource::<FishCatalog>()
            .add_systems(
                Update,
                (
                    requests::ensure_meta_request,
                    requests::ensure_layers_request,
                    requests::ensure_zones_request,
                    requests::ensure_fish_catalog_request,
                    requests::poll_requests,
                )
                    .chain(),
            );
    }
}
