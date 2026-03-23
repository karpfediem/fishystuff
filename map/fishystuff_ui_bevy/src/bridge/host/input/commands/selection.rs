use crate::bridge::contract::FishyMapSelectionPointKind;
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::hover_query::WorldPointQueryContext;
use crate::map::layers::LayerRegistry;
use crate::map::layers::LayerRuntime;
use crate::map::raster::RasterTileCache;
use crate::map::selection_query::{
    selected_info_at_world_point, selected_info_for_semantic_field, selected_info_for_zone_rgb,
};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::WorldPoint;
use crate::plugins::api::{
    build_zone_stats_request, spawn_zone_stats_request, ApiBootstrapState, PatchFilterState,
    PendingRequests, SelectedInfo, SelectionState,
};
use crate::plugins::vector_layers::VectorLayerRuntime;
use fishystuff_core::field_metadata::FieldHoverTarget;

pub(super) fn apply_zone_selection_command(
    bootstrap: &ApiBootstrapState,
    patch_filter: &PatchFilterState,
    layer_registry: &LayerRegistry,
    field_metadata: &FieldMetadataCache,
    selection: &mut SelectionState,
    pending: &mut PendingRequests,
    zone_rgb: u32,
) {
    let selected_info = selected_info_for_zone_rgb(layer_registry, field_metadata, zone_rgb);
    apply_selected_info(
        bootstrap,
        patch_filter,
        selection,
        pending,
        Some(selected_info),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_semantic_field_selection_command(
    bootstrap: &ApiBootstrapState,
    patch_filter: &PatchFilterState,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
    exact_lookups: &ExactLookupCache,
    field_metadata: &FieldMetadataCache,
    tile_cache: &RasterTileCache,
    vector_runtime: &VectorLayerRuntime,
    selection: &mut SelectionState,
    pending: &mut PendingRequests,
    layer_key: &str,
    field_id: u32,
    target_key: Option<&str>,
) {
    let selected_info =
        selected_info_for_semantic_field(layer_registry, field_metadata, layer_key, field_id);
    let preferred_target = selected_info
        .as_ref()
        .and_then(|info| preferred_selection_target(info, target_key));
    let target_world_point =
        preferred_target.map(|target| WorldPoint::new(target.world_x, target.world_z));
    let selected_info = target_world_point
        .and_then(|world_point| {
            selected_info_at_world_point(
                world_point,
                &WorldPointQueryContext {
                    layer_registry,
                    layer_runtime,
                    exact_lookups,
                    field_metadata,
                    tile_cache,
                    vector_runtime,
                    map_to_world: MapToWorld::default(),
                },
                FishyMapSelectionPointKind::Waypoint,
                preferred_target.map(|target| target.label.as_str()),
            )
        })
        .or(selected_info);
    apply_selected_info(bootstrap, patch_filter, selection, pending, selected_info);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_world_point_selection_command(
    bootstrap: &ApiBootstrapState,
    patch_filter: &PatchFilterState,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
    exact_lookups: &ExactLookupCache,
    field_metadata: &FieldMetadataCache,
    tile_cache: &RasterTileCache,
    vector_runtime: &VectorLayerRuntime,
    selection: &mut SelectionState,
    pending: &mut PendingRequests,
    world_x: f64,
    world_z: f64,
    point_kind: Option<FishyMapSelectionPointKind>,
    point_label: Option<&str>,
) {
    let selected_info = selected_info_at_world_point(
        WorldPoint::new(world_x, world_z),
        &WorldPointQueryContext {
            layer_registry,
            layer_runtime,
            exact_lookups,
            field_metadata,
            tile_cache,
            vector_runtime,
            map_to_world: MapToWorld::default(),
        },
        point_kind.unwrap_or(FishyMapSelectionPointKind::Clicked),
        point_label,
    );
    apply_selected_info(bootstrap, patch_filter, selection, pending, selected_info);
}

fn preferred_selection_target<'a>(
    selected_info: &'a SelectedInfo,
    target_key: Option<&str>,
) -> Option<&'a FieldHoverTarget> {
    let layer_sample = selected_info.layer_samples.first()?;
    if let Some(target_key) = target_key {
        if let Some(target) = layer_sample
            .targets
            .iter()
            .find(|target| target.key == target_key)
        {
            return Some(target);
        }
    }
    layer_sample.targets.first()
}

fn apply_selected_info(
    bootstrap: &ApiBootstrapState,
    patch_filter: &PatchFilterState,
    selection: &mut SelectionState,
    pending: &mut PendingRequests,
    selected_info: Option<SelectedInfo>,
) {
    selection.info = selected_info.clone();
    selection.zone_stats = None;
    pending.zone_stats = None;
    let Some(selected_info) = selected_info else {
        selection.zone_stats_status = "zone stats: unavailable".to_string();
        return;
    };
    let Some(rgb) = selected_info.zone_rgb() else {
        selection.zone_stats_status = "zone stats: unavailable".to_string();
        return;
    };
    let Some(rgb_u32) = selected_info.zone_rgb_u32() else {
        selection.zone_stats_status = "zone stats: unavailable".to_string();
        return;
    };
    selection.zone_stats_status = "zone stats: loading".to_string();
    if let Some(request) = build_zone_stats_request(bootstrap, patch_filter, rgb) {
        pending.zone_stats = Some((rgb_u32, spawn_zone_stats_request(request)));
    } else {
        selection.zone_stats_status = "zone stats: missing defaults".to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::apply_selected_info;
    use crate::bridge::contract::FishyMapSelectionPointKind;
    use crate::plugins::api::{PendingRequests, SelectedInfo, SelectionState};

    #[test]
    fn apply_selected_info_marks_missing_selection_unavailable() {
        let bootstrap = crate::plugins::api::ApiBootstrapState::default();
        let patch_filter = crate::plugins::api::PatchFilterState::default();
        let mut selection = SelectionState::default();
        let mut pending = PendingRequests::default();

        apply_selected_info(
            &bootstrap,
            &patch_filter,
            &mut selection,
            &mut pending,
            None,
        );

        assert!(selection.info.is_none());
        assert!(pending.zone_stats.is_none());
        assert_eq!(selection.zone_stats_status, "zone stats: unavailable");
    }

    #[test]
    fn apply_selected_info_keeps_non_zone_selection_without_zone_stats() {
        let bootstrap = crate::plugins::api::ApiBootstrapState::default();
        let patch_filter = crate::plugins::api::PatchFilterState::default();
        let mut selection = SelectionState::default();
        let mut pending = PendingRequests::default();
        let info = SelectedInfo {
            map_px: 10,
            map_py: 20,
            world_x: 123.0,
            world_z: 456.0,
            sampled_world_point: true,
            point_kind: Some(FishyMapSelectionPointKind::Clicked),
            point_label: None,
            layer_samples: Vec::new(),
        };

        apply_selected_info(
            &bootstrap,
            &patch_filter,
            &mut selection,
            &mut pending,
            Some(info.clone()),
        );

        assert_eq!(selection.info, Some(info));
        assert!(pending.zone_stats.is_none());
        assert_eq!(selection.zone_stats_status, "zone stats: unavailable");
    }
}
