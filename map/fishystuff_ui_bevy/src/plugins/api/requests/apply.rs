use fishystuff_api::models::meta::MetaResponse;

use crate::map::terrain::Terrain3dConfig;

use super::super::state::{ApiBootstrapState, PatchFilterState};
use super::util::{default_from_patch_id, default_from_ts, now_utc_seconds, pick_map_version};

pub(super) fn apply_meta_response(
    bootstrap: &mut ApiBootstrapState,
    patch_filter: &mut PatchFilterState,
    terrain_config: &mut Terrain3dConfig,
    meta: MetaResponse,
) {
    terrain_config.map_width = meta.canonical_map.image_size_x;
    terrain_config.map_height = meta.canonical_map.image_size_y;

    let map_version = pick_map_version(&meta);
    if map_version != bootstrap.map_version {
        bootstrap.map_version_dirty = true;
        bootstrap.layers_loaded_map_version = None;
    }
    bootstrap.meta_status = "meta: loaded".to_string();
    bootstrap.defaults = Some(meta.defaults.clone());
    bootstrap.map_version = map_version;
    patch_filter.from_ts = Some(default_from_ts(&meta));
    patch_filter.to_ts = Some(now_utc_seconds());
    patch_filter.patches = meta.patches.clone();
    bootstrap.meta = Some(meta);
    if patch_filter.selected_patch.is_none() {
        patch_filter.selected_patch = bootstrap.meta.as_ref().and_then(default_from_patch_id);
    }
}
