mod filters;
mod state;
mod view;

use self::filters::{bridge_capabilities, current_layer_summaries};
use self::state::{effective_hover_snapshot, effective_selection_snapshot, effective_ui_state};
use super::*;

pub(super) use self::filters::{current_layer_order, effective_filters};
pub(super) use self::state::hover_layer_samples_snapshot;
pub(super) use self::view::effective_view_snapshot;

pub(super) fn initial_snapshot() -> FishyMapStateSnapshot {
    let mut snapshot = FishyMapStateSnapshot::default();
    snapshot.catalog.capabilities = bridge_capabilities();
    snapshot
}

pub(super) fn sync_current_snapshot(
    bridge: Res<BrowserBridgeState>,
    bootstrap: Res<ApiBootstrapState>,
    patch_filter: Res<PatchFilterState>,
    fish_filter: Res<FishFilterState>,
    display_state: Res<MapDisplayState>,
    fish: Res<FishCatalog>,
    points: Res<PointsState>,
    selection: Res<SelectionState>,
    hover: Res<HoverState>,
    debug_layers: Res<LayerDebugSettings>,
    layer_registry: Res<LayerRegistry>,
    layer_runtime: Res<LayerRuntime>,
    view_mode: Res<ViewModeState>,
    map_view: Res<Map2dViewState>,
    terrain_view: Res<Terrain3dViewState>,
) {
    if !bridge.is_changed()
        && !bootstrap.is_changed()
        && !patch_filter.is_changed()
        && !fish_filter.is_changed()
        && !display_state.is_changed()
        && !fish.is_changed()
        && !points.is_changed()
        && !selection.is_changed()
        && !hover.is_changed()
        && !debug_layers.is_changed()
        && !layer_registry.is_changed()
        && !layer_runtime.is_changed()
        && !view_mode.is_changed()
        && !map_view.is_changed()
        && !terrain_view.is_changed()
    {
        return;
    }

    CURRENT_SNAPSHOT.with(|snapshot| {
        let mut snapshot = snapshot.borrow_mut();
        snapshot.ready = bootstrap.meta.is_some()
            && !layer_registry.ordered().is_empty()
            && !fish.entries.is_empty();
        snapshot.theme = bridge.input.theme.clone();
        snapshot.filters = effective_filters(
            &bridge.input,
            &patch_filter,
            &fish_filter,
            &layer_registry,
            &layer_runtime,
        );
        snapshot.ui = effective_ui_state(&bridge.input, &display_state, debug_layers.enabled);
        snapshot.view = effective_view_snapshot(&view_mode, &map_view, &terrain_view);
        snapshot.selection =
            effective_selection_snapshot(selection.info.as_ref(), selection.zone_stats.as_ref());
        snapshot.hover = effective_hover_snapshot(hover.info.as_ref());
        snapshot.catalog = FishyMapCatalogSnapshot {
            capabilities: bridge_capabilities(),
            layers: current_layer_summaries(&layer_registry, &layer_runtime),
            patches: patch_filter
                .patches
                .iter()
                .map(|patch| FishyMapPatchSummary {
                    patch_id: patch.patch_id.0.clone(),
                    patch_name: patch.patch_name.clone(),
                    start_ts_utc: patch.start_ts_utc,
                })
                .collect(),
            fish: fish
                .entries
                .iter()
                .map(|entry| FishyMapFishSummary {
                    fish_id: entry.id,
                    item_id: entry.item_id,
                    encyclopedia_key: entry.encyclopedia_key,
                    encyclopedia_id: entry.encyclopedia_id,
                    name: entry.name.clone(),
                    grade: entry.grade.clone(),
                    is_prize: entry.is_prize,
                })
                .collect(),
        };
        snapshot.statuses = FishyMapStatusSnapshot {
            meta_status: bootstrap.meta_status.clone(),
            layers_status: bootstrap.layers_status.clone(),
            zones_status: bootstrap.zones_status.clone(),
            points_status: points.status.clone(),
            fish_status: fish.status.clone(),
            zone_stats_status: selection.zone_stats_status.clone(),
        };
    });
}
