use bevy::ecs::system::SystemParam;
use bevy::input::touch::Touches;
use bevy::input::ButtonInput;
use bevy::window::PrimaryWindow;

use fishystuff_core::field_metadata::{FieldHoverRow, FIELD_HOVER_ROW_KEY_ZONE};

use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::layer_query::{
    sample_layers_at_world_point, LayerQuerySample, LayerSamplingContext,
};
use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::map::raster::RasterTileCache;
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::WorldPoint;
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
    if context.view_mode.mode != ViewMode::Map2D {
        clear_hover_state(&mut context.hover, &mut context.display_state);
        return;
    }
    let active_touch_count = context.touches.iter().count();
    if context.mouse_buttons.pressed(MouseButton::Left)
        || active_touch_count >= 2
        || (active_touch_count == 1 && context.pan.drag_distance > DRAG_THRESHOLD)
    {
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
    let Ok(window) = context.windows.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = context.camera_q.single() else {
        return;
    };
    let Some(cursor) = window
        .cursor_position()
        .or_else(|| touch_hover_position(&context.touches))
    else {
        clear_hover_state(&mut context.hover, &mut context.display_state);
        return;
    };
    let Some(world) = camera
        .viewport_to_world_2d(&GlobalTransform::from(*camera_transform), cursor)
        .ok()
    else {
        clear_hover_state(&mut context.hover, &mut context.display_state);
        return;
    };
    let map_to_world = MapToWorld::default();
    let map = map_to_world.world_to_map(WorldPoint::new(world.x as f64, world.y as f64));
    let map_x = map.x as f32;
    let map_y = map.y as f32;

    if map_x < 0.0
        || map_y < 0.0
        || map_x >= map_to_world.image_size_x as f32
        || map_y >= map_to_world.image_size_y as f32
    {
        clear_hover_state(&mut context.hover, &mut context.display_state);
        return;
    }

    let map_px = map_x.floor() as i32;
    let map_py = map_y.floor() as i32;
    let world_point = WorldPoint::new(world.x as f64, world.y as f64);
    let hover_layers = current_hover_layers(&context.layer_registry, &context.layer_runtime);
    let sampling = LayerSamplingContext {
        exact_lookups: &context.exact_lookups,
        field_metadata: &context.field_metadata,
        tile_cache: &context.tile_cache,
        vector_runtime: &context.vector_runtime,
        world_point,
        map_to_world,
        map_version_id: context.layer_registry.map_version_id(),
    };
    let layer_samples = sample_layers_at_world_point(&hover_layers, &sampling);
    let zone_sample = zone_mask_hover_sample(&layer_samples);
    let zone_name = zone_sample.and_then(|sample| zone_name_from_hover_rows(&sample.rows));
    let zone_rgb = zone_sample.as_ref().map(|sample| sample.rgb);
    let zone_rgb_u32 = zone_sample.as_ref().map(|sample| sample.rgb_u32);
    let next_hover = crate::plugins::api::HoverInfo {
        map_px,
        map_py,
        rgb: zone_rgb,
        rgb_u32: zone_rgb_u32,
        zone_name,
        world_x: world_point.x,
        world_z: world_point.z,
        layer_samples,
    };
    set_hover_state(
        &mut context.hover,
        &mut context.display_state,
        Some(next_hover),
    );
}

fn handle_click(mut context: MaskClickContext<'_, '_>) {
    if context.view_mode.mode != ViewMode::Map2D {
        return;
    }
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
    let Some(hover) = context.hover.info.clone() else {
        return;
    };
    let (Some(rgb), Some(rgb_u32)) = (hover.rgb, hover.rgb_u32) else {
        return;
    };
    context.selection.info = Some(crate::plugins::api::SelectedInfo {
        map_px: hover.map_px,
        map_py: hover.map_py,
        rgb,
        rgb_u32,
        zone_name: hover.zone_name.clone(),
        world_x: hover.world_x,
        world_z: hover.world_z,
    });
    context.selection.zone_stats = None;
    context.selection.zone_stats_status = "zone stats: loading".to_string();

    let Some(request) = build_zone_stats_request(&context.bootstrap, &context.patch_filter, rgb)
    else {
        context.selection.zone_stats_status = "zone stats: missing defaults".to_string();
        return;
    };

    let receiver = spawn_zone_stats_request(request);
    context.pending.zone_stats = Some((rgb_u32, receiver));
}

fn current_hover_layers<'a>(
    layer_registry: &'a LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Vec<&'a crate::map::layers::LayerSpec> {
    let mut layers = layer_registry
        .ordered()
        .iter()
        .filter(|layer| layer.key != "minimap" && layer_runtime.visible(layer.id))
        .collect::<Vec<_>>();
    layers.sort_by(|lhs, rhs| {
        layer_runtime
            .get(rhs.id)
            .map(|state| state.display_order)
            .unwrap_or(rhs.display_order)
            .cmp(
                &layer_runtime
                    .get(lhs.id)
                    .map(|state| state.display_order)
                    .unwrap_or(lhs.display_order),
            )
            .then_with(|| rhs.display_order.cmp(&lhs.display_order))
            .then_with(|| lhs.key.cmp(&rhs.key))
    });
    layers
}

#[derive(SystemParam)]
struct HoverUpdateContext<'w, 's> {
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
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
    layer_registry: Res<'w, LayerRegistry>,
    layer_runtime: Res<'w, LayerRuntime>,
    vector_runtime: Res<'w, VectorLayerRuntime>,
    view_mode: Res<'w, ViewModeState>,
}

#[derive(SystemParam)]
struct MaskClickContext<'w, 's> {
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
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

fn zone_mask_hover_sample(layer_samples: &[LayerQuerySample]) -> Option<&LayerQuerySample> {
    layer_samples
        .iter()
        .find(|sample| sample.layer_id == "zone_mask")
}

fn zone_name_from_hover_rows(rows: &[FieldHoverRow]) -> Option<String> {
    rows.iter()
        .find(|row| row.key == FIELD_HOVER_ROW_KEY_ZONE)
        .map(|row| row.value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::{hovered_zone_rgb, zone_mask_hover_sample, zone_name_from_hover_rows};
    use crate::map::layer_query::LayerQuerySample;
    use crate::plugins::api::HoverInfo;
    use fishystuff_api::Rgb;
    use fishystuff_core::field_metadata::{FieldHoverRow, FIELD_HOVER_ROW_KEY_ZONE};

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

    #[test]
    fn zone_name_from_hover_rows_reads_zone_row() {
        let rows = vec![FieldHoverRow {
            key: FIELD_HOVER_ROW_KEY_ZONE.to_string(),
            icon: "hover-zone".to_string(),
            label: "Zone".to_string(),
            value: "Olvia Coast".to_string(),
            hide_label: false,
            status_icon: None,
            status_icon_tone: None,
        }];
        assert_eq!(
            zone_name_from_hover_rows(&rows),
            Some("Olvia Coast".to_string())
        );
    }

    #[test]
    fn zone_mask_hover_sample_prefers_zone_mask_layer_id() {
        let samples = vec![
            LayerQuerySample {
                layer_id: "regions".to_string(),
                layer_name: "Regions".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x112233),
                rgb_u32: 0x112233,
                field_id: Some(88),
                rows: Vec::new(),
                targets: Vec::new(),
            },
            LayerQuerySample {
                layer_id: "zone_mask".to_string(),
                layer_name: "Zone Mask".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x445566),
                rgb_u32: 0x445566,
                field_id: Some(0x445566),
                rows: Vec::new(),
                targets: Vec::new(),
            },
        ];
        assert_eq!(
            zone_mask_hover_sample(&samples).map(|sample| sample.rgb_u32),
            Some(0x445566)
        );
    }
}
