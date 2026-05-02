use bevy::ecs::system::SystemParam;
use bevy::input::touch::Touches;
use bevy::input::ButtonInput;
use bevy::window::PrimaryWindow;

use crate::bridge::contract::{FishyMapSelectionHistoryBehavior, FishyMapSelectionPointKind};
use crate::map::camera::map2d::map2d_cursor_to_world;
use crate::map::camera::map2d::Map2dViewState;
use crate::map::events::EventsSnapshotState;
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::hover_query::{hover_info_at_world_point, WorldPointQueryContext};
use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::map::raster::RasterTileCache;
use crate::map::selection_query::{selected_info_at_world_point, selected_info_from_hover};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::WorldPoint;
use crate::plugins::api::{
    build_zone_stats_request, spawn_zone_stats_request, ApiBootstrapState, HoverInfo, HoverState,
    LayerEffectiveFilterState, MapDisplayState, PatchFilterState, PendingRequests,
    PointSampleSummary, SelectedInfo, SelectionState,
};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::input::PanState;
use crate::plugins::points::{
    point_hover_samples_at_world_point, point_samples_at_world_point, PointsState,
};
use crate::plugins::ui::UiPointerCapture;
use crate::plugins::vector_layers::VectorLayerRuntime;
use crate::plugins::waypoint_layers::{
    waypoint_layers_pending, waypoint_sample_at_world_point,
    waypoint_sample_at_world_point_with_options, WaypointLayerInteractionSample,
    WaypointLayerRuntime, WaypointSampleOptions,
};
use crate::prelude::*;

pub struct MaskPlugin;

impl Plugin for MaskPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExactLookupCache>()
            .init_resource::<WaypointLayerRuntime>()
            .init_resource::<PendingSelectionDetails>()
            .add_systems(
                Update,
                (
                    update_hover,
                    handle_click,
                    process_pending_selection_details,
                )
                    .chain(),
            );
    }
}

#[derive(Resource, Default)]
pub(crate) struct PendingSelectionDetails {
    request: Option<PendingSelectionDetailsRequest>,
}

#[derive(Debug, Clone)]
struct PendingSelectionDetailsRequest {
    details_generation: u64,
    click_world_point: WorldPoint,
    selected_world_point: WorldPoint,
    waypoint_sample: Option<WaypointLayerInteractionSample>,
    point_kind: FishyMapSelectionPointKind,
    point_label: Option<String>,
    stage: PendingSelectionDetailsStage,
    defer_frames: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingSelectionDetailsStage {
    ProbeWaypoint,
    BuildLayerSelection,
    BuildPointSamples,
}

fn hovered_zone_rgb(info: Option<&crate::plugins::api::HoverInfo>) -> Option<u32> {
    info.and_then(crate::plugins::api::HoverInfo::zone_rgb_u32)
}

fn set_hover_state(
    hover: &mut ResMut<'_, HoverState>,
    display_state: &mut MapDisplayState,
    next_info: Option<crate::plugins::api::HoverInfo>,
) {
    match hover_storage_update(hover.info.as_ref(), next_info.as_ref()) {
        HoverStorageUpdate::None => {}
        HoverStorageUpdate::CoordinatesOnly => {
            hover.bypass_change_detection().info = next_info.clone();
        }
        HoverStorageUpdate::Content => {
            hover.info = next_info.clone();
        }
    }
    let next_hovered_zone_rgb = hovered_zone_rgb(next_info.as_ref());
    if display_state.hovered_zone_rgb != next_hovered_zone_rgb {
        display_state.hovered_zone_rgb = next_hovered_zone_rgb;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoverStorageUpdate {
    None,
    CoordinatesOnly,
    Content,
}

fn hover_storage_update(
    current: Option<&crate::plugins::api::HoverInfo>,
    next: Option<&crate::plugins::api::HoverInfo>,
) -> HoverStorageUpdate {
    if current == next {
        HoverStorageUpdate::None
    } else if hover_content_matches(current, next) {
        HoverStorageUpdate::CoordinatesOnly
    } else {
        HoverStorageUpdate::Content
    }
}

fn hover_content_matches(
    current: Option<&crate::plugins::api::HoverInfo>,
    next: Option<&crate::plugins::api::HoverInfo>,
) -> bool {
    match (current, next) {
        (None, None) => true,
        (Some(current), Some(next)) => {
            current.layer_samples == next.layer_samples
                && current.point_samples == next.point_samples
        }
        _ => false,
    }
}

fn clear_hover_state(hover: &mut ResMut<'_, HoverState>, display_state: &mut MapDisplayState) {
    set_hover_state(hover, display_state, None);
}

fn update_hover(mut context: HoverUpdateContext<'_, '_>) {
    if hover_interaction_blocked(&context) {
        let next_hovered_zone_rgb = hovered_zone_rgb(context.hover.info.as_ref());
        if context.display_state.hovered_zone_rgb != next_hovered_zone_rgb {
            context.display_state.hovered_zone_rgb = next_hovered_zone_rgb;
        }
        return;
    }
    if context.ui_capture.blocked {
        clear_hover_state(&mut context.hover, &mut context.display_state);
        return;
    }
    let Some(world_point) = hover_world_point(&context) else {
        clear_hover_state(&mut context.hover, &mut context.display_state);
        return;
    };
    let point_samples = point_hover_samples_at_world_point(
        world_point,
        &context.points,
        &context.snapshot,
        &context.display_state,
        &context.point_camera_q,
    );
    let waypoint_sample = waypoint_sample_at_world_point(
        world_point,
        &context.waypoint_runtime,
        &context.layer_registry,
        &context.layer_runtime,
        &context.exact_lookups,
        &context.tile_cache,
        &context.vector_runtime,
        &context.layer_filters,
        &context.point_camera_q,
    );
    let Some(mut next_hover) = waypoint_sample
        .as_ref()
        .map(|sample| waypoint_hover_info(world_point, sample))
        .or_else(|| {
            hover_info_at_world_point(
                world_point,
                &WorldPointQueryContext {
                    layer_registry: &context.layer_registry,
                    layer_runtime: &context.layer_runtime,
                    exact_lookups: &context.exact_lookups,
                    field_metadata: &context.field_metadata,
                    tile_cache: &context.tile_cache,
                    vector_runtime: &context.vector_runtime,
                    layer_filters: &context.layer_filters,
                    map_to_world: MapToWorld::default(),
                },
            )
        })
        .or_else(|| point_hover_info(world_point, point_samples.clone()))
    else {
        clear_hover_state(&mut context.hover, &mut context.display_state);
        return;
    };
    if !point_samples.is_empty() {
        next_hover.point_samples = point_samples;
    }
    set_hover_state(
        &mut context.hover,
        &mut context.display_state,
        Some(next_hover),
    );
}

fn handle_click(mut context: MaskClickContext<'_, '_>) {
    crate::perf_scope!("selection.click.quick");
    if context.ui_capture.blocked {
        return;
    }
    if !context.mouse_buttons.just_released(MouseButton::Left)
        && !context.touches.any_just_released()
    {
        return;
    }
    if context.pan.drag_distance > DRAG_THRESHOLD {
        return;
    }
    let Some(world_point) =
        interaction_world_point(&context.windows, &context.camera_q, &context.touches)
    else {
        return;
    };
    let details_generation = context.selection.begin_details_selection(
        "point",
        Some(world_point),
        Some(FishyMapSelectionPointKind::Clicked),
        None,
        FishyMapSelectionHistoryBehavior::Append,
    );
    let quick_selected_info = quick_selected_info(world_point, None, context.hover.info.as_ref());
    apply_selection_without_zone_stats_request(
        &mut context.selection,
        &mut context.pending,
        quick_selected_info,
    );
    context.pending_selection_details.request = Some(PendingSelectionDetailsRequest {
        details_generation,
        click_world_point: world_point,
        selected_world_point: world_point,
        waypoint_sample: None,
        point_kind: FishyMapSelectionPointKind::Clicked,
        point_label: None,
        stage: PendingSelectionDetailsStage::ProbeWaypoint,
        defer_frames: 1,
    });
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn queue_selection_details(
    pending_selection_details: &mut PendingSelectionDetails,
    details_generation: u64,
    world_point: WorldPoint,
    point_kind: FishyMapSelectionPointKind,
    point_label: Option<String>,
) {
    pending_selection_details.request = Some(PendingSelectionDetailsRequest {
        details_generation,
        click_world_point: world_point,
        selected_world_point: world_point,
        waypoint_sample: None,
        point_kind,
        point_label,
        stage: PendingSelectionDetailsStage::ProbeWaypoint,
        defer_frames: 1,
    });
}

fn process_pending_selection_details(mut context: SelectionDetailsContext<'_, '_>) {
    let Some(mut request) = context.pending_selection_details.request.take() else {
        return;
    };
    if request.defer_frames > 0 {
        request.defer_frames -= 1;
        context.pending_selection_details.request = Some(request);
        return;
    }
    if selection_details_should_yield(&context) {
        context.pending_selection_details.request = Some(request);
        return;
    }
    if request.details_generation != context.selection.details_generation {
        return;
    }

    match request.stage {
        PendingSelectionDetailsStage::ProbeWaypoint => {
            crate::perf_scope!("selection.click.details.waypoint");
            let waypoint_options = WaypointSampleOptions {
                include_hidden_layers: matches!(
                    request.point_kind,
                    FishyMapSelectionPointKind::Waypoint
                ),
            };
            let waypoint_sample = waypoint_sample_at_world_point_with_options(
                request.click_world_point,
                &context.waypoint_runtime,
                &context.layer_registry,
                &context.layer_runtime,
                &context.exact_lookups,
                &context.tile_cache,
                &context.vector_runtime,
                &context.layer_filters,
                &context.point_camera_q,
                waypoint_options,
            );
            if waypoint_sample.is_none()
                && waypoint_options.include_hidden_layers
                && waypoint_layers_pending(
                    &context.waypoint_runtime,
                    &context.layer_registry,
                    &context.layer_runtime,
                    waypoint_options,
                )
            {
                context.pending_selection_details.request = Some(request);
                return;
            }
            request.selected_world_point = waypoint_sample
                .as_ref()
                .map(|sample| WorldPoint::new(sample.world_x, sample.world_z))
                .unwrap_or(request.click_world_point);
            request.waypoint_sample = waypoint_sample;
            request.stage = PendingSelectionDetailsStage::BuildLayerSelection;
            context.pending_selection_details.request = Some(request);
        }
        PendingSelectionDetailsStage::BuildLayerSelection => {
            crate::perf_scope!("selection.click.details.layers");
            apply_pending_selection_layer_details(&mut context, &request);
            request.stage = PendingSelectionDetailsStage::BuildPointSamples;
            context.pending_selection_details.request = Some(request);
        }
        PendingSelectionDetailsStage::BuildPointSamples => {
            crate::perf_scope!("selection.click.details.points");
            apply_pending_selection_point_samples(&mut context, &request);
        }
    }
}

fn apply_pending_selection_layer_details(
    context: &mut SelectionDetailsContext<'_, '_>,
    request: &PendingSelectionDetailsRequest,
) {
    let query_context = WorldPointQueryContext {
        layer_registry: &context.layer_registry,
        layer_runtime: &context.layer_runtime,
        exact_lookups: &context.exact_lookups,
        field_metadata: &context.field_metadata,
        tile_cache: &context.tile_cache,
        vector_runtime: &context.vector_runtime,
        layer_filters: &context.layer_filters,
        map_to_world: MapToWorld::default(),
    };
    let Some(selected_info) = request
        .waypoint_sample
        .as_ref()
        .map(|sample| {
            waypoint_selected_info_at_exact_world_point(
                sample,
                &query_context,
                Some(&context.bootstrap.zones),
            )
        })
        .or_else(|| {
            selected_info_at_world_point(
                request.click_world_point,
                &query_context,
                request.point_kind,
                request.point_label.as_deref(),
                Some(&context.bootstrap.zones),
            )
        })
    else {
        context.selection.zone_stats_status = "zone stats: unavailable".to_string();
        return;
    };
    apply_selection_with_zone_stats_request(
        &context.bootstrap,
        &context.patch_filter,
        &mut context.selection,
        &mut context.pending,
        selected_info,
    );
}

fn apply_pending_selection_point_samples(
    context: &mut SelectionDetailsContext<'_, '_>,
    request: &PendingSelectionDetailsRequest,
) {
    let point_samples = point_samples_at_world_point(
        request.selected_world_point,
        &context.points,
        &context.snapshot,
        &context.display_state,
        &context.point_camera_q,
    );
    if point_samples.is_empty() {
        return;
    }
    let mut selected_info = context
        .selection
        .info
        .clone()
        .or_else(|| {
            point_selected_info(
                request.click_world_point,
                request.point_kind,
                request.point_label.as_deref(),
                point_samples.clone(),
            )
        })
        .unwrap_or_else(|| quick_selected_info(request.click_world_point, None, None));
    selected_info.point_samples = point_samples;
    context.selection.info = Some(selected_info);
}

fn apply_selection_without_zone_stats_request(
    selection: &mut SelectionState,
    pending: &mut PendingRequests,
    selected_info: SelectedInfo,
) {
    selection.info = Some(selected_info);
    selection.zone_stats = None;
    selection.zone_stats_status = "zone stats: pending details".to_string();
    pending.zone_stats = None;
}

fn apply_selection_with_zone_stats_request(
    bootstrap: &ApiBootstrapState,
    patch_filter: &PatchFilterState,
    selection: &mut SelectionState,
    pending: &mut PendingRequests,
    selected_info: SelectedInfo,
) {
    let zone_rgb = selected_info.zone_rgb();
    let zone_rgb_u32 = selected_info.zone_rgb_u32();
    selection.info = Some(selected_info);
    selection.zone_stats = None;
    pending.zone_stats = None;

    let Some(rgb) = zone_rgb else {
        selection.zone_stats_status = "zone stats: unavailable".to_string();
        return;
    };
    selection.zone_stats_status = "zone stats: loading".to_string();

    let Some(request) = build_zone_stats_request(bootstrap, patch_filter, rgb) else {
        selection.zone_stats_status = "zone stats: missing defaults".to_string();
        return;
    };

    let Some(rgb_u32) = zone_rgb_u32 else {
        selection.zone_stats_status = "zone stats: unavailable".to_string();
        return;
    };
    let receiver = spawn_zone_stats_request(request);
    pending.zone_stats = Some((rgb_u32, receiver));
}

fn hover_world_point(context: &HoverUpdateContext<'_, '_>) -> Option<WorldPoint> {
    interaction_world_point(&context.windows, &context.camera_q, &context.touches)
}

fn map_pixel_for_world_point(world_point: WorldPoint) -> (i32, i32) {
    let map = MapToWorld::default().world_to_map(world_point);
    (map.x.floor() as i32, map.y.floor() as i32)
}

fn point_hover_info(
    world_point: WorldPoint,
    point_samples: Vec<PointSampleSummary>,
) -> Option<HoverInfo> {
    if point_samples.is_empty() {
        return None;
    }
    let (map_px, map_py) = map_pixel_for_world_point(world_point);
    Some(HoverInfo {
        map_px,
        map_py,
        world_x: world_point.x,
        world_z: world_point.z,
        layer_samples: Vec::new(),
        point_samples,
    })
}

fn waypoint_hover_info(
    cursor_world_point: WorldPoint,
    sample: &WaypointLayerInteractionSample,
) -> HoverInfo {
    let (map_px, map_py) = map_pixel_for_world_point(cursor_world_point);
    HoverInfo {
        map_px,
        map_py,
        world_x: sample.world_x,
        world_z: sample.world_z,
        layer_samples: vec![sample.layer_sample.clone()],
        point_samples: Vec::new(),
    }
}

fn point_selected_info(
    world_point: WorldPoint,
    point_kind: FishyMapSelectionPointKind,
    point_label: Option<&str>,
    point_samples: Vec<PointSampleSummary>,
) -> Option<SelectedInfo> {
    if point_samples.is_empty() {
        return None;
    }
    let (map_px, map_py) = map_pixel_for_world_point(world_point);
    Some(SelectedInfo {
        map_px,
        map_py,
        world_x: world_point.x,
        world_z: world_point.z,
        sampled_world_point: true,
        point_kind: Some(point_kind),
        point_label: point_label
            .map(ToOwned::to_owned)
            .or_else(|| Some("Ranking Samples".to_string())),
        layer_samples: Vec::new(),
        point_samples,
    })
}

fn waypoint_selected_info(sample: &WaypointLayerInteractionSample) -> SelectedInfo {
    let (map_px, map_py) =
        map_pixel_for_world_point(WorldPoint::new(sample.world_x, sample.world_z));
    SelectedInfo {
        map_px,
        map_py,
        world_x: sample.world_x,
        world_z: sample.world_z,
        sampled_world_point: true,
        point_kind: Some(FishyMapSelectionPointKind::Waypoint),
        point_label: sample.point_label.clone(),
        layer_samples: vec![sample.layer_sample.clone()],
        point_samples: Vec::new(),
    }
}

fn waypoint_selected_info_at_exact_world_point(
    sample: &WaypointLayerInteractionSample,
    query_context: &WorldPointQueryContext<'_>,
    zone_names: Option<&std::collections::HashMap<u32, Option<String>>>,
) -> SelectedInfo {
    let exact_world_point = WorldPoint::new(sample.world_x, sample.world_z);
    let mut selected = selected_info_at_world_point(
        exact_world_point,
        query_context,
        FishyMapSelectionPointKind::Waypoint,
        sample.point_label.as_deref(),
        zone_names,
    )
    .unwrap_or_else(|| waypoint_selected_info(sample));
    selected.world_x = sample.world_x;
    selected.world_z = sample.world_z;
    selected.sampled_world_point = true;
    selected.point_kind = Some(crate::bridge::contract::FishyMapSelectionPointKind::Waypoint);
    if sample.point_label.is_some() {
        selected.point_label = sample.point_label.clone();
    }
    selected
        .layer_samples
        .insert(0, sample.layer_sample.clone());
    selected
}

fn interaction_world_point(
    windows: &Query<&Window, With<PrimaryWindow>>,
    camera_q: &Query<(&Projection, &Transform), With<Map2dCamera>>,
    touches: &Touches,
) -> Option<WorldPoint> {
    let Ok(window) = windows.single() else {
        return None;
    };
    let Ok((projection, camera_transform)) = camera_q.single() else {
        return None;
    };
    let cursor = window
        .cursor_position()
        .or_else(|| touch_hover_position(touches))?;
    let world = map2d_cursor_to_world(window, projection, camera_transform, cursor)?;
    Some(WorldPoint::new(world.x as f64, world.y as f64))
}

fn hover_interaction_blocked(context: &HoverUpdateContext<'_, '_>) -> bool {
    let active_touch_count = context.touches.iter().count();
    context.mouse_buttons.pressed(MouseButton::Left)
        || context.mouse_buttons.just_released(MouseButton::Left)
        || context.touches.any_just_released()
        || active_touch_count >= 2
        || (active_touch_count == 1 && context.pan.drag_distance > DRAG_THRESHOLD)
}

fn selection_details_should_yield(context: &SelectionDetailsContext<'_, '_>) -> bool {
    let active_touch_count = context.touches.iter().count();
    context.mouse_buttons.pressed(MouseButton::Left)
        || context.mouse_buttons.just_released(MouseButton::Left)
        || context.touches.any_just_released()
        || active_touch_count > 0
        || context.pan.drag_distance > DRAG_THRESHOLD
        || context.map_view.is_changed()
        || context.hover.is_changed()
}

fn hover_matches_world_point(hover: &HoverInfo, world_point: WorldPoint) -> bool {
    let (map_px, map_py) = map_pixel_for_world_point(world_point);
    hover.map_px == map_px && hover.map_py == map_py
}

fn quick_selected_info(
    world_point: WorldPoint,
    waypoint_sample: Option<&WaypointLayerInteractionSample>,
    hover: Option<&HoverInfo>,
) -> SelectedInfo {
    if let Some(sample) = waypoint_sample {
        return waypoint_selected_info(sample);
    }
    if let Some(hover_info) =
        hover.filter(|hover_info| hover_matches_world_point(hover_info, world_point))
    {
        if let Some(mut selected_info) = selected_info_from_hover(hover_info) {
            let (map_px, map_py) = map_pixel_for_world_point(world_point);
            selected_info.map_px = map_px;
            selected_info.map_py = map_py;
            selected_info.world_x = world_point.x;
            selected_info.world_z = world_point.z;
            selected_info.sampled_world_point = true;
            selected_info.point_kind = Some(FishyMapSelectionPointKind::Clicked);
            return selected_info;
        }
    }
    let (map_px, map_py) = map_pixel_for_world_point(world_point);
    SelectedInfo {
        map_px,
        map_py,
        world_x: world_point.x,
        world_z: world_point.z,
        sampled_world_point: true,
        point_kind: Some(FishyMapSelectionPointKind::Clicked),
        point_label: Some("Clicked point".to_string()),
        layer_samples: Vec::new(),
        point_samples: Vec::new(),
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn quick_world_point_selection_info(
    world_point: WorldPoint,
    point_kind: FishyMapSelectionPointKind,
    point_label: Option<&str>,
) -> SelectedInfo {
    let (map_px, map_py) = map_pixel_for_world_point(world_point);
    SelectedInfo {
        map_px,
        map_py,
        world_x: world_point.x,
        world_z: world_point.z,
        sampled_world_point: true,
        point_kind: Some(point_kind),
        point_label: point_label.map(ToOwned::to_owned),
        layer_samples: Vec::new(),
        point_samples: Vec::new(),
    }
}

#[derive(SystemParam)]
struct HoverUpdateContext<'w, 's> {
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    touches: Res<'w, Touches>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    camera_q: Query<'w, 's, (&'static Projection, &'static Transform), With<Map2dCamera>>,
    exact_lookups: Res<'w, ExactLookupCache>,
    field_metadata: Res<'w, FieldMetadataCache>,
    tile_cache: Res<'w, RasterTileCache>,
    display_state: ResMut<'w, MapDisplayState>,
    ui_capture: Res<'w, UiPointerCapture>,
    hover: ResMut<'w, HoverState>,
    pan: Res<'w, PanState>,
    points: Res<'w, PointsState>,
    snapshot: Res<'w, EventsSnapshotState>,
    layer_registry: Res<'w, LayerRegistry>,
    layer_runtime: Res<'w, LayerRuntime>,
    vector_runtime: Res<'w, VectorLayerRuntime>,
    waypoint_runtime: Res<'w, WaypointLayerRuntime>,
    layer_filters: Res<'w, LayerEffectiveFilterState>,
    point_camera_q: Query<'w, 's, &'static Projection, With<Map2dCamera>>,
}

#[derive(SystemParam)]
struct MaskClickContext<'w, 's> {
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    touches: Res<'w, Touches>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    camera_q: Query<'w, 's, (&'static Projection, &'static Transform), With<Map2dCamera>>,
    pending: ResMut<'w, PendingRequests>,
    pending_selection_details: ResMut<'w, PendingSelectionDetails>,
    selection: ResMut<'w, SelectionState>,
    hover: Res<'w, HoverState>,
    pan: Res<'w, PanState>,
    ui_capture: Res<'w, UiPointerCapture>,
    _marker: std::marker::PhantomData<&'s ()>,
}

#[derive(SystemParam)]
struct SelectionDetailsContext<'w, 's> {
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    touches: Res<'w, Touches>,
    exact_lookups: Res<'w, ExactLookupCache>,
    field_metadata: Res<'w, FieldMetadataCache>,
    tile_cache: Res<'w, RasterTileCache>,
    layer_registry: Res<'w, LayerRegistry>,
    layer_runtime: Res<'w, LayerRuntime>,
    vector_runtime: Res<'w, VectorLayerRuntime>,
    waypoint_runtime: Res<'w, WaypointLayerRuntime>,
    layer_filters: Res<'w, LayerEffectiveFilterState>,
    pending: ResMut<'w, PendingRequests>,
    pending_selection_details: ResMut<'w, PendingSelectionDetails>,
    selection: ResMut<'w, SelectionState>,
    points: Res<'w, PointsState>,
    snapshot: Res<'w, EventsSnapshotState>,
    display_state: Res<'w, MapDisplayState>,
    bootstrap: Res<'w, ApiBootstrapState>,
    patch_filter: Res<'w, PatchFilterState>,
    hover: Res<'w, HoverState>,
    pan: Res<'w, PanState>,
    map_view: Res<'w, Map2dViewState>,
    point_camera_q: Query<'w, 's, &'static Projection, With<Map2dCamera>>,
    _marker: std::marker::PhantomData<&'s ()>,
}

fn touch_hover_position(touches: &Touches) -> Option<Vec2> {
    touches.first_pressed_position().or_else(|| {
        touches
            .iter_just_released()
            .next()
            .map(|touch| touch.position())
    })
}

#[cfg(test)]
mod tests {
    use super::{
        hover_content_matches, hover_storage_update, hovered_zone_rgb, quick_selected_info,
        HoverStorageUpdate,
    };
    use crate::bridge::contract::FishyMapSelectionPointKind;
    use crate::map::spaces::WorldPoint;
    use crate::plugins::api::{HoverInfo, PointSampleSummary};
    use crate::plugins::waypoint_layers::WaypointLayerInteractionSample;

    fn hover_info(map_px: i32, world_x: f64, zone_rgb: u32) -> HoverInfo {
        HoverInfo {
            map_px,
            map_py: 34,
            world_x,
            world_z: 2.0,
            layer_samples: vec![zone_sample(zone_rgb)],
            point_samples: Vec::new(),
        }
    }

    fn zone_sample(zone_rgb: u32) -> crate::map::layer_query::LayerQuerySample {
        crate::map::layer_query::LayerQuerySample {
            layer_id: "zone_mask".to_string(),
            layer_name: "Zone Mask".to_string(),
            kind: "field".to_string(),
            rgb: fishystuff_api::Rgb::from_u32(zone_rgb),
            rgb_u32: zone_rgb,
            field_id: Some(zone_rgb),
            targets: Vec::new(),
            detail_pane: None,
            detail_sections: Vec::new(),
        }
    }

    #[test]
    fn hovered_zone_rgb_reads_zone_from_hover_info() {
        let info = hover_info(12, 1.0, 0x123456);
        assert_eq!(hovered_zone_rgb(Some(&info)), Some(0x123456));
        assert_eq!(hovered_zone_rgb(None), None);
    }

    #[test]
    fn hover_content_match_ignores_coordinate_only_motion() {
        let current = hover_info(12, 1.0, 0x123456);
        let next = hover_info(99, 88.0, 0x123456);

        assert!(hover_content_matches(Some(&current), Some(&next)));
    }

    #[test]
    fn hover_content_match_tracks_sample_changes() {
        let current = hover_info(12, 1.0, 0x123456);
        let mut next = hover_info(99, 88.0, 0x123456);
        next.point_samples.push(PointSampleSummary {
            fish_id: 116,
            sample_count: 1,
            last_ts_utc: 1_700_000_000,
            sample_id: None,
            zone_rgbs: vec![0x123456],
            full_zone_rgbs: vec![0x123456],
        });

        assert!(!hover_content_matches(Some(&current), Some(&next)));
    }

    #[test]
    fn hover_content_match_tracks_zone_changes() {
        let current = hover_info(12, 1.0, 0x123456);
        let next = hover_info(99, 88.0, 0x654321);

        assert!(!hover_content_matches(Some(&current), Some(&next)));
    }

    #[test]
    fn hover_storage_update_detects_coordinate_only_motion() {
        let current = hover_info(12, 1.0, 0x123456);
        let next = hover_info(99, 88.0, 0x123456);

        assert_eq!(
            hover_storage_update(Some(&current), Some(&next)),
            HoverStorageUpdate::CoordinatesOnly,
        );
    }

    #[test]
    fn hover_storage_update_detects_content_motion() {
        let current = hover_info(12, 1.0, 0x123456);
        let next = hover_info(99, 88.0, 0x654321);

        assert_eq!(
            hover_storage_update(Some(&current), Some(&next)),
            HoverStorageUpdate::Content,
        );
    }

    #[test]
    fn hover_storage_update_detects_clears() {
        let current = hover_info(12, 1.0, 0x123456);

        assert_eq!(
            hover_storage_update(Some(&current), None),
            HoverStorageUpdate::Content,
        );
    }

    #[test]
    fn hover_storage_update_detects_noop() {
        let current = hover_info(12, 1.0, 0x123456);

        assert_eq!(
            hover_storage_update(Some(&current), Some(&current)),
            HoverStorageUpdate::None,
        );
        assert_eq!(hover_storage_update(None, None), HoverStorageUpdate::None);
    }

    #[test]
    fn quick_selection_prefers_waypoint_feedback() {
        let sample = WaypointLayerInteractionSample {
            world_x: 10.0,
            world_z: 20.0,
            point_label: Some("Chunsu".to_string()),
            layer_sample: zone_sample(0x123456),
        };

        let selected = quick_selected_info(WorldPoint::new(1.0, 2.0), Some(&sample), None);

        assert_eq!(selected.world_x, 10.0);
        assert_eq!(selected.world_z, 20.0);
        assert_eq!(
            selected.point_kind,
            Some(FishyMapSelectionPointKind::Waypoint)
        );
        assert_eq!(selected.point_label.as_deref(), Some("Chunsu"));
        assert_eq!(selected.layer_samples.len(), 1);
    }

    #[test]
    fn quick_selection_uses_matching_hover_details() {
        let world_point = WorldPoint::new(100.0, 200.0);
        let (map_px, map_py) = super::map_pixel_for_world_point(world_point);
        let hover = HoverInfo {
            map_px,
            map_py,
            world_x: world_point.x + 1.0,
            world_z: world_point.z + 1.0,
            layer_samples: vec![zone_sample(0x123456)],
            point_samples: Vec::new(),
        };

        let selected = quick_selected_info(world_point, None, Some(&hover));

        assert_eq!(
            selected.point_kind,
            Some(FishyMapSelectionPointKind::Clicked)
        );
        assert_eq!(selected.world_x, world_point.x);
        assert_eq!(selected.world_z, world_point.z);
        assert_eq!(selected.layer_samples.len(), 1);
    }

    #[test]
    fn quick_selection_falls_back_to_clicked_point_feedback() {
        let world_point = WorldPoint::new(100.0, 200.0);

        let selected = quick_selected_info(world_point, None, None);

        assert_eq!(
            selected.point_kind,
            Some(FishyMapSelectionPointKind::Clicked)
        );
        assert_eq!(selected.point_label.as_deref(), Some("Clicked point"));
        assert_eq!(selected.world_x, world_point.x);
        assert_eq!(selected.world_z, world_point.z);
        assert!(selected.layer_samples.is_empty());
    }
}
