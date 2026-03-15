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
    let ready_changed = bootstrap.is_changed() || layer_registry.is_changed();
    let theme_changed = bridge.is_changed();
    let filters_changed = bridge.is_changed()
        || patch_filter.is_changed()
        || fish_filter.is_changed()
        || layer_registry.is_changed()
        || layer_runtime.is_changed();
    let ui_changed = bridge.is_changed() || display_state.is_changed() || debug_layers.is_changed();
    let view_changed = view_mode.is_changed() || map_view.is_changed() || terrain_view.is_changed();
    let selection_changed = selection.is_changed();
    let hover_changed = hover.is_changed();
    let layer_catalog_changed = layer_registry.is_changed() || layer_runtime.is_changed();
    let patch_catalog_changed = patch_filter.is_changed();
    let fish_catalog_changed = fish.is_changed();
    let statuses_changed = bootstrap.is_changed()
        || points.is_changed()
        || fish.is_changed()
        || selection.is_changed();

    if !ready_changed
        && !theme_changed
        && !filters_changed
        && !ui_changed
        && !view_changed
        && !selection_changed
        && !hover_changed
        && !layer_catalog_changed
        && !patch_catalog_changed
        && !fish_catalog_changed
        && !statuses_changed
    {
        return;
    }

    CURRENT_SNAPSHOT.with(|snapshot| {
        let mut snapshot = snapshot.borrow_mut();
        if ready_changed {
            snapshot.ready = bootstrap.meta.is_some() && !layer_registry.ordered().is_empty();
        }
        if theme_changed {
            snapshot.theme = bridge.input.theme.clone();
        }
        if filters_changed {
            snapshot.filters = effective_filters(
                &bridge.input,
                &patch_filter,
                &fish_filter,
                &layer_registry,
                &layer_runtime,
            );
        }
        if ui_changed {
            snapshot.ui = effective_ui_state(&bridge.input, &display_state, debug_layers.enabled);
        }
        if view_changed {
            snapshot.view = effective_view_snapshot(&view_mode, &map_view, &terrain_view);
        }
        if selection_changed {
            snapshot.selection = effective_selection_snapshot(
                selection.info.as_ref(),
                selection.zone_stats.as_ref(),
            );
        }
        if hover_changed {
            snapshot.hover = effective_hover_snapshot(hover.info.as_ref());
        }
        if layer_catalog_changed {
            snapshot.catalog.layers = current_layer_summaries(&layer_registry, &layer_runtime);
        }
        if patch_catalog_changed {
            snapshot.catalog.patches = patch_filter
                .patches
                .iter()
                .map(|patch| FishyMapPatchSummary {
                    patch_id: patch.patch_id.0.clone(),
                    patch_name: patch.patch_name.clone(),
                    start_ts_utc: patch.start_ts_utc,
                })
                .collect();
        }
        if fish_catalog_changed {
            snapshot.catalog.fish = fish
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
                .collect();
        }
        if statuses_changed {
            snapshot.statuses = FishyMapStatusSnapshot {
                meta_status: bootstrap.meta_status.clone(),
                layers_status: bootstrap.layers_status.clone(),
                zones_status: bootstrap.zones_status.clone(),
                points_status: points.status.clone(),
                fish_status: fish.status.clone(),
                zone_stats_status: selection.zone_stats_status.clone(),
            };
        }
    });
}
