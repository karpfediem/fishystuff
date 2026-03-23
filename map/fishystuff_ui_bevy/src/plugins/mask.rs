use bevy::ecs::system::SystemParam;
use bevy::input::touch::Touches;
use bevy::input::ButtonInput;
use bevy::window::PrimaryWindow;

use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::hover_query::{hover_info_at_world_point, WorldPointQueryContext};
use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::map::raster::RasterTileCache;
use crate::map::selection_query::selected_info_from_hover;
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::WorldPoint;
use crate::map::terrain::runtime::TerrainViewEstimate;
use crate::plugins::api::{
    build_zone_stats_request, spawn_zone_stats_request, ApiBootstrapState, HoverState,
    MapDisplayState, PatchFilterState, PendingRequests, SelectionState,
};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::input::PanState;
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
    info.and_then(|hover| hover.rgb_u32)
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
    if !matches!(
        context.view_mode.mode,
        ViewMode::Map2D | ViewMode::Terrain3D
    ) {
        clear_hover_state(&mut context.hover, &mut context.display_state);
        return;
    }
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
    let Some(next_hover) = hover_info_at_world_point(
        world_point,
        &WorldPointQueryContext {
            layer_registry: &context.layer_registry,
            layer_runtime: &context.layer_runtime,
            exact_lookups: &context.exact_lookups,
            field_metadata: &context.field_metadata,
            tile_cache: &context.tile_cache,
            vector_runtime: &context.vector_runtime,
            map_to_world: MapToWorld::default(),
        },
    ) else {
        clear_hover_state(&mut context.hover, &mut context.display_state);
        return;
    };
    set_hover_state(
        &mut context.hover,
        &mut context.display_state,
        Some(next_hover),
    );
}

fn handle_click(mut context: MaskClickContext<'_, '_>) {
    if !matches!(
        context.view_mode.mode,
        ViewMode::Map2D | ViewMode::Terrain3D
    ) {
        return;
    }
    if context.ui_capture.blocked {
        return;
    }
    if context.view_mode.mode == ViewMode::Terrain3D
        && (context.key_buttons.pressed(KeyCode::AltLeft)
            || context.key_buttons.pressed(KeyCode::AltRight))
    {
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
    let Some(hover) = context.hover.info.clone() else {
        return;
    };
    let Some(selected_info) = selected_info_from_hover(&hover) else {
        return;
    };
    let zone_rgb = selected_info.rgb;
    let zone_rgb_u32 = selected_info.rgb_u32;
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
    match context.view_mode.mode {
        ViewMode::Map2D => {
            let Ok(window) = context.windows.single() else {
                return None;
            };
            let Ok((camera, camera_transform)) = context.camera_q.single() else {
                return None;
            };
            let cursor = window
                .cursor_position()
                .or_else(|| touch_hover_position(&context.touches))?;
            let world = camera
                .viewport_to_world_2d(&GlobalTransform::from(*camera_transform), cursor)
                .ok()?;
            Some(WorldPoint::new(world.x as f64, world.y as f64))
        }
        ViewMode::Terrain3D => context.terrain_view.cursor_world,
    }
}

fn hover_interaction_blocked(context: &HoverUpdateContext<'_, '_>) -> bool {
    match context.view_mode.mode {
        ViewMode::Map2D => {
            let active_touch_count = context.touches.iter().count();
            context.mouse_buttons.pressed(MouseButton::Left)
                || active_touch_count >= 2
                || (active_touch_count == 1 && context.pan.drag_distance > DRAG_THRESHOLD)
        }
        ViewMode::Terrain3D => {
            context.mouse_buttons.pressed(MouseButton::Middle)
                || ((context.key_buttons.pressed(KeyCode::AltLeft)
                    || context.key_buttons.pressed(KeyCode::AltRight))
                    && context.mouse_buttons.pressed(MouseButton::Left))
        }
    }
}

#[derive(SystemParam)]
struct HoverUpdateContext<'w, 's> {
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    key_buttons: Res<'w, ButtonInput<KeyCode>>,
    touches: Res<'w, Touches>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    camera_q: Query<'w, 's, (&'static Camera, &'static Transform), With<Map2dCamera>>,
    exact_lookups: Res<'w, ExactLookupCache>,
    field_metadata: Res<'w, FieldMetadataCache>,
    tile_cache: Res<'w, RasterTileCache>,
    display_state: ResMut<'w, MapDisplayState>,
    ui_capture: Res<'w, UiPointerCapture>,
    hover: ResMut<'w, HoverState>,
    pan: Res<'w, PanState>,
    terrain_view: Res<'w, TerrainViewEstimate>,
    layer_registry: Res<'w, LayerRegistry>,
    layer_runtime: Res<'w, LayerRuntime>,
    vector_runtime: Res<'w, VectorLayerRuntime>,
    view_mode: Res<'w, ViewModeState>,
}

#[derive(SystemParam)]
struct MaskClickContext<'w, 's> {
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    key_buttons: Res<'w, ButtonInput<KeyCode>>,
    touches: Res<'w, Touches>,
    pending: ResMut<'w, PendingRequests>,
    selection: ResMut<'w, SelectionState>,
    hover: Res<'w, HoverState>,
    pan: Res<'w, PanState>,
    bootstrap: Res<'w, ApiBootstrapState>,
    patch_filter: Res<'w, PatchFilterState>,
    ui_capture: Res<'w, UiPointerCapture>,
    view_mode: Res<'w, ViewModeState>,
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
            rgb: None,
            rgb_u32: Some(0x123456),
            zone_name: Some("Test Zone".to_string()),
            world_x: 1.0,
            world_z: 2.0,
            layer_samples: Vec::new(),
        };
        assert_eq!(hovered_zone_rgb(Some(&info)), Some(0x123456));
        assert_eq!(hovered_zone_rgb(None), None);
    }
}
