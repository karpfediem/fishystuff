use std::collections::{HashMap, HashSet};

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;

use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::exact_lookup::{
    ensure_exact_lookup_request, poll_exact_lookup_requests, ExactLookupCache, ExactLookupStatus,
    PendingExactLookups,
};
use crate::map::field_metadata::{
    ensure_field_metadata_request, poll_field_metadata_requests, FieldMetadataCache,
    PendingFieldMetadata,
};
use crate::map::layers::{
    FieldColorMode, LayerId, LayerManifestStatus, LayerRegistry, LayerRuntime,
};
use crate::map::spaces::layer_transform::LayerTransform;
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::{MapPoint, MapRect, WorldPoint, WorldRect};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};
use crate::prelude::*;

const FIELD_MAX_TEXTURE_EDGE: u32 = 2048;
const FIELD_OVERSCAN_SCREEN_PX: u32 = 128;
const FIELD_REUSE_SCALE_TOLERANCE: f64 = 0.2;

pub struct FieldLayersPlugin;

impl Plugin for FieldLayersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExactLookupCache>()
            .init_resource::<PendingExactLookups>()
            .init_resource::<FieldMetadataCache>()
            .init_resource::<PendingFieldMetadata>()
            .init_resource::<FieldLayerRuntime>()
            .add_systems(
                PostUpdate,
                (sync_exact_lookup_cache, update_field_layer_visuals).chain(),
            );
    }
}

#[derive(Resource, Default)]
struct FieldLayerRuntime {
    entries: HashMap<LayerId, FieldLayerEntry>,
}

#[derive(Debug)]
struct FieldLayerEntry {
    entity_2d: Entity,
    image: Handle<Image>,
    request: FieldTextureRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FieldTextureRequest {
    focus_origin_x: i32,
    focus_origin_y: i32,
    focus_width: u32,
    focus_height: u32,
    source_origin_x: i32,
    source_origin_y: i32,
    source_width: u32,
    source_height: u32,
    output_width: u16,
    output_height: u16,
}

impl FieldTextureRequest {
    fn can_reuse_for(self, next: Self) -> bool {
        self.output_width == next.output_width
            && self.output_height == next.output_height
            && rect_contains(
                self.source_origin_x,
                self.source_origin_y,
                self.source_width,
                self.source_height,
                next.focus_origin_x,
                next.focus_origin_y,
                next.focus_width,
                next.focus_height,
            )
            && scale_change_within_tolerance(self.focus_width, next.focus_width)
            && scale_change_within_tolerance(self.focus_height, next.focus_height)
    }
}

fn sync_exact_lookup_cache(
    layer_registry: Res<LayerRegistry>,
    mut layer_runtime: ResMut<LayerRuntime>,
    mut exact_lookups: ResMut<ExactLookupCache>,
    mut pending_lookups: ResMut<PendingExactLookups>,
    mut field_metadata: ResMut<FieldMetadataCache>,
    mut pending_field_metadata: ResMut<PendingFieldMetadata>,
) {
    poll_exact_lookup_requests(&mut exact_lookups, &mut pending_lookups);
    poll_field_metadata_requests(&mut field_metadata, &mut pending_field_metadata);

    let active_layer_ids = layer_registry
        .ordered()
        .iter()
        .filter(|layer| layer.field_url().is_some())
        .map(|layer| layer.id)
        .collect::<HashSet<_>>();

    for layer in layer_registry.ordered() {
        let Some(url) = layer.field_url() else {
            if layer.field_metadata_url().is_some() {
                ensure_field_metadata_request(
                    layer,
                    &mut field_metadata,
                    &mut pending_field_metadata,
                );
            }
            continue;
        };
        ensure_exact_lookup_request(layer, &mut exact_lookups, &mut pending_lookups);
        ensure_field_metadata_request(layer, &mut field_metadata, &mut pending_field_metadata);

        if let Some(runtime_state) = layer_runtime.get_mut(layer.id) {
            match exact_lookups.status(layer.id, &url) {
                ExactLookupStatus::Missing => {
                    runtime_state.manifest_status = LayerManifestStatus::Missing;
                    runtime_state.resident_tile_count = 0;
                    runtime_state.pending_count = 0;
                    runtime_state.inflight_count = 0;
                }
                ExactLookupStatus::Loading => {
                    runtime_state.manifest_status = LayerManifestStatus::Loading;
                    runtime_state.resident_tile_count = 0;
                    runtime_state.pending_count = 1;
                    runtime_state.inflight_count = 1;
                }
                ExactLookupStatus::Ready => {
                    runtime_state.manifest_status = LayerManifestStatus::Ready;
                    runtime_state.resident_tile_count = 1;
                    runtime_state.pending_count = 0;
                    runtime_state.inflight_count = 0;
                }
                ExactLookupStatus::Failed => {
                    runtime_state.manifest_status = LayerManifestStatus::Failed;
                    runtime_state.resident_tile_count = 0;
                    runtime_state.pending_count = 0;
                    runtime_state.inflight_count = 0;
                }
            }
        }
    }

    let stale_lookup_ids = exact_lookups
        .layer_ids()
        .into_iter()
        .filter(|layer_id| !active_layer_ids.contains(layer_id))
        .collect::<Vec<_>>();
    for layer_id in stale_lookup_ids {
        exact_lookups.remove_layer(layer_id);
    }

    let stale_pending_ids = pending_lookups
        .layer_ids()
        .into_iter()
        .filter(|layer_id| !active_layer_ids.contains(layer_id))
        .collect::<Vec<_>>();
    for layer_id in stale_pending_ids {
        pending_lookups.remove_layer(layer_id);
    }

    let active_metadata_ids = layer_registry
        .ordered()
        .iter()
        .filter(|layer| layer.field_metadata_url().is_some())
        .map(|layer| layer.id)
        .collect::<HashSet<_>>();
    let stale_metadata_ids = field_metadata
        .layer_ids()
        .into_iter()
        .filter(|layer_id| !active_metadata_ids.contains(layer_id))
        .collect::<Vec<_>>();
    for layer_id in stale_metadata_ids {
        field_metadata.remove_layer(layer_id);
    }
    let stale_pending_metadata_ids = pending_field_metadata
        .layer_ids()
        .into_iter()
        .filter(|layer_id| !active_metadata_ids.contains(layer_id))
        .collect::<Vec<_>>();
    for layer_id in stale_pending_metadata_ids {
        pending_field_metadata.remove_layer(layer_id);
    }
}

fn update_field_layer_visuals(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &Transform), With<Map2dCamera>>,
    view_mode: Res<ViewModeState>,
    layer_registry: Res<LayerRegistry>,
    layer_runtime: Res<LayerRuntime>,
    exact_lookups: Res<ExactLookupCache>,
    mut field_runtime: ResMut<FieldLayerRuntime>,
) {
    let active_exact_layers = layer_registry
        .ordered()
        .iter()
        .filter(|layer| layer.field_url().is_some())
        .map(|layer| layer.id)
        .collect::<HashSet<_>>();
    cleanup_stale_field_layers(
        &mut commands,
        &mut images,
        &mut field_runtime,
        &active_exact_layers,
    );

    if view_mode.mode != ViewMode::Map2D {
        hide_all_field_layers(&mut commands, &field_runtime);
        return;
    }

    let Ok(window) = windows.single() else {
        hide_all_field_layers(&mut commands, &field_runtime);
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.single() else {
        hide_all_field_layers(&mut commands, &field_runtime);
        return;
    };
    let Some(view_world) = view_rect(camera, camera_transform, window) else {
        hide_all_field_layers(&mut commands, &field_runtime);
        return;
    };
    let map_to_world = MapToWorld::default();
    let visible_map_rect = world_rect_to_clamped_map_rect(view_world, map_to_world);
    let Some((focus_output_width, focus_output_height)) =
        bounded_focus_output_size(window.width(), window.height())
    else {
        hide_all_field_layers(&mut commands, &field_runtime);
        return;
    };

    for layer in layer_registry.ordered() {
        let Some(url) = layer.field_url() else {
            continue;
        };
        let Some(color_mode) = layer.field_color_mode() else {
            continue;
        };
        let Some(runtime_state) = layer_runtime.get(layer.id) else {
            continue;
        };
        let Some(lookup) = exact_lookups.get(layer.id, &url) else {
            hide_field_layer(&mut commands, field_runtime.entries.get(&layer.id));
            continue;
        };
        if !runtime_state.visible || !matches!(layer.transform, LayerTransform::IdentityMapSpace) {
            hide_field_layer(&mut commands, field_runtime.entries.get(&layer.id));
            continue;
        }
        let Some(next_request) = build_texture_request(
            visible_map_rect,
            lookup.width(),
            lookup.height(),
            focus_output_width,
            focus_output_height,
        ) else {
            hide_field_layer(&mut commands, field_runtime.entries.get(&layer.id));
            continue;
        };

        ensure_field_layer_image(
            &mut commands,
            &mut images,
            &mut field_runtime,
            layer.id,
            next_request,
            lookup,
            color_mode,
        );

        let Some(entry) = field_runtime.entries.get(&layer.id) else {
            continue;
        };
        let (world_x, world_z, world_width, world_height) =
            request_world_placement(entry.request, map_to_world);
        commands.entity(entry.entity_2d).insert((
            Visibility::Visible,
            Sprite {
                image: entry.image.clone(),
                custom_size: Some(Vec2::new(world_width, world_height)),
                color: Color::srgba(1.0, 1.0, 1.0, runtime_state.opacity.clamp(0.0, 1.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(world_x, world_z, runtime_state.z_base)),
        ));
    }
}

fn ensure_field_layer_image(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    field_runtime: &mut FieldLayerRuntime,
    layer_id: LayerId,
    next_request: FieldTextureRequest,
    lookup: &fishystuff_core::field::DiscreteFieldRows,
    color_mode: FieldColorMode,
) {
    let should_rerender = field_runtime
        .entries
        .get(&layer_id)
        .map(|entry| !entry.request.can_reuse_for(next_request))
        .unwrap_or(true);
    if !should_rerender {
        return;
    }

    let image = render_lookup_request_image(lookup, next_request, color_mode);
    if let Some(entry) = field_runtime.entries.get_mut(&layer_id) {
        images.remove(entry.image.id());
        entry.image = images.add(image);
        entry.request = next_request;
        return;
    }

    let handle = images.add(image);
    let entity_2d = commands
        .spawn((
            World2dRenderEntity,
            world_2d_layers(),
            Sprite {
                image: handle.clone(),
                ..default()
            },
            Transform::default(),
            Visibility::Hidden,
        ))
        .id();
    field_runtime.entries.insert(
        layer_id,
        FieldLayerEntry {
            entity_2d,
            image: handle,
            request: next_request,
        },
    );
}

fn cleanup_stale_field_layers(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    field_runtime: &mut FieldLayerRuntime,
    active_exact_layers: &HashSet<LayerId>,
) {
    let stale_ids = field_runtime
        .entries
        .keys()
        .filter(|layer_id| !active_exact_layers.contains(layer_id))
        .copied()
        .collect::<Vec<_>>();
    for layer_id in stale_ids {
        if let Some(entry) = field_runtime.entries.remove(&layer_id) {
            commands.entity(entry.entity_2d).despawn();
            images.remove(entry.image.id());
        }
    }
}

fn hide_all_field_layers(commands: &mut Commands, field_runtime: &FieldLayerRuntime) {
    for entry in field_runtime.entries.values() {
        commands.entity(entry.entity_2d).insert(Visibility::Hidden);
    }
}

fn hide_field_layer(commands: &mut Commands, entry: Option<&FieldLayerEntry>) {
    let Some(entry) = entry else {
        return;
    };
    commands.entity(entry.entity_2d).insert(Visibility::Hidden);
}

fn build_texture_request(
    visible_map_rect: MapRect,
    field_width: u16,
    field_height: u16,
    focus_output_width: u16,
    focus_output_height: u16,
) -> Option<FieldTextureRequest> {
    let focus_origin_x = visible_map_rect.min.x.floor().max(0.0) as i32;
    let focus_origin_y = visible_map_rect.min.y.floor().max(0.0) as i32;
    let focus_max_x = visible_map_rect.max.x.ceil().min(f64::from(field_width)) as i32;
    let focus_max_y = visible_map_rect.max.y.ceil().min(f64::from(field_height)) as i32;
    if focus_max_x <= focus_origin_x || focus_max_y <= focus_origin_y {
        return None;
    }

    let focus_width = u32::try_from(focus_max_x - focus_origin_x).ok()?;
    let focus_height = u32::try_from(focus_max_y - focus_origin_y).ok()?;
    let margin_source_x = div_ceil_u32(
        focus_width.saturating_mul(FIELD_OVERSCAN_SCREEN_PX),
        u32::from(focus_output_width),
    );
    let margin_source_y = div_ceil_u32(
        focus_height.saturating_mul(FIELD_OVERSCAN_SCREEN_PX),
        u32::from(focus_output_height),
    );

    let source_origin_x = (i64::from(focus_origin_x) - i64::from(margin_source_x))
        .clamp(0, i64::from(field_width)) as i32;
    let source_origin_y = (i64::from(focus_origin_y) - i64::from(margin_source_y))
        .clamp(0, i64::from(field_height)) as i32;
    let source_max_x = (i64::from(focus_max_x) + i64::from(margin_source_x))
        .clamp(0, i64::from(field_width)) as i32;
    let source_max_y = (i64::from(focus_max_y) + i64::from(margin_source_y))
        .clamp(0, i64::from(field_height)) as i32;
    if source_max_x <= source_origin_x || source_max_y <= source_origin_y {
        return None;
    }

    Some(FieldTextureRequest {
        focus_origin_x,
        focus_origin_y,
        focus_width,
        focus_height,
        source_origin_x,
        source_origin_y,
        source_width: u32::try_from(source_max_x - source_origin_x).ok()?,
        source_height: u32::try_from(source_max_y - source_origin_y).ok()?,
        output_width: focus_output_width
            .checked_add(u16::try_from(FIELD_OVERSCAN_SCREEN_PX.checked_mul(2)?).ok()?)?,
        output_height: focus_output_height
            .checked_add(u16::try_from(FIELD_OVERSCAN_SCREEN_PX.checked_mul(2)?).ok()?)?,
    })
}

fn bounded_focus_output_size(viewport_width: f32, viewport_height: f32) -> Option<(u16, u16)> {
    let viewport_width = viewport_width.max(1.0).ceil() as u32;
    let viewport_height = viewport_height.max(1.0).ceil() as u32;
    let max_focus_edge = FIELD_MAX_TEXTURE_EDGE.checked_sub(FIELD_OVERSCAN_SCREEN_PX * 2)?;
    if max_focus_edge == 0 {
        return None;
    }
    let scale_down = ((viewport_width as f64) / (max_focus_edge as f64))
        .max((viewport_height as f64) / (max_focus_edge as f64))
        .max(1.0);
    let width = ((viewport_width as f64) / scale_down).ceil() as u32;
    let height = ((viewport_height as f64) / scale_down).ceil() as u32;
    Some((u16::try_from(width).ok()?, u16::try_from(height).ok()?))
}

fn request_world_placement(
    request: FieldTextureRequest,
    map_to_world: MapToWorld,
) -> (f32, f32, f32, f32) {
    let min = map_to_world.map_to_world(MapPoint::new(
        f64::from(request.source_origin_x),
        f64::from(request.source_origin_y),
    ));
    let max = map_to_world.map_to_world(MapPoint::new(
        f64::from(request.source_origin_x) + f64::from(request.source_width),
        f64::from(request.source_origin_y) + f64::from(request.source_height),
    ));
    let min_x = min.x.min(max.x) as f32;
    let max_x = min.x.max(max.x) as f32;
    let min_z = min.z.min(max.z) as f32;
    let max_z = min.z.max(max.z) as f32;
    (
        (min_x + max_x) * 0.5,
        (min_z + max_z) * 0.5,
        max_x - min_x,
        max_z - min_z,
    )
}

fn render_lookup_request_image(
    lookup: &fishystuff_core::field::DiscreteFieldRows,
    request: FieldTextureRequest,
    color_mode: FieldColorMode,
) -> Image {
    let chunk = lookup.render_rgba_resampled_chunk(
        request.source_origin_x,
        request.source_origin_y,
        request.source_width,
        request.source_height,
        request.output_width,
        request.output_height,
        |id| render_field_id_rgba(id, color_mode),
    );
    Image::new(
        Extent3d {
            width: u32::from(chunk.width()),
            height: u32::from(chunk.height()),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        chunk.into_data(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn render_field_id_rgba(id: u32, color_mode: FieldColorMode) -> [u8; 4] {
    match color_mode {
        FieldColorMode::RgbU24 => [
            ((id >> 16) & 0xff) as u8,
            ((id >> 8) & 0xff) as u8,
            (id & 0xff) as u8,
            255,
        ],
        FieldColorMode::DebugHash => {
            let hash = hash_u32(id);
            let r = ((hash >> 16) & 0xff) as u8;
            let g = ((hash >> 8) & 0xff) as u8;
            let b = (hash & 0xff) as u8;
            [r.max(32), g.max(32), b.max(32), 255]
        }
    }
}

fn hash_u32(mut value: u32) -> u32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}

fn world_rect_to_clamped_map_rect(world_rect: WorldRect, map_to_world: MapToWorld) -> MapRect {
    let map_min = map_to_world.world_to_map(WorldPoint::new(world_rect.min.x, world_rect.max.z));
    let map_max = map_to_world.world_to_map(WorldPoint::new(world_rect.max.x, world_rect.min.z));
    MapRect {
        min: MapPoint::new(
            map_min
                .x
                .min(map_max.x)
                .clamp(0.0, f64::from(map_to_world.image_size_x)),
            map_min
                .y
                .min(map_max.y)
                .clamp(0.0, f64::from(map_to_world.image_size_y)),
        ),
        max: MapPoint::new(
            map_min
                .x
                .max(map_max.x)
                .clamp(0.0, f64::from(map_to_world.image_size_x)),
            map_min
                .y
                .max(map_max.y)
                .clamp(0.0, f64::from(map_to_world.image_size_y)),
        ),
    }
}

fn view_rect(camera: &Camera, camera_transform: &Transform, window: &Window) -> Option<WorldRect> {
    let global = GlobalTransform::from(*camera_transform);
    let min = camera
        .viewport_to_world_2d(&global, Vec2::new(0.0, 0.0))
        .ok()?;
    let max = camera
        .viewport_to_world_2d(&global, Vec2::new(window.width(), window.height()))
        .ok()?;
    Some(WorldRect {
        min: WorldPoint::new(min.x.min(max.x) as f64, min.y.min(max.y) as f64),
        max: WorldPoint::new(min.x.max(max.x) as f64, min.y.max(max.y) as f64),
    })
}

fn div_ceil_u32(value: u32, divisor: u32) -> u32 {
    value.div_ceil(divisor.max(1))
}

fn rect_contains(
    outer_x: i32,
    outer_y: i32,
    outer_width: u32,
    outer_height: u32,
    inner_x: i32,
    inner_y: i32,
    inner_width: u32,
    inner_height: u32,
) -> bool {
    let outer_max_x = i64::from(outer_x) + i64::from(outer_width);
    let outer_max_y = i64::from(outer_y) + i64::from(outer_height);
    let inner_max_x = i64::from(inner_x) + i64::from(inner_width);
    let inner_max_y = i64::from(inner_y) + i64::from(inner_height);
    i64::from(inner_x) >= i64::from(outer_x)
        && i64::from(inner_y) >= i64::from(outer_y)
        && inner_max_x <= outer_max_x
        && inner_max_y <= outer_max_y
}

fn scale_change_within_tolerance(previous: u32, next: u32) -> bool {
    let previous = previous.max(1) as f64;
    let next = next.max(1) as f64;
    let ratio = (next / previous).max(previous / next);
    ratio <= 1.0 + FIELD_REUSE_SCALE_TOLERANCE
}

#[cfg(test)]
mod tests {
    use super::{
        bounded_focus_output_size, build_texture_request, scale_change_within_tolerance,
        FieldTextureRequest, FIELD_MAX_TEXTURE_EDGE, FIELD_OVERSCAN_SCREEN_PX,
    };
    use crate::map::spaces::{MapPoint, MapRect};

    #[test]
    fn bounded_focus_output_size_reserves_overscan_budget() {
        let (width, height) =
            bounded_focus_output_size(4096.0, 2160.0).expect("bounded output size");
        let total_width = u32::from(width) + FIELD_OVERSCAN_SCREEN_PX * 2;
        let total_height = u32::from(height) + FIELD_OVERSCAN_SCREEN_PX * 2;
        assert!(total_width <= FIELD_MAX_TEXTURE_EDGE);
        assert!(total_height <= FIELD_MAX_TEXTURE_EDGE);
    }

    #[test]
    fn request_reuse_allows_small_pan_without_rerender() {
        let previous = FieldTextureRequest {
            focus_origin_x: 1000,
            focus_origin_y: 2000,
            focus_width: 800,
            focus_height: 600,
            source_origin_x: 900,
            source_origin_y: 1900,
            source_width: 1000,
            source_height: 800,
            output_width: 1280,
            output_height: 960,
        };
        let next = FieldTextureRequest {
            focus_origin_x: 1040,
            focus_origin_y: 2020,
            focus_width: 810,
            focus_height: 590,
            source_origin_x: 940,
            source_origin_y: 1920,
            source_width: 1010,
            source_height: 790,
            output_width: 1280,
            output_height: 960,
        };
        assert!(previous.can_reuse_for(next));
    }

    #[test]
    fn request_reuse_rejects_large_zoom_change() {
        assert!(scale_change_within_tolerance(1000, 1180));
        assert!(!scale_change_within_tolerance(1000, 1400));
    }

    #[test]
    fn build_texture_request_expands_visible_rect_with_overscan() {
        let request = build_texture_request(
            MapRect {
                min: MapPoint::new(100.0, 200.0),
                max: MapPoint::new(900.0, 700.0),
            },
            11_560,
            10_540,
            1200,
            800,
        )
        .expect("request");
        assert_eq!(request.focus_origin_x, 100);
        assert_eq!(request.focus_origin_y, 200);
        assert!(request.source_origin_x < request.focus_origin_x);
        assert!(request.source_origin_y < request.focus_origin_y);
        assert!(request.source_width > request.focus_width);
        assert!(request.source_height > request.focus_height);
    }
}
