mod filters;
mod state;
mod view;

use bevy::ecs::system::SystemParam;

use self::filters::{bridge_capabilities, current_layer_summaries};
use self::state::{effective_hover_snapshot, effective_selection_snapshot, effective_ui_state};
use super::*;
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_metadata::FieldMetadataCache;
use crate::plugins::bookmarks::BookmarkState;
use crate::plugins::vector_layers::VectorLayerRuntime;

pub(super) use self::filters::{current_layer_order, effective_filters};
pub(super) use self::state::hover_layer_samples_snapshot;
pub(super) use self::view::effective_view_snapshot;

pub(super) fn initial_snapshot() -> FishyMapStateSnapshot {
    let mut snapshot = FishyMapStateSnapshot::default();
    snapshot.catalog.capabilities = bridge_capabilities();
    snapshot
}

pub(super) fn sync_current_snapshot(context: SnapshotSyncContext<'_, '_>) {
    crate::perf_scope!("bridge.snapshot_sync");
    let ready_changed = context.bootstrap.is_changed() || context.layer_registry.is_changed();
    let theme_changed = context.bridge.is_changed();
    let filters_changed = context.bridge.is_changed()
        || context.patch_filter.is_changed()
        || context.fish_filter.is_changed()
        || context.semantic_filter.is_changed()
        || context.layer_registry.is_changed()
        || context.layer_runtime.is_changed();
    let ui_changed = context.bridge.is_changed()
        || context.display_state.is_changed()
        || context.debug_layers.is_changed()
        || context.bookmarks.is_changed()
        || context.exact_lookups.is_changed()
        || context.field_metadata.is_changed();
    let view_changed = context.view_mode.is_changed()
        || context.map_view.is_changed()
        || context.terrain_view.is_changed();
    let selection_changed = context.selection.is_changed();
    let hover_changed = context.hover.is_changed();
    let layer_catalog_changed =
        context.layer_registry.is_changed() || context.layer_runtime.is_changed();
    let patch_catalog_changed = context.patch_filter.is_changed();
    let fish_catalog_changed = context.fish.is_changed();
    let statuses_changed = context.bootstrap.is_changed()
        || context.points.is_changed()
        || context.fish.is_changed()
        || context.selection.is_changed();

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

    crate::perf_counter_add!("bridge.snapshot_sync.count", 1);

    CURRENT_SNAPSHOT.with(|snapshot| {
        let mut snapshot = snapshot.borrow_mut();
        if ready_changed {
            snapshot.ready =
                context.bootstrap.meta.is_some() && !context.layer_registry.ordered().is_empty();
        }
        if theme_changed {
            snapshot.theme = context.bridge.input.theme.clone();
        }
        if filters_changed {
            snapshot.filters = effective_filters(
                &context.bridge.input,
                &context.patch_filter,
                &context.fish_filter,
                &context.semantic_filter,
                &context.layer_registry,
                &context.layer_runtime,
            );
        }
        if ui_changed {
            snapshot.ui = effective_ui_state(
                &context.bridge.input,
                &context.display_state,
                context.debug_layers.enabled,
                &context.bookmarks,
                &context.layer_registry,
                &context.exact_lookups,
                &context.field_metadata,
            );
        }
        if view_changed {
            snapshot.view = effective_view_snapshot(
                &context.view_mode,
                &context.map_view,
                &context.terrain_view,
            );
        }
        if selection_changed {
            snapshot.selection = effective_selection_snapshot(
                context.selection.info.as_ref(),
                context.selection.zone_stats.as_ref(),
            );
        }
        if hover_changed {
            snapshot.hover = effective_hover_snapshot(context.hover.info.as_ref());
        }
        if layer_catalog_changed {
            snapshot.catalog.layers =
                current_layer_summaries(&context.layer_registry, &context.layer_runtime);
        }
        if patch_catalog_changed {
            snapshot.catalog.patches = context
                .patch_filter
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
            snapshot.catalog.fish = context
                .fish
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
                meta_status: context.bootstrap.meta_status.clone(),
                layers_status: context.bootstrap.layers_status.clone(),
                zones_status: context.bootstrap.zones_status.clone(),
                points_status: context.points.status.clone(),
                fish_status: context.fish.status.clone(),
                zone_stats_status: context.selection.zone_stats_status.clone(),
            };
        }
    });
}

#[derive(SystemParam)]
pub(super) struct SnapshotSyncContext<'w, 's> {
    bridge: Res<'w, BrowserBridgeState>,
    bootstrap: Res<'w, ApiBootstrapState>,
    patch_filter: Res<'w, PatchFilterState>,
    fish_filter: Res<'w, FishFilterState>,
    semantic_filter: Res<'w, SemanticFieldFilterState>,
    display_state: Res<'w, MapDisplayState>,
    fish: Res<'w, FishCatalog>,
    points: Res<'w, PointsState>,
    bookmarks: Res<'w, BookmarkState>,
    selection: Res<'w, SelectionState>,
    hover: Res<'w, HoverState>,
    debug_layers: Res<'w, LayerDebugSettings>,
    layer_registry: Res<'w, LayerRegistry>,
    layer_runtime: Res<'w, LayerRuntime>,
    exact_lookups: Res<'w, ExactLookupCache>,
    field_metadata: Res<'w, FieldMetadataCache>,
    vector_runtime: Res<'w, VectorLayerRuntime>,
    view_mode: Res<'w, ViewModeState>,
    map_view: Res<'w, Map2dViewState>,
    terrain_view: Res<'w, Terrain3dViewState>,
    _marker: std::marker::PhantomData<&'s ()>,
}
