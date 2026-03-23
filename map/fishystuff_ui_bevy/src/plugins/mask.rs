use bevy::ecs::system::SystemParam;
use bevy::input::touch::Touches;
use bevy::input::ButtonInput;
use bevy::window::PrimaryWindow;
use serde_json::{Map, Value};

use fishystuff_api::Rgb;
use fishystuff_core::field_metadata::FieldHoverMetadataEntry;

use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::exact_lookup::{
    sample_exact_lookup_rgb, sample_field_layer_id_u32, sample_field_layer_rgb, ExactLookupCache,
};
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::layers::{LayerRegistry, LayerRuntime, PickMode};
use crate::map::raster::{map_version_id, RasterTileCache, TileKey};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::WorldPoint;
use crate::plugins::api::{
    build_zone_stats_request, spawn_zone_stats_request, ApiBootstrapState, HoverLayerSample,
    HoverState, MapDisplayState, PatchFilterState, PendingRequests, SelectionState,
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
    let mut sampling = HoverSamplingContext {
        exact_lookups: &context.exact_lookups,
        field_metadata: &context.field_metadata,
        tile_cache: &context.tile_cache,
        vector_runtime: &context.vector_runtime,
        bootstrap: &context.bootstrap,
        world_point,
        map_px: (map_px, map_py),
        map_to_world,
        registry_map_version_id: context.layer_registry.map_version_id(),
    };
    let layer_samples = collect_hover_layer_samples(&hover_layers, &mut sampling);
    let zone_sample = hover_layers
        .iter()
        .find(|layer| layer.pick_mode == PickMode::ExactTilePixel)
        .and_then(|layer| {
            layer_samples
                .iter()
                .find(|sample| sample.layer_id == layer.key)
                .cloned()
        });

    let zone_name = zone_sample.as_ref().and_then(|sample| {
        context
            .bootstrap
            .zones
            .get(&sample.rgb_u32)
            .cloned()
            .unwrap_or(None)
    });
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

fn collect_hover_layer_samples(
    hover_layers: &[&crate::map::layers::LayerSpec],
    sampling: &mut HoverSamplingContext<'_>,
) -> Vec<HoverLayerSample> {
    hover_layers
        .iter()
        .filter_map(|layer| sample_hover_layer(layer, sampling))
        .collect()
}

fn sample_hover_layer(
    layer: &crate::map::layers::LayerSpec,
    sampling: &mut HoverSamplingContext<'_>,
) -> Option<HoverLayerSample> {
    let (rgb, hover_metadata, kind) = if layer.field_url().is_some() {
        (
            sample_field_layer_rgb(
                layer,
                sampling.exact_lookups,
                sampling.map_px.0,
                sampling.map_px.1,
            )?,
            sample_field_layer_hover_metadata(
                layer,
                sampling.exact_lookups,
                sampling.field_metadata,
                sampling.map_px,
            ),
            "field".to_string(),
        )
    } else if layer.is_raster() {
        (
            sample_raster_layer_rgb(
                layer,
                sampling.exact_lookups,
                sampling.tile_cache,
                sampling.bootstrap,
                sampling.world_point,
                sampling.map_px,
                sampling.map_to_world,
            )?,
            HoverVectorMetadata::default(),
            "tiled-raster".to_string(),
        )
    } else if layer.is_vector() {
        (
            sample_vector_layer_rgb(
                layer,
                sampling.vector_runtime,
                sampling.registry_map_version_id,
                sampling.world_point,
            )?,
            sample_vector_layer_hover_metadata(
                layer,
                sampling.vector_runtime,
                sampling.registry_map_version_id,
                sampling.world_point,
            ),
            "vector-geojson".to_string(),
        )
    } else {
        return None;
    };
    let rgb_u32 = rgb.to_u32();
    Some(HoverLayerSample {
        layer_id: layer.key.clone(),
        layer_name: layer.name.clone(),
        kind,
        rgb,
        rgb_u32,
        region_id: hover_metadata.region_id,
        region_group: hover_metadata.region_group,
        region_name: hover_metadata.region_name,
        resource_bar_waypoint: hover_metadata.resource_bar_waypoint,
        resource_bar_world_x: hover_metadata.resource_bar_world_x,
        resource_bar_world_z: hover_metadata.resource_bar_world_z,
        origin_waypoint: hover_metadata.origin_waypoint,
        origin_world_x: hover_metadata.origin_world_x,
        origin_world_z: hover_metadata.origin_world_z,
    })
}

#[derive(Debug, Clone, Default, PartialEq)]
struct HoverVectorMetadata {
    region_id: Option<u32>,
    region_group: Option<u32>,
    region_name: Option<String>,
    resource_bar_waypoint: Option<u32>,
    resource_bar_world_x: Option<f64>,
    resource_bar_world_z: Option<f64>,
    origin_waypoint: Option<u32>,
    origin_world_x: Option<f64>,
    origin_world_z: Option<f64>,
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
    bootstrap: Res<'w, ApiBootstrapState>,
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

struct HoverSamplingContext<'a> {
    exact_lookups: &'a ExactLookupCache,
    field_metadata: &'a FieldMetadataCache,
    tile_cache: &'a RasterTileCache,
    vector_runtime: &'a VectorLayerRuntime,
    bootstrap: &'a ApiBootstrapState,
    world_point: WorldPoint,
    map_px: (i32, i32),
    map_to_world: MapToWorld,
    registry_map_version_id: Option<&'a str>,
}

fn sample_field_layer_hover_metadata(
    layer: &crate::map::layers::LayerSpec,
    exact_lookups: &ExactLookupCache,
    field_metadata: &FieldMetadataCache,
    map_px: (i32, i32),
) -> HoverVectorMetadata {
    let Some(metadata_url) = layer.field_metadata_url() else {
        return HoverVectorMetadata::default();
    };
    let Some(field_id) = sample_field_layer_id_u32(layer, exact_lookups, map_px.0, map_px.1) else {
        return HoverVectorMetadata::default();
    };
    let Some(entry) = field_metadata.entry(layer.id, &metadata_url, field_id) else {
        return HoverVectorMetadata::default();
    };
    hover_metadata_from_field_entry(entry)
}

fn hover_metadata_from_field_entry(entry: &FieldHoverMetadataEntry) -> HoverVectorMetadata {
    HoverVectorMetadata {
        region_id: entry.region_id,
        region_group: entry.region_group,
        region_name: entry.region_name.clone(),
        resource_bar_waypoint: entry.resource_bar_waypoint,
        resource_bar_world_x: entry.resource_bar_world_x,
        resource_bar_world_z: entry.resource_bar_world_z,
        origin_waypoint: entry.origin_waypoint,
        origin_world_x: entry.origin_world_x,
        origin_world_z: entry.origin_world_z,
    }
}

#[cfg(test)]
mod tests {
    use super::{hover_metadata_from_properties, hovered_zone_rgb, HoverVectorMetadata};
    use crate::plugins::api::HoverInfo;
    use serde_json::{Map, Value};

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
    fn hover_metadata_extracts_region_details_for_detailed_region_layer() {
        let mut properties = Map::new();
        properties.insert("r".to_string(), Value::from(76u32));
        properties.insert("rg".to_string(), Value::from(118u32));
        properties.insert(
            "on".to_string(),
            Value::String("Solgaji Forest".to_string()),
        );
        properties.insert("rgwp".to_string(), Value::from(1785u32));
        properties.insert("rgx".to_string(), Value::from(-1314259.375f64));
        properties.insert("rgz".to_string(), Value::from(1142209.625f64));
        properties.insert("owp".to_string(), Value::from(1437u32));
        properties.insert("ox".to_string(), Value::from(98484.74786281586f64));
        properties.insert("oz".to_string(), Value::from(365929.37886714935f64));
        let metadata = hover_metadata_from_properties("regions", &properties);
        assert_eq!(
            metadata,
            HoverVectorMetadata {
                region_id: Some(76),
                region_group: Some(118),
                region_name: Some("Solgaji Forest".to_string()),
                resource_bar_waypoint: Some(1785),
                resource_bar_world_x: Some(-1314259.375),
                resource_bar_world_z: Some(1142209.625),
                origin_waypoint: Some(1437),
                origin_world_x: Some(98484.74786281586),
                origin_world_z: Some(365929.37886714935),
            }
        );
    }

    #[test]
    fn hover_metadata_ignores_origin_name_for_region_group_layer() {
        let mut properties = Map::new();
        properties.insert("rg".to_string(), Value::from(58u32));
        properties.insert("on".to_string(), Value::String("Tarif".to_string()));
        properties.insert("rgwp".to_string(), Value::from(306u32));
        let metadata = hover_metadata_from_properties("region_groups", &properties);
        assert_eq!(metadata.region_id, None);
        assert_eq!(metadata.region_group, Some(58));
        assert_eq!(metadata.region_name, None);
        assert_eq!(metadata.resource_bar_waypoint, Some(306));
    }
}

fn sample_raster_layer_rgb(
    layer: &crate::map::layers::LayerSpec,
    exact_lookups: &ExactLookupCache,
    tile_cache: &RasterTileCache,
    bootstrap: &ApiBootstrapState,
    world_point: WorldPoint,
    map_px: (i32, i32),
    map_to_world: MapToWorld,
) -> Option<Rgb> {
    if layer.field_url().is_some() {
        return sample_field_layer_rgb(layer, exact_lookups, map_px.0, map_px.1);
    }

    if layer.pick_mode == PickMode::ExactTilePixel {
        return sample_exact_lookup_rgb(layer, exact_lookups, map_px.0, map_px.1);
    }

    let world_transform = layer.world_transform(map_to_world)?;
    let layer_px = world_transform.world_to_layer(world_point);
    if layer_px.x < 0.0 || layer_px.y < 0.0 {
        return None;
    }
    let map_version = if layer.tile_url_template.contains("{map_version}") {
        bootstrap.map_version.as_deref()
    } else {
        None
    };
    if layer.tile_url_template.contains("{map_version}") && map_version.is_none() {
        return None;
    }
    let tile_px = layer.tile_px.max(1);
    let layer_ix = layer_px.x.floor() as u32;
    let layer_iy = layer_px.y.floor() as u32;
    let tx = layer_ix / tile_px;
    let ty = layer_iy / tile_px;
    let key = TileKey {
        layer: layer.id,
        map_version: map_version.map(map_version_id).unwrap_or(0),
        z: 0,
        tx: tx as i32,
        ty: ty as i32,
    };
    let tile = tile_cache.get_ready_pixel_data(&key)?;
    let local_x = layer_ix - tx * tile_px;
    let local_y = layer_iy - ty * tile_px;
    if local_x >= tile.width || local_y >= tile.height {
        return None;
    }
    let idx = ((local_y * tile.width + local_x) * 4) as usize;
    if idx + 3 >= tile.data.len() || tile.data[idx + 3] == 0 {
        return None;
    }
    Some(Rgb::new(
        tile.data[idx],
        tile.data[idx + 1],
        tile.data[idx + 2],
    ))
}

fn sample_vector_layer_rgb(
    layer: &crate::map::layers::LayerSpec,
    vector_runtime: &VectorLayerRuntime,
    registry_map_version_id: Option<&str>,
    world_point: WorldPoint,
) -> Option<Rgb> {
    let source = layer.vector_source.as_ref()?;
    let revision = resolved_vector_revision(source, registry_map_version_id);
    let bundle = vector_runtime.finished.get_ref(&(layer.id, revision))?;
    let rgba = bundle.sample_rgb(world_point.x as f32, world_point.z as f32)?;
    Some(Rgb::new(rgba[0], rgba[1], rgba[2]))
}

fn sample_vector_layer_hover_metadata(
    layer: &crate::map::layers::LayerSpec,
    vector_runtime: &VectorLayerRuntime,
    registry_map_version_id: Option<&str>,
    world_point: WorldPoint,
) -> HoverVectorMetadata {
    if layer.key != "region_groups" && layer.key != "regions" {
        return HoverVectorMetadata::default();
    }
    let Some(source) = layer.vector_source.as_ref() else {
        return HoverVectorMetadata::default();
    };
    let revision = resolved_vector_revision(source, registry_map_version_id);
    let Some(bundle) = vector_runtime.finished.get_ref(&(layer.id, revision)) else {
        return HoverVectorMetadata::default();
    };
    let Some(properties) = bundle.sample_properties(world_point.x as f32, world_point.z as f32)
    else {
        return HoverVectorMetadata::default();
    };
    hover_metadata_from_properties(&layer.key, properties)
}

fn hover_metadata_from_properties(
    layer_key: &str,
    properties: &Map<String, Value>,
) -> HoverVectorMetadata {
    HoverVectorMetadata {
        region_id: json_u32(properties.get("r")),
        region_group: json_u32(properties.get("rg")),
        region_name: if layer_key == "regions" {
            json_string(properties.get("on"))
        } else {
            None
        },
        resource_bar_waypoint: json_u32(properties.get("rgwp")),
        resource_bar_world_x: json_f64(properties.get("rgx")),
        resource_bar_world_z: json_f64(properties.get("rgz")),
        origin_waypoint: json_u32(properties.get("owp")),
        origin_world_x: json_f64(properties.get("ox")),
        origin_world_z: json_f64(properties.get("oz")),
    }
}

fn json_u32(value: Option<&Value>) -> Option<u32> {
    match value {
        Some(Value::Number(number)) => number.as_u64().and_then(|raw| u32::try_from(raw).ok()),
        Some(Value::String(text)) => text.trim().parse::<u32>().ok(),
        _ => None,
    }
}

fn json_string(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(text)) => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        _ => None,
    }
}

fn json_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(number)) => number.as_f64(),
        Some(Value::String(text)) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn resolved_vector_revision(
    source: &crate::map::layers::VectorSourceSpec,
    map_version_id: Option<&str>,
) -> String {
    let mut url = source.url.clone();
    if url.contains("{map_version}") {
        let version = map_version_id
            .filter(|value| !value.trim().is_empty() && *value != "0v0")
            .unwrap_or("v1");
        url = url.replace("{map_version}", version);
    }
    let revision = source.revision.trim();
    if revision.is_empty() {
        format!("url:{url}")
    } else {
        revision.to_string()
    }
}
