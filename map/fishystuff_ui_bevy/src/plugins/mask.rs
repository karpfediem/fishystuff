use bevy::input::ButtonInput;
use bevy::window::PrimaryWindow;

use fishystuff_api::Rgb;

use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::layers::{LayerRegistry, LayerRuntime, PickMode};
use crate::map::raster::{map_version_id, queue_pick_probe_request, RasterTileCache, TileKey};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::WorldPoint;
use crate::map::streaming::TileStreamer;
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
        app.add_systems(Update, (update_hover, handle_click));
    }
}

fn update_hover(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &Transform), With<Map2dCamera>>,
    tile_cache: Res<RasterTileCache>,
    mut streamer: ResMut<TileStreamer>,
    bootstrap: Res<ApiBootstrapState>,
    mut display_state: ResMut<MapDisplayState>,
    ui_capture: Res<UiPointerCapture>,
    mut hover: ResMut<HoverState>,
    layer_registry: Res<LayerRegistry>,
    layer_runtime: Res<LayerRuntime>,
    vector_runtime: Res<VectorLayerRuntime>,
    view_mode: Res<ViewModeState>,
) {
    display_state.hovered_zone_rgb = None;
    if view_mode.mode != ViewMode::Map2D {
        hover.info = None;
        return;
    }
    if ui_capture.blocked {
        hover.info = None;
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        hover.info = None;
        return;
    };
    let Some(world) = camera
        .viewport_to_world_2d(&GlobalTransform::from(*camera_transform), cursor)
        .ok()
    else {
        hover.info = None;
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
        hover.info = None;
        return;
    }

    let map_px = map_x.floor() as i32;
    let map_py = map_y.floor() as i32;
    let world_point = WorldPoint::new(world.x as f64, world.y as f64);
    let hover_layers = current_hover_layers(&layer_registry, &layer_runtime);
    let layer_samples = collect_hover_layer_samples(
        &hover_layers,
        &tile_cache,
        &mut streamer,
        &vector_runtime,
        &bootstrap,
        world_point,
        map_to_world,
        layer_registry.map_version_id(),
    );
    let zone_sample = hover_layers
        .iter()
        .find(|layer| layer.pick_mode == PickMode::ExactTilePixel)
        .and_then(|layer| {
            layer_samples
                .iter()
                .find(|sample| sample.layer_id == layer.key)
                .cloned()
        });

    if zone_sample.is_none() && layer_samples.is_empty() {
        hover.info = None;
        return;
    }

    let zone_name = zone_sample.as_ref().and_then(|sample| {
        bootstrap
            .zones
            .get(&sample.rgb_u32)
            .cloned()
            .unwrap_or(None)
    });
    let zone_rgb = zone_sample.as_ref().map(|sample| sample.rgb);
    let zone_rgb_u32 = zone_sample.as_ref().map(|sample| sample.rgb_u32);
    let world_at_center = map_to_world.map_to_world(crate::map::spaces::MapPoint::new(
        map_px as f64 + 0.5,
        map_py as f64 + 0.5,
    ));
    hover.info = Some(crate::plugins::api::HoverInfo {
        map_px,
        map_py,
        rgb: zone_rgb,
        rgb_u32: zone_rgb_u32,
        zone_name,
        world_x: world_at_center.x,
        world_z: world_at_center.z,
        layer_samples,
    });
    display_state.hovered_zone_rgb = zone_rgb_u32;
}

fn handle_click(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut pending: ResMut<PendingRequests>,
    mut selection: ResMut<SelectionState>,
    hover: Res<HoverState>,
    pan: Res<PanState>,
    bootstrap: Res<ApiBootstrapState>,
    patch_filter: Res<PatchFilterState>,
    ui_capture: Res<UiPointerCapture>,
    view_mode: Res<ViewModeState>,
) {
    if view_mode.mode != ViewMode::Map2D {
        return;
    }
    if ui_capture.blocked {
        return;
    }
    if !mouse_buttons.just_released(MouseButton::Left) {
        return;
    }
    if pan.drag_distance > DRAG_THRESHOLD {
        return;
    }
    let Some(hover) = hover.info.clone() else {
        return;
    };
    let (Some(rgb), Some(rgb_u32)) = (hover.rgb, hover.rgb_u32) else {
        return;
    };
    selection.info = Some(crate::plugins::api::SelectedInfo {
        map_px: hover.map_px,
        map_py: hover.map_py,
        rgb,
        rgb_u32,
        zone_name: hover.zone_name.clone(),
        world_x: hover.world_x,
        world_z: hover.world_z,
    });
    selection.zone_stats = None;
    selection.zone_stats_status = "zone stats: loading".to_string();

    let Some(request) = build_zone_stats_request(&bootstrap, &patch_filter, rgb) else {
        selection.zone_stats_status = "zone stats: missing defaults".to_string();
        return;
    };

    let receiver = spawn_zone_stats_request(request);
    pending.zone_stats = Some((rgb_u32, receiver));
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
    tile_cache: &RasterTileCache,
    streamer: &mut TileStreamer,
    vector_runtime: &VectorLayerRuntime,
    bootstrap: &ApiBootstrapState,
    world_point: WorldPoint,
    map_to_world: MapToWorld,
    registry_map_version_id: Option<&str>,
) -> Vec<HoverLayerSample> {
    hover_layers
        .iter()
        .filter_map(|layer| {
            sample_hover_layer(
                layer,
                tile_cache,
                streamer,
                vector_runtime,
                bootstrap,
                world_point,
                map_to_world,
                registry_map_version_id,
            )
        })
        .collect()
}

fn sample_hover_layer(
    layer: &crate::map::layers::LayerSpec,
    tile_cache: &RasterTileCache,
    streamer: &mut TileStreamer,
    vector_runtime: &VectorLayerRuntime,
    bootstrap: &ApiBootstrapState,
    world_point: WorldPoint,
    map_to_world: MapToWorld,
    registry_map_version_id: Option<&str>,
) -> Option<HoverLayerSample> {
    let rgb = if layer.is_raster() {
        sample_raster_layer_rgb(
            layer,
            tile_cache,
            streamer,
            bootstrap,
            world_point,
            map_to_world,
        )?
    } else if layer.is_vector() {
        sample_vector_layer_rgb(layer, vector_runtime, registry_map_version_id, world_point)?
    } else {
        return None;
    };
    let rgb_u32 = rgb.to_u32();
    Some(HoverLayerSample {
        layer_id: layer.key.clone(),
        layer_name: layer.name.clone(),
        kind: if layer.is_vector() {
            "vector-geojson".to_string()
        } else {
            "tiled-raster".to_string()
        },
        rgb,
        rgb_u32,
    })
}

fn sample_raster_layer_rgb(
    layer: &crate::map::layers::LayerSpec,
    tile_cache: &RasterTileCache,
    streamer: &mut TileStreamer,
    bootstrap: &ApiBootstrapState,
    world_point: WorldPoint,
    map_to_world: MapToWorld,
) -> Option<Rgb> {
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
    let Some(tile) = tile_cache.get_ready_pixel_data(&key) else {
        if !tile_cache.contains(&key) && !streamer.has_queued_key(&key) {
            queue_pick_probe_request(streamer, layer, key, map_version);
        }
        return None;
    };
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
