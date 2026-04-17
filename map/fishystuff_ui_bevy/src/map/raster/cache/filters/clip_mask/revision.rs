use crate::map::layers::{
    LayerId, LayerManifestStatus, LayerRegistry, LayerRuntime, LayerVectorStatus, PickMode,
};
use crate::plugins::api::ZoneMembershipFilter;

pub(crate) fn clip_mask_state_revision(
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
    clip_mask_layer: Option<LayerId>,
    filter: &ZoneMembershipFilter,
) -> u64 {
    let Some(mask_layer_id) = clip_mask_layer else {
        return 0;
    };
    let Some(mask_layer) = layer_registry.get(mask_layer_id) else {
        return 0;
    };
    let Some(mask_state) = layer_runtime.get(mask_layer_id) else {
        return 0;
    };
    let mut revision = u64::from(mask_layer_id.as_u16());
    revision = revision
        .wrapping_mul(131)
        .wrapping_add(u64::from(mask_state.resident_tile_count));
    revision = revision
        .wrapping_mul(131)
        .wrapping_add(u64::from(mask_state.pending_count));
    revision = revision
        .wrapping_mul(131)
        .wrapping_add(u64::from(mask_state.inflight_count));
    revision = revision
        .wrapping_mul(131)
        .wrapping_add(u64::from(mask_state.vector_cache_entries));
    revision = revision
        .wrapping_mul(131)
        .wrapping_add(layer_manifest_status_code(mask_state.manifest_status));
    revision = revision
        .wrapping_mul(131)
        .wrapping_add(layer_vector_status_code(mask_state.vector_status));
    if mask_layer.pick_mode == PickMode::ExactTilePixel
        && mask_layer.key == "zone_mask"
        && filter.active
    {
        revision = revision.wrapping_mul(131).wrapping_add(filter.revision);
    }
    revision
}

fn layer_manifest_status_code(status: LayerManifestStatus) -> u64 {
    match status {
        LayerManifestStatus::Missing => 0,
        LayerManifestStatus::Loading => 1,
        LayerManifestStatus::Ready => 2,
        LayerManifestStatus::Failed => 3,
    }
}

fn layer_vector_status_code(status: LayerVectorStatus) -> u64 {
    match status {
        LayerVectorStatus::Inactive => 0,
        LayerVectorStatus::NotRequested => 1,
        LayerVectorStatus::Fetching => 2,
        LayerVectorStatus::Parsing => 3,
        LayerVectorStatus::Building => 4,
        LayerVectorStatus::Ready => 5,
        LayerVectorStatus::Failed => 6,
    }
}
