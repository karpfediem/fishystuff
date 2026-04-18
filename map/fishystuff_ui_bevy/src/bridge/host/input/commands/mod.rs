mod selection;
mod view;

use crate::bridge::host::BrowserBridgeState;
use crate::map::camera::map2d::Map2dViewState;
use crate::map::camera::mode::ViewModeState;
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::layers::LayerRegistry;
use crate::map::layers::LayerRuntime;
use crate::map::raster::RasterTileCache;
use crate::plugins::api::{
    ApiBootstrapState, LayerEffectiveFilterState, PatchFilterState, PendingRequests, SelectionState,
};
use crate::plugins::camera::CameraZoomBounds;
use crate::plugins::vector_layers::VectorLayerRuntime;
use crate::prelude::*;

pub(in crate::bridge::host) fn apply_browser_commands(
    mut bridge: ResMut<BrowserBridgeState>,
    zoom_bounds: Res<CameraZoomBounds>,
    bootstrap: Res<ApiBootstrapState>,
    patch_filter: Res<PatchFilterState>,
    layer_registry: Res<LayerRegistry>,
    layer_runtime: Res<LayerRuntime>,
    exact_lookups: Res<ExactLookupCache>,
    field_metadata: Res<FieldMetadataCache>,
    tile_cache: Res<RasterTileCache>,
    vector_runtime: Res<VectorLayerRuntime>,
    layer_filters: Res<LayerEffectiveFilterState>,
    mut selection: ResMut<SelectionState>,
    mut pending: ResMut<PendingRequests>,
    mut view_mode: ResMut<ViewModeState>,
    mut map_view: ResMut<Map2dViewState>,
) {
    crate::perf_scope!("bridge.command_apply");
    let mut commands = Vec::new();
    std::mem::swap(&mut bridge.pending_commands, &mut commands);
    crate::perf_gauge!("bridge.pending_commands", commands.len());
    crate::perf_counter_add!("bridge.commands.applied", commands.len());

    for command in commands {
        view::apply_view_commands(&command, &zoom_bounds, &mut view_mode, &mut map_view);

        if let Some(zone_rgb) = command.select_zone_rgb {
            selection::apply_zone_selection_command(
                &bootstrap,
                &patch_filter,
                &layer_registry,
                &field_metadata,
                &mut selection,
                &mut pending,
                zone_rgb,
            );
        }
        if let Some(select_semantic_field) = command.select_semantic_field.as_ref() {
            selection::apply_semantic_field_selection_command(
                &bootstrap,
                &patch_filter,
                &layer_registry,
                &layer_runtime,
                &exact_lookups,
                &field_metadata,
                &tile_cache,
                &vector_runtime,
                &layer_filters,
                &mut selection,
                &mut pending,
                &select_semantic_field.layer_id,
                select_semantic_field.field_id,
                select_semantic_field.target_key.as_deref(),
            );
        }
        if let Some(world_point) = command.select_world_point {
            selection::apply_world_point_selection_command(
                &bootstrap,
                &patch_filter,
                &layer_registry,
                &layer_runtime,
                &exact_lookups,
                &field_metadata,
                &tile_cache,
                &vector_runtime,
                &layer_filters,
                &mut selection,
                &mut pending,
                world_point.world_x,
                world_point.world_z,
                world_point.point_kind,
                world_point.point_label.as_deref(),
            );
        }
    }
}
