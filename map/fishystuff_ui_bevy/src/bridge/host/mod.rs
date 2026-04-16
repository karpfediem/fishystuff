mod emission;
mod input;
mod persistence;
mod snapshot;

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::bridge::contract::{
    FishyMapCameraSnapshot, FishyMapCommands, FishyMapFiltersState, FishyMapFishSummary,
    FishyMapHoverSnapshot, FishyMapInputState, FishyMapLayerSummary, FishyMapOutputEvent,
    FishyMapPatchSummary, FishyMapSelectionSnapshot, FishyMapStatePatch, FishyMapStateSnapshot,
    FishyMapStatusSnapshot, FishyMapThemeColors, FishyMapViewMode, FishyMapViewSnapshot,
    FishyMapZoneConfidenceSnapshot, FishyMapZoneDriftSnapshot, FishyMapZoneEvidenceEntrySnapshot,
    FishyMapZoneStatsSnapshot, FishyMapZoneWindowSnapshot,
};
use crate::bridge::theme::parse_css_color;
use crate::bridge::BrowserInputStateSet;
use crate::map::camera::map2d::Map2dViewState;
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::camera::terrain3d::Terrain3dViewState;
use crate::map::layer_query::LayerQuerySample;
use crate::map::layers::{LayerKind, LayerRegistry, LayerRuntime};
use crate::map::terrain::runtime::TerrainDiagnostics;
use crate::map::ui_layers::LayerDebugSettings;
use crate::plugins::api::{
    now_utc_seconds, ApiBootstrapState, FishCatalog, FishFilterState, HoverInfo, HoverState,
    MapDisplayState, PatchFilterState, SelectionState, SemanticFieldFilterState,
    POINT_ICON_SCALE_MAX, POINT_ICON_SCALE_MIN,
};
use crate::plugins::camera::CameraZoomBounds;
use crate::plugins::points::{PointIconCache, PointsState};

thread_local! {
    static EVENT_SINK: RefCell<Option<js_sys::Function>> = const { RefCell::new(None) };
    static PENDING_PATCHES: RefCell<Vec<FishyMapStatePatch>> = const { RefCell::new(Vec::new()) };
    static CURRENT_SNAPSHOT: RefCell<FishyMapStateSnapshot> =
        RefCell::new(snapshot::initial_snapshot());
    static READY_EMITTED: RefCell<bool> = const { RefCell::new(false) };
    static LAST_VIEW_PAYLOAD: RefCell<Option<String>> = const { RefCell::new(None) };
    static LAST_VIEW_EMIT_SECS: RefCell<f64> = const { RefCell::new(0.0) };
    static LAST_HOVER_PAYLOAD: RefCell<Option<String>> = const { RefCell::new(None) };
    static LAST_DIAGNOSTIC_PAYLOAD: RefCell<Option<String>> = const { RefCell::new(None) };
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FishyMapBootstrapSnapshot {
    version: u8,
    ready: bool,
    statuses: FishyMapStatusSnapshot,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct FishyMapProfilingOptions {
    scenario: Option<String>,
    warmup_frames: u64,
    capture_trace: bool,
}

#[wasm_bindgen]
pub fn fishymap_set_event_sink(callback: js_sys::Function) {
    EVENT_SINK.with(|sink| {
        *sink.borrow_mut() = Some(callback);
    });
}

#[wasm_bindgen]
pub fn fishymap_clear_event_sink() {
    EVENT_SINK.with(|sink| {
        *sink.borrow_mut() = None;
    });
}

#[wasm_bindgen]
pub fn fishymap_mount() {}

#[wasm_bindgen]
pub fn fishymap_apply_state_patch_json(json: &str) -> Result<(), JsValue> {
    let _profiling_scope = crate::profiling::scope("bridge.patch_json_parse");
    let patch: FishyMapStatePatch =
        serde_json::from_str(json).map_err(|err| JsValue::from_str(&err.to_string()))?;
    crate::perf_counter_add!("bridge.patch_json_parse.count", 1);
    PENDING_PATCHES.with(|pending| {
        pending.borrow_mut().push(patch);
    });
    Ok(())
}

#[wasm_bindgen]
pub fn fishymap_send_command_json(json: &str) -> Result<(), JsValue> {
    let _profiling_scope = crate::profiling::scope("bridge.command_json_parse");
    let commands: FishyMapCommands =
        serde_json::from_str(json).map_err(|err| JsValue::from_str(&err.to_string()))?;
    crate::perf_counter_add!("bridge.command_json_parse.count", 1);
    PENDING_PATCHES.with(|pending| {
        pending.borrow_mut().push(FishyMapStatePatch {
            commands: Some(commands),
            ..FishyMapStatePatch::default()
        });
    });
    Ok(())
}

#[wasm_bindgen]
pub fn fishymap_get_current_state_json() -> String {
    let _profiling_scope = crate::profiling::scope("bridge.state_export");
    CURRENT_SNAPSHOT.with(|snapshot| {
        crate::perf_counter_add!("bridge.state_export.count", 1);
        serde_json::to_string(&*snapshot.borrow())
            .unwrap_or_else(|_| "{\"version\":1,\"ready\":false}".to_string())
    })
}

#[wasm_bindgen]
pub fn fishymap_get_bootstrap_state_json() -> String {
    let _profiling_scope = crate::profiling::scope("bridge.bootstrap_export");
    CURRENT_SNAPSHOT.with(|snapshot| {
        let snapshot = snapshot.borrow();
        crate::perf_counter_add!("bridge.bootstrap_export.count", 1);
        serde_json::to_string(&FishyMapBootstrapSnapshot {
            version: snapshot.version,
            ready: snapshot.ready,
            statuses: snapshot.statuses.clone(),
        })
        .unwrap_or_else(|_| "{\"version\":1,\"ready\":false}".to_string())
    })
}

#[wasm_bindgen]
pub fn fishymap_reset_profiling_json(json: &str) -> Result<(), JsValue> {
    let options: FishyMapProfilingOptions =
        serde_json::from_str(json).map_err(|err| JsValue::from_str(&err.to_string()))?;
    crate::profiling::start_live_session(
        options.scenario.unwrap_or_else(|| "browser".to_string()),
        options.warmup_frames,
        options.capture_trace,
    );
    Ok(())
}

#[wasm_bindgen]
pub fn fishymap_get_profiling_summary_json() -> String {
    serde_json::to_string(&crate::profiling::live_report())
        .unwrap_or_else(|_| "{\"scenario\":\"browser\"}".to_string())
}

#[wasm_bindgen]
pub fn fishymap_get_profiling_trace_json() -> String {
    crate::profiling::trace_json().unwrap_or_else(|_| "{\"traceEvents\":[]}".to_string())
}

#[wasm_bindgen]
pub fn fishymap_destroy() {
    fishymap_clear_event_sink();
    crate::profiling::clear_live_session();
    PENDING_PATCHES.with(|pending| pending.borrow_mut().clear());
    CURRENT_SNAPSHOT.with(|snapshot| {
        *snapshot.borrow_mut() = snapshot::initial_snapshot();
    });
    READY_EMITTED.with(|value| *value.borrow_mut() = false);
    LAST_VIEW_PAYLOAD.with(|value| *value.borrow_mut() = None);
    LAST_VIEW_EMIT_SECS.with(|value| *value.borrow_mut() = 0.0);
    LAST_HOVER_PAYLOAD.with(|value| *value.borrow_mut() = None);
    LAST_DIAGNOSTIC_PAYLOAD.with(|value| *value.borrow_mut() = None);
}

#[derive(Resource, Default)]
pub struct BrowserBridgeState {
    pub input: FishyMapInputState,
    pending_commands: Vec<FishyMapCommands>,
}

pub struct BrowserBridgePlugin;

impl Plugin for BrowserBridgePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BrowserBridgeState>()
            .configure_sets(PreUpdate, BrowserInputStateSet)
            .add_systems(
                PreUpdate,
                (
                    input::ingest_pending_browser_patches,
                    input::apply_browser_input_state.in_set(BrowserInputStateSet),
                    input::apply_browser_commands,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                (
                    snapshot::sync_current_snapshot,
                    emission::emit_ready_event,
                    emission::emit_view_changed_event,
                    emission::emit_selection_changed_event,
                    emission::emit_hover_changed_event,
                    emission::emit_diagnostic_event,
                )
                    .chain(),
            );
    }
}

pub(super) fn parse_theme_background_color(colors: &FishyMapThemeColors) -> Option<Color> {
    colors
        .base200
        .as_deref()
        .or(colors.base100.as_deref())
        .and_then(parse_css_color)
}

pub(super) fn emit_event(event: &FishyMapOutputEvent) {
    crate::perf_scope!("bridge.emit.dispatch");
    let Ok(json) = serde_json::to_string(event) else {
        return;
    };
    crate::perf_counter_add!("bridge.events.total", 1);
    crate::perf_counter_add!(event_counter_name(event), 1);
    crate::perf_gauge!("bridge.emit.payload_bytes", json.len());
    EVENT_SINK.with(|sink| {
        let Some(callback) = sink.borrow().as_ref().cloned() else {
            return;
        };
        let _ = callback.call1(&JsValue::NULL, &JsValue::from_str(&json));
    });
}

fn event_counter_name(event: &FishyMapOutputEvent) -> &'static str {
    match event {
        FishyMapOutputEvent::Ready { .. } => "bridge.events.ready",
        FishyMapOutputEvent::ViewChanged { .. } => "bridge.events.view_changed",
        FishyMapOutputEvent::SelectionChanged { .. } => "bridge.events.selection_changed",
        FishyMapOutputEvent::HoverChanged { .. } => "bridge.events.hover_changed",
        FishyMapOutputEvent::Diagnostic { .. } => "bridge.events.diagnostic",
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_theme_background_color, *};
    use crate::bridge::theme::parse_css_color;
    use crate::map::camera::map2d::Map2dViewState;
    use crate::map::camera::mode::ViewModeState;
    use crate::map::camera::terrain3d::Terrain3dViewState;
    use crate::map::layers::{
        build_local_layer_specs, AvailableLayerCatalog, FISH_EVIDENCE_LAYER_KEY,
    };
    use crate::map::{exact_lookup::ExactLookupCache, field_metadata::FieldMetadataCache};
    use crate::plugins::api::{
        ApiBootstrapState, FishCatalog, FishFilterState, HoverState, MapDisplayState,
        PatchFilterState, SelectionState, SemanticFieldFilterState, ZoneMembershipLayerFilterState,
    };
    use crate::plugins::bookmarks::BookmarkState;
    use crate::plugins::points::PointsState;
    use fishystuff_api::models::meta::MetaResponse;

    fn clear_pending_patches() {
        PENDING_PATCHES.with(|pending| pending.borrow_mut().clear());
    }

    fn seed_layer_resources(world: &mut World) {
        let available_layers = AvailableLayerCatalog::default();
        let (revision, layers) = build_local_layer_specs(available_layers.entries(), Some("v1"));
        let mut registry = LayerRegistry::default();
        registry.apply_layer_specs(revision, Some("v1".to_string()), layers);
        let mut runtime = LayerRuntime::default();
        runtime.sync_to_registry(&registry);
        world.insert_resource(registry);
        world.insert_resource(runtime);
    }

    #[test]
    fn snapshot_ready_recomputes_from_current_resources_after_remount_like_reset() {
        clear_pending_patches();
        CURRENT_SNAPSHOT.with(|snapshot| {
            *snapshot.borrow_mut() = snapshot::initial_snapshot();
        });
        READY_EMITTED.with(|value| *value.borrow_mut() = false);

        let mut app = App::new();
        app.insert_resource(BrowserBridgeState::default());
        app.insert_resource(ApiBootstrapState {
            meta_status: "meta: loaded".to_string(),
            layers_status: "layers: local (7, v1)".to_string(),
            zones_status: "zones: 287".to_string(),
            meta: Some(MetaResponse::default()),
            ..Default::default()
        });
        app.insert_resource(PatchFilterState::default());
        app.insert_resource(FishFilterState::default());
        app.insert_resource(SemanticFieldFilterState::default());
        app.insert_resource(ZoneMembershipLayerFilterState::default());
        app.insert_resource(MapDisplayState::default());
        app.insert_resource(FishCatalog::default());
        app.insert_resource(PointsState::default());
        app.insert_resource(BookmarkState::default());
        app.insert_resource(SelectionState::default());
        app.insert_resource(HoverState::default());
        app.insert_resource(LayerDebugSettings::default());
        app.insert_resource(ExactLookupCache::default());
        app.insert_resource(FieldMetadataCache::default());
        app.insert_resource(ViewModeState::default());
        app.insert_resource(Map2dViewState::default());
        app.insert_resource(Terrain3dViewState::default());
        seed_layer_resources(&mut app.world_mut());
        app.add_systems(PostUpdate, snapshot::sync_current_snapshot);

        app.world_mut().clear_trackers();
        app.world_mut().resource_mut::<FishCatalog>().status = "fish: loaded".to_string();

        app.update();

        CURRENT_SNAPSHOT.with(|snapshot| {
            assert!(snapshot.borrow().ready);
        });
    }

    #[test]
    fn browser_patch_hides_fish_evidence_layer_from_runtime() {
        clear_pending_patches();

        let mut app = App::new();
        app.insert_resource(BrowserBridgeState::default());
        app.insert_resource(PatchFilterState::default());
        app.insert_resource(FishFilterState::default());
        app.insert_resource(SemanticFieldFilterState::default());
        app.insert_resource(ZoneMembershipLayerFilterState::default());
        app.insert_resource(BookmarkState::default());
        app.insert_resource(MapDisplayState::default());
        app.insert_resource(LayerDebugSettings::default());
        app.insert_resource(ClearColor(Color::BLACK));
        seed_layer_resources(&mut app.world_mut());
        app.add_systems(
            Update,
            (
                input::ingest_pending_browser_patches,
                input::apply_browser_input_state,
            )
                .chain(),
        );

        fishymap_apply_state_patch_json(
            r#"{
                "version": 1,
                "filters": {
                    "layerIdsVisible": ["bookmarks", "zone_mask", "minimap"]
                }
            }"#,
        )
        .expect("queue patch");

        app.update();

        let bridge = app.world().resource::<BrowserBridgeState>();
        assert_eq!(
            bridge.input.filters.layer_ids_visible,
            Some(vec![
                "bookmarks".to_string(),
                "zone_mask".to_string(),
                "minimap".to_string(),
            ])
        );

        let registry = app.world().resource::<LayerRegistry>();
        let runtime = app.world().resource::<LayerRuntime>();
        let fish_evidence_id = registry
            .id_by_key(FISH_EVIDENCE_LAYER_KEY)
            .expect("fish evidence layer");
        assert!(!runtime.visible(fish_evidence_id));

        let display = app.world().resource::<MapDisplayState>();
        assert!(!display.show_points);
    }

    #[test]
    fn browser_patch_propagates_zone_filter_into_bevy_snapshot_state() {
        clear_pending_patches();
        CURRENT_SNAPSHOT.with(|snapshot| {
            *snapshot.borrow_mut() = snapshot::initial_snapshot();
        });

        let mut app = App::new();
        app.insert_resource(BrowserBridgeState::default());
        app.insert_resource(ApiBootstrapState::default());
        app.insert_resource(PatchFilterState::default());
        app.insert_resource(FishFilterState::default());
        app.insert_resource(SemanticFieldFilterState::default());
        app.insert_resource(ZoneMembershipLayerFilterState::default());
        app.insert_resource(MapDisplayState::default());
        app.insert_resource(FishCatalog::default());
        app.insert_resource(PointsState::default());
        app.insert_resource(BookmarkState::default());
        app.insert_resource(SelectionState::default());
        app.insert_resource(HoverState::default());
        app.insert_resource(LayerDebugSettings::default());
        app.insert_resource(ExactLookupCache::default());
        app.insert_resource(FieldMetadataCache::default());
        app.insert_resource(ViewModeState::default());
        app.insert_resource(Map2dViewState::default());
        app.insert_resource(Terrain3dViewState::default());
        app.insert_resource(ClearColor(Color::BLACK));
        seed_layer_resources(&mut app.world_mut());
        app.add_systems(
            Update,
            (
                input::ingest_pending_browser_patches,
                input::apply_browser_input_state,
            )
                .chain(),
        );
        app.add_systems(PostUpdate, snapshot::sync_current_snapshot);

        fishymap_apply_state_patch_json(
            r#"{
                "version": 1,
                "filters": {
                    "zoneRgbs": [15747658],
                    "semanticFieldIdsByLayer": {
                        "zone_mask": [15747658]
                    }
                }
            }"#,
        )
        .expect("queue patch");

        app.update();

        let bridge = app.world().resource::<BrowserBridgeState>();
        assert_eq!(bridge.input.filters.zone_rgbs, vec![15747658]);
        assert_eq!(
            bridge
                .input
                .filters
                .semantic_field_ids_by_layer
                .get("zone_mask")
                .cloned(),
            Some(vec![15747658])
        );

        let semantic = app.world().resource::<SemanticFieldFilterState>();
        assert_eq!(semantic.selected_zone_rgbs(), &[15747658]);

        CURRENT_SNAPSHOT.with(|snapshot| {
            let snapshot = snapshot.borrow();
            assert_eq!(snapshot.filters.zone_rgbs, vec![15747658]);
            assert_eq!(
                snapshot
                    .filters
                    .semantic_field_ids_by_layer
                    .get("zone_mask"),
                Some(&vec![15747658])
            );
        });
    }

    #[test]
    fn prefers_base200_for_theme_background_color() {
        let colors = FishyMapThemeColors {
            base100: Some("#112233".to_string()),
            base200: Some("#223344".to_string()),
            ..Default::default()
        };

        assert_eq!(
            parse_theme_background_color(&colors),
            parse_css_color("#223344")
        );
    }

    #[test]
    fn falls_back_to_base100_for_theme_background_color() {
        let colors = FishyMapThemeColors {
            base100: Some("#112233".to_string()),
            ..Default::default()
        };

        assert_eq!(
            parse_theme_background_color(&colors),
            parse_css_color("#112233")
        );
    }
}
