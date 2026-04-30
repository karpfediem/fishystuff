use bevy::ecs::system::SystemParam;
use bevy::input::touch::Touches;
use bevy::input::ButtonInput;
use bevy::window::PrimaryWindow;

use crate::map::camera::map2d::map2d_cursor_to_world;
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::hover_query::{hover_info_at_world_point, WorldPointQueryContext};
use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::map::raster::RasterTileCache;
use crate::map::selection_query::selected_info_at_world_point;
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
use crate::prelude::*;

pub struct MaskPlugin;

impl Plugin for MaskPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExactLookupCache>()
            .add_systems(Update, (update_hover, handle_click).chain());
    }
}

fn hovered_zone_rgb(info: Option<&crate::plugins::api::HoverInfo>) -> Option<u32> {
    info.and_then(crate::plugins::api::HoverInfo::zone_rgb_u32)
}

fn set_hover_state(
    hover: &mut HoverState,
    display_state: &mut MapDisplayState,
    next_info: Option<crate::plugins::api::HoverInfo>,
) {
    if hover.info != next_info {
        hover.info = next_info.clone();
    }
    let next_hovered_zone_rgb = hovered_zone_rgb(next_info.as_ref());
    if display_state.hovered_zone_rgb != next_hovered_zone_rgb {
        display_state.hovered_zone_rgb = next_hovered_zone_rgb;
    }
}

fn clear_hover_state(hover: &mut HoverState, display_state: &mut MapDisplayState) {
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
        &context.display_state,
        &context.point_camera_q,
    );
    let Some(mut next_hover) = hover_info_at_world_point(
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
    .or_else(|| point_hover_info(world_point, point_samples.clone())) else {
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
    let point_samples = point_samples_at_world_point(
        world_point,
        &context.points,
        &context.display_state,
        &context.point_camera_q,
    );
    let Some(mut selected_info) = selected_info_at_world_point(
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
        crate::bridge::contract::FishyMapSelectionPointKind::Clicked,
        None,
        Some(&context.bootstrap.zones),
    )
    .or_else(|| point_selected_info(world_point, point_samples.clone())) else {
        return;
    };
    if !point_samples.is_empty() {
        selected_info.point_samples = point_samples;
    }
    let zone_rgb = selected_info.zone_rgb();
    let zone_rgb_u32 = selected_info.zone_rgb_u32();
    context.selection.info = Some(selected_info);
    context.selection.zone_stats = None;
    context.pending.zone_stats = None;

    let Some(rgb) = zone_rgb else {
        context.selection.zone_stats_status = "zone stats: unavailable".to_string();
        return;
    };
    context.selection.zone_stats_status = "zone stats: loading".to_string();

    let Some(request) = build_zone_stats_request(&context.bootstrap, &context.patch_filter, rgb)
    else {
        context.selection.zone_stats_status = "zone stats: missing defaults".to_string();
        return;
    };

    let Some(rgb_u32) = zone_rgb_u32 else {
        context.selection.zone_stats_status = "zone stats: unavailable".to_string();
        return;
    };
    let receiver = spawn_zone_stats_request(request);
    context.pending.zone_stats = Some((rgb_u32, receiver));
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

fn point_selected_info(
    world_point: WorldPoint,
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
        point_kind: Some(crate::bridge::contract::FishyMapSelectionPointKind::Clicked),
        point_label: Some("Ranking Samples".to_string()),
        layer_samples: Vec::new(),
        point_samples,
    })
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
        || active_touch_count >= 2
        || (active_touch_count == 1 && context.pan.drag_distance > DRAG_THRESHOLD)
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
    layer_registry: Res<'w, LayerRegistry>,
    layer_runtime: Res<'w, LayerRuntime>,
    vector_runtime: Res<'w, VectorLayerRuntime>,
    layer_filters: Res<'w, LayerEffectiveFilterState>,
    point_camera_q: Query<'w, 's, &'static Projection, With<Map2dCamera>>,
}

#[derive(SystemParam)]
struct MaskClickContext<'w, 's> {
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    touches: Res<'w, Touches>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    camera_q: Query<'w, 's, (&'static Projection, &'static Transform), With<Map2dCamera>>,
    exact_lookups: Res<'w, ExactLookupCache>,
    field_metadata: Res<'w, FieldMetadataCache>,
    tile_cache: Res<'w, RasterTileCache>,
    layer_registry: Res<'w, LayerRegistry>,
    layer_runtime: Res<'w, LayerRuntime>,
    vector_runtime: Res<'w, VectorLayerRuntime>,
    layer_filters: Res<'w, LayerEffectiveFilterState>,
    pending: ResMut<'w, PendingRequests>,
    selection: ResMut<'w, SelectionState>,
    points: Res<'w, PointsState>,
    display_state: Res<'w, MapDisplayState>,
    pan: Res<'w, PanState>,
    bootstrap: Res<'w, ApiBootstrapState>,
    patch_filter: Res<'w, PatchFilterState>,
    ui_capture: Res<'w, UiPointerCapture>,
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
    use super::hovered_zone_rgb;
    use crate::plugins::api::HoverInfo;

    #[test]
    fn hovered_zone_rgb_reads_zone_from_hover_info() {
        let info = HoverInfo {
            map_px: 12,
            map_py: 34,
            world_x: 1.0,
            world_z: 2.0,
            layer_samples: vec![crate::map::layer_query::LayerQuerySample {
                layer_id: "zone_mask".to_string(),
                layer_name: "Zone Mask".to_string(),
                kind: "field".to_string(),
                rgb: fishystuff_api::Rgb::from_u32(0x123456),
                rgb_u32: 0x123456,
                field_id: Some(0x123456),
                targets: Vec::new(),
                detail_pane: None,
                detail_sections: Vec::new(),
            }],
            point_samples: Vec::new(),
        };
        assert_eq!(hovered_zone_rgb(Some(&info)), Some(0x123456));
        assert_eq!(hovered_zone_rgb(None), None);
    }
}
