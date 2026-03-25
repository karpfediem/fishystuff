use crate::map::layers::{
    build_local_layer_specs, AvailableLayerCatalog, LayerRegistry, LayerRuntime, PickMode,
};
use crate::plugins::api::{ApiBootstrapState, MapDisplayState};
use crate::prelude::*;

pub(super) fn sync_local_layers(
    available_layers: Res<AvailableLayerCatalog>,
    mut bootstrap: ResMut<ApiBootstrapState>,
    mut display_state: ResMut<MapDisplayState>,
    mut layer_registry: ResMut<LayerRegistry>,
    mut layer_runtime: ResMut<LayerRuntime>,
) {
    let Some(map_version) = bootstrap.map_version.clone() else {
        return;
    };
    if !available_layers.is_changed()
        && bootstrap.layers_loaded_map_version.as_deref() == Some::<&str>(map_version.as_str())
    {
        return;
    }

    let (revision, layers) =
        build_local_layer_specs(available_layers.entries(), Some(map_version.as_str()));
    let layer_count = layers.len();
    layer_registry.apply_layer_specs(revision.clone(), Some(map_version.clone()), layers);
    layer_runtime.sync_to_registry(&layer_registry);
    sync_zone_mask_controls(&mut display_state, &layer_registry, &layer_runtime);

    bootstrap.layers_loaded_map_version = Some(map_version);
    bootstrap.layers_status = format!("layers: local ({layer_count}, {revision})");
    bootstrap.map_version_dirty = true;
}

pub(crate) fn sync_zone_mask_controls(
    display_state: &mut MapDisplayState,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) {
    if let Some(mask_layer_id) = layer_registry.first_id_by_pick_mode(PickMode::ExactTilePixel) {
        if let Some(state) = layer_runtime.get(mask_layer_id) {
            if display_state.show_zone_mask != state.visible {
                display_state.show_zone_mask = state.visible;
            }
            if (display_state.zone_mask_opacity - state.opacity).abs() > f32::EPSILON {
                display_state.zone_mask_opacity = state.opacity;
            }
        }
    }
}
