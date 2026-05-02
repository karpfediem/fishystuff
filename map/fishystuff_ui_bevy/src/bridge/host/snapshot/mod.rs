mod filters;
mod state;
mod view;

use bevy::ecs::system::SystemParam;
use std::cell::Cell;

use self::filters::{
    bridge_capabilities, current_layer_summaries, current_semantic_term_summaries,
};
use self::state::{effective_hover_snapshot, effective_selection_snapshot, effective_ui_state};
use super::*;
use crate::map::field_metadata::FieldMetadataCache;
use crate::plugins::bookmarks::BookmarkState;

pub(super) use self::filters::{current_layer_order, effective_filter_snapshot, effective_filters};
pub(super) use self::state::{hover_layer_samples_snapshot, point_sample_snapshots};
pub(super) use self::view::effective_view_snapshot;

thread_local! {
    static LAST_SEMANTIC_METADATA_REVISION: Cell<u64> = const { Cell::new(0) };
}

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
        || context.search_expression.is_changed()
        || context.layer_effective_filters.is_changed()
        || context.layer_registry.is_changed()
        || context.layer_runtime.is_changed();
    let ui_changed = context.bridge.is_changed()
        || context.debug_layers.is_changed()
        || context.bookmarks.is_changed()
        || context.layer_registry.is_changed()
        || context.layer_runtime.is_changed();
    let view_changed = context.view_mode.is_changed() || context.map_view.is_changed();
    let selection_changed = context.selection.is_changed();
    let hover_changed = context.hover.is_changed();
    let layer_catalog_changed = context.layer_registry.is_changed()
        || context.layer_runtime.is_changed()
        || context.layer_filter_binding_overrides.is_changed();
    let patch_catalog_changed = context.patch_filter.is_changed();
    let fish_catalog_changed = context.fish.is_changed();
    let semantic_catalog_changed = context.layer_registry.is_changed()
        || semantic_metadata_revision_changed(context.field_metadata.revision());
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
        && !semantic_catalog_changed
        && !statuses_changed
    {
        return;
    }

    CURRENT_SNAPSHOT.with(|snapshot| {
        let mut snapshot = snapshot.borrow_mut();
        let mut snapshot_changed = false;

        if ready_changed {
            snapshot_changed |= replace_if_changed(
                &mut snapshot.ready,
                context.bootstrap.meta.is_some() && !context.layer_registry.ordered().is_empty(),
            );
        }
        if theme_changed {
            snapshot_changed |=
                replace_if_changed(&mut snapshot.theme, context.bridge.input.theme.clone());
        }
        if filters_changed {
            snapshot_changed |= replace_if_changed(
                &mut snapshot.filters,
                effective_filters(
                    &context.bridge.input,
                    &context.search_expression,
                    &context.layer_effective_filters,
                    &context.layer_registry,
                    &context.layer_runtime,
                ),
            );
            snapshot_changed |= replace_if_changed(
                &mut snapshot.effective_filters,
                effective_filter_snapshot(
                    &context.bridge.input,
                    &context.search_expression,
                    &context.layer_effective_filters,
                ),
            );
        }
        if ui_changed {
            snapshot_changed |= replace_if_changed(
                &mut snapshot.ui,
                effective_ui_state(
                    &context.bridge.input,
                    &context.display_state,
                    context.debug_layers.enabled,
                    &context.bookmarks,
                    &context.layer_registry,
                    &context.layer_runtime,
                    Some(&context.bootstrap.zones),
                ),
            );
        }
        if view_changed {
            snapshot_changed |= replace_if_changed(
                &mut snapshot.view,
                effective_view_snapshot(&context.view_mode, &context.map_view),
            );
        }
        if selection_changed {
            snapshot_changed |= replace_if_changed(
                &mut snapshot.selection,
                effective_selection_snapshot(
                    context.selection.details_generation,
                    context.selection.details_target.as_ref(),
                    context.selection.info.as_ref(),
                    context.selection.zone_stats.as_ref(),
                ),
            );
        }
        if hover_changed {
            snapshot_changed |= replace_if_changed(
                &mut snapshot.hover,
                effective_hover_snapshot(context.hover.info.as_ref()),
            );
        }
        if layer_catalog_changed {
            snapshot_changed |= replace_if_changed(
                &mut snapshot.catalog.layers,
                current_layer_summaries(
                    &context.layer_registry,
                    &context.layer_runtime,
                    &context.layer_filter_binding_overrides,
                ),
            );
        }
        if patch_catalog_changed {
            snapshot_changed |= replace_if_changed(
                &mut snapshot.catalog.patches,
                context
                    .patch_filter
                    .patches
                    .iter()
                    .map(|patch| FishyMapPatchSummary {
                        patch_id: patch.patch_id.0.clone(),
                        patch_name: patch.patch_name.clone(),
                        start_ts_utc: patch.start_ts_utc,
                    })
                    .collect(),
            );
        }
        if fish_catalog_changed {
            snapshot_changed |= replace_if_changed(
                &mut snapshot.catalog.fish,
                context
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
                    .collect(),
            );
        }
        if semantic_catalog_changed {
            snapshot_changed |= replace_if_changed(
                &mut snapshot.catalog.semantic_terms,
                current_semantic_term_summaries(&context.layer_registry, &context.field_metadata),
            );
            note_semantic_metadata_revision(context.field_metadata.revision());
        }
        if statuses_changed {
            snapshot_changed |= replace_if_changed(
                &mut snapshot.statuses,
                FishyMapStatusSnapshot {
                    meta_status: context.bootstrap.meta_status.clone(),
                    layers_status: context.bootstrap.layers_status.clone(),
                    zones_status: context.bootstrap.zones_status.clone(),
                    points_status: context.points.status.clone(),
                    fish_status: context.fish.status.clone(),
                    zone_stats_status: context.selection.zone_stats_status.clone(),
                },
            );
        }

        if snapshot_changed {
            crate::perf_counter_add!("bridge.snapshot_sync.count", 1);
        }
    });
}

fn replace_if_changed<T: PartialEq>(slot: &mut T, next: T) -> bool {
    if *slot == next {
        return false;
    }
    *slot = next;
    true
}

fn semantic_metadata_revision_changed(revision: u64) -> bool {
    LAST_SEMANTIC_METADATA_REVISION.with(|last_revision| last_revision.get() != revision)
}

fn note_semantic_metadata_revision(revision: u64) {
    LAST_SEMANTIC_METADATA_REVISION.with(|last_revision| {
        last_revision.set(revision);
    });
}

#[derive(SystemParam)]
pub(super) struct SnapshotSyncContext<'w, 's> {
    bridge: Res<'w, BrowserBridgeState>,
    bootstrap: Res<'w, ApiBootstrapState>,
    patch_filter: Res<'w, PatchFilterState>,
    search_expression: Res<'w, SearchExpressionState>,
    layer_effective_filters: Res<'w, LayerEffectiveFilterState>,
    layer_filter_binding_overrides: Res<'w, LayerFilterBindingOverrideState>,
    display_state: Res<'w, MapDisplayState>,
    fish: Res<'w, FishCatalog>,
    points: Res<'w, PointsState>,
    bookmarks: Res<'w, BookmarkState>,
    selection: Res<'w, SelectionState>,
    hover: Res<'w, HoverState>,
    debug_layers: Res<'w, LayerDebugSettings>,
    layer_registry: Res<'w, LayerRegistry>,
    layer_runtime: Res<'w, LayerRuntime>,
    field_metadata: Res<'w, FieldMetadataCache>,
    view_mode: Res<'w, ViewModeState>,
    map_view: Res<'w, Map2dViewState>,
    _marker: std::marker::PhantomData<&'s ()>,
}
