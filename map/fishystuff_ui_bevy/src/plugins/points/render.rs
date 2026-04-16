use std::collections::{HashMap, HashSet};

use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::SystemParam;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use fishystuff_api::models::events::MapBboxPx;

use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::layers::{LayerRegistry, LayerRuntime, FISH_EVIDENCE_LAYER_KEY};
use crate::map::raster::{cache::clip_mask_allows_world_point, RasterTileCache};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::{MapPoint, WorldPoint};
use crate::plugins::api::{
    fish_item_icon_url, remote_image_handle, FishCatalog, MapDisplayState, RemoteImageCache,
    RemoteImageEpoch, RemoteImageStatus, POINT_ICON_SCALE_MAX, POINT_ICON_SCALE_MIN,
};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};
use crate::plugins::vector_layers::VectorLayerRuntime;

use super::{
    query::{PointsState, RenderPoint},
    EvidenceZoneFilter,
};

type PointRingQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static mut Visibility,
        &'static mut Sprite,
    ),
    (With<EventPointRingMarker>, Without<EventPointIconMarker>),
>;

type PointIconQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static mut Visibility,
        &'static mut Sprite,
    ),
    (With<EventPointIconMarker>, Without<EventPointRingMarker>),
>;

pub(super) const RING_RADIUS_GAME_UNITS: f32 = 500.0;
const RING_Z_OFFSET: f32 = 0.0;
const ICON_Z_OFFSET: f32 = 0.2;
const ICON_SIZE_SCREEN_PX: f32 = 12.0;
const RING_TEXTURE_SIZE_PX: usize = 32;
const RING_TEXTURE_THICKNESS_PX: f32 = 3.5;
const RING_COLOR: [u8; 3] = [255, 54, 26];
const AGGREGATE_RING_ALPHA_MIN: f32 = 0.35;
const AGGREGATE_RING_ALPHA_MAX: f32 = 0.92;

#[derive(Component)]
pub struct EventPointRingMarker;

#[derive(Component)]
pub struct EventPointIconMarker;

#[derive(Clone, Copy, Debug)]
struct MarkerPair {
    ring: Entity,
    icon: Entity,
}

#[derive(Resource, Default)]
pub(super) struct PointRingAssets {
    pub(super) texture: Option<Handle<Image>>,
    pub(super) diameter_map_px: f32,
}

#[derive(Resource, Default)]
pub(super) struct PointMarkerPool {
    markers: Vec<MarkerPair>,
}

#[derive(Resource, Default)]
pub(crate) struct PointIconCache {
    requested_urls: HashMap<i32, String>,
    loading_ids: HashSet<i32>,
    loaded_ids: HashSet<i32>,
    missing_catalog_ids: HashSet<i32>,
    failed_ids: HashMap<i32, String>,
    pub(crate) visible_icon_count: usize,
    visible_fish_ids_sample: Vec<i32>,
}

impl PointIconCache {
    pub(crate) fn requested_count(&self) -> usize {
        self.requested_urls.len()
    }

    pub(crate) fn loading_count(&self) -> usize {
        self.loading_ids.len()
    }

    pub(crate) fn loaded_count(&self) -> usize {
        self.loaded_ids.len()
    }

    pub(crate) fn failed_count(&self) -> usize {
        self.failed_ids.len()
    }

    pub(crate) fn missing_catalog_count(&self) -> usize {
        self.missing_catalog_ids.len()
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn missing_catalog_sample(&self) -> Vec<i32> {
        let mut ids = self.missing_catalog_ids.iter().copied().collect::<Vec<_>>();
        ids.sort_unstable();
        ids.truncate(5);
        ids
    }

    pub(crate) fn requested_sample(&self) -> Vec<String> {
        icon_sample(
            self.requested_urls
                .iter()
                .map(|(fish_id, url)| format!("{fish_id}:{url}")),
        )
    }

    pub(crate) fn failed_sample(&self) -> Vec<String> {
        icon_sample(
            self.failed_ids
                .iter()
                .map(|(fish_id, error)| format!("{fish_id}:{error}")),
        )
    }

    pub(crate) fn visible_sample(&self) -> Vec<i32> {
        self.visible_fish_ids_sample.clone()
    }
}

fn icon_sample<T>(values: impl Iterator<Item = T>) -> Vec<T>
where
    T: Ord,
{
    let mut values = values.collect::<Vec<_>>();
    values.sort();
    values.truncate(5);
    values
}

pub(super) fn sync_point_markers(mut context: PointMarkerSync<'_, '_>) {
    crate::perf_scope!("events.point_entity_update");
    let fish_evidence_layer_id = context.layer_registry.id_by_key(FISH_EVIDENCE_LAYER_KEY);
    let fish_evidence_visible = context
        .layer_registry
        .id_by_key(FISH_EVIDENCE_LAYER_KEY)
        .map(|id| context.layer_runtime.visible(id))
        .unwrap_or(context.display_state.show_points);
    let fish_evidence_icons_visible = context
        .layer_registry
        .id_by_key(FISH_EVIDENCE_LAYER_KEY)
        .map(|id| context.layer_runtime.point_icons_visible(id))
        .unwrap_or(context.display_state.show_point_icons);
    if !context.display_state.show_points
        || !fish_evidence_visible
        || context.view_mode.mode != ViewMode::Map2D
    {
        context.icon_cache.visible_icon_count = 0;
        context.icon_cache.visible_fish_ids_sample.clear();
        if !context.pool.markers.is_empty() {
            for pair in context.pool.markers.drain(..) {
                context.commands.entity(pair.ring).despawn();
                context.commands.entity(pair.icon).despawn();
            }
        }
        return;
    }

    if context.fish.is_changed() {
        context.icon_cache.requested_urls.clear();
        context.icon_cache.loading_ids.clear();
        context.icon_cache.loaded_ids.clear();
        context.icon_cache.missing_catalog_ids.clear();
        context.icon_cache.failed_ids.clear();
    }

    let effective_show_point_icons =
        context.display_state.show_point_icons && fish_evidence_icons_visible;
    let icons_mode_changed = context.points.icons_enabled != effective_show_point_icons;
    context.points.icons_enabled = effective_show_point_icons;
    let icon_size_world_units = point_icon_world_size(&context.display_state, &context.camera_q);
    let icon_size_changed =
        (context.points.icon_size_world_units - icon_size_world_units).abs() > 0.01;
    context.points.icon_size_world_units = icon_size_world_units;

    if context.pool.markers.is_empty() && !context.points.points.is_empty() && !context.points.dirty
    {
        context.points.dirty = true;
    }

    let needs_refresh = context.points.dirty
        || icons_mode_changed
        || (context.points.icons_enabled
            && (context.fish.is_changed()
                || icon_size_changed
                || context.remote_image_epoch.is_changed()));
    if !needs_refresh {
        return;
    }

    let Some(texture) = context.ring_assets.texture.as_ref() else {
        return;
    };
    if context.ring_assets.diameter_map_px <= 0.0 {
        return;
    }

    let mut spawned_markers = false;
    while context.pool.markers.len() < context.points.points.len() {
        spawned_markers = true;
        let ring = context
            .commands
            .spawn((
                EventPointRingMarker,
                World2dRenderEntity,
                world_2d_layers(),
                Sprite {
                    image: texture.clone(),
                    custom_size: Some(Vec2::splat(context.ring_assets.diameter_map_px)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, context.display_state.point_z_base + RING_Z_OFFSET),
                Visibility::Hidden,
            ))
            .id();
        let icon = context
            .commands
            .spawn((
                EventPointIconMarker,
                World2dRenderEntity,
                world_2d_layers(),
                Sprite {
                    custom_size: Some(Vec2::splat(icon_size_world_units)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, context.display_state.point_z_base + ICON_Z_OFFSET),
                Visibility::Hidden,
            ))
            .id();
        context.pool.markers.push(MarkerPair { ring, icon });
    }

    let mut visible_icon_count = 0usize;
    let mut visible_fish_ids = Vec::new();
    let point_opacity = context.display_state.point_opacity.clamp(0.0, 1.0);
    let ring_z = context.display_state.point_z_base + RING_Z_OFFSET;
    let icon_z = context.display_state.point_z_base + ICON_Z_OFFSET;
    for (idx, point) in context.points.points.iter().enumerate() {
        let world = map_point_to_world(point);
        let pair = context.pool.markers[idx];
        let point_visible_here = fish_evidence_layer_id.is_none_or(|layer_id| {
            world_point_visible_in_layer_clip(
                layer_id,
                world,
                &context.layer_registry,
                &context.layer_runtime,
                &context.exact_lookups,
                &context.tile_cache,
                &context.vector_runtime,
                &context.evidence_zone_filter,
            )
        });
        let (ring_scale, ring_alpha) = ring_style_for_point(point);
        let ring_diameter_world = context.ring_assets.diameter_map_px * ring_scale;
        let icon_diameter_world = icon_size_world_units.max(ring_diameter_world);
        if let Ok((mut transform, mut visibility, mut sprite)) = context.rings.get_mut(pair.ring) {
            transform.translation.x = world.x as f32;
            transform.translation.y = world.z as f32;
            transform.translation.z = ring_z;
            sprite.custom_size = Some(Vec2::splat(ring_diameter_world));
            sprite.color = Color::srgba(1.0, 1.0, 1.0, ring_alpha * point_opacity);
            *visibility = if point_visible_here {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }

        if let Ok((mut transform, mut visibility, mut sprite)) = context.icons.get_mut(pair.icon) {
            transform.translation.x = world.x as f32;
            transform.translation.y = world.z as f32;
            transform.translation.z = icon_z;

            if point_visible_here && context.points.icons_enabled {
                if let Some(handle) = icon_handle_for_point(
                    point,
                    &mut context.icon_cache,
                    &context.fish,
                    &mut context.remote_images,
                ) {
                    if sprite.image != handle {
                        sprite.image = handle;
                    }
                    sprite.color = Color::srgba(1.0, 1.0, 1.0, point_opacity);
                    sprite.custom_size = Some(Vec2::splat(icon_diameter_world));
                    *visibility = Visibility::Visible;
                    visible_icon_count += 1;
                    if let Some(fish_id) = point.fish_id {
                        visible_fish_ids.push(fish_id);
                    }
                } else {
                    *visibility = Visibility::Hidden;
                }
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }

    for pair in context
        .pool
        .markers
        .iter()
        .skip(context.points.points.len())
    {
        if let Ok((_, mut visibility, _)) = context.rings.get_mut(pair.ring) {
            *visibility = Visibility::Hidden;
        }
        if let Ok((_, mut visibility, _)) = context.icons.get_mut(pair.icon) {
            *visibility = Visibility::Hidden;
        }
    }

    let requested_ids = context
        .icon_cache
        .requested_urls
        .keys()
        .copied()
        .collect::<HashSet<_>>();
    let loaded_ids = context.icon_cache.loaded_ids.clone();
    context
        .icon_cache
        .loading_ids
        .retain(|fish_id| requested_ids.contains(fish_id) && !loaded_ids.contains(fish_id));
    visible_fish_ids.sort_unstable();
    visible_fish_ids.dedup();
    visible_fish_ids.truncate(5);
    context.icon_cache.visible_icon_count = visible_icon_count;
    context.icon_cache.visible_fish_ids_sample = visible_fish_ids;

    // New markers spawned through `Commands` do not appear in the query world until the
    // next frame. Keep the points dirty so the following frame positions and shows them
    // without needing an unrelated camera movement to retrigger the render pass.
    context.points.dirty = spawned_markers;
}

pub(super) fn mark_points_dirty_on_remote_image_update(
    remote_image_epoch: Res<RemoteImageEpoch>,
    mut points: ResMut<PointsState>,
) {
    if remote_image_epoch.is_changed() {
        points.dirty = true;
    }
}

#[derive(SystemParam)]
pub(super) struct PointMarkerSync<'w, 's> {
    commands: Commands<'w, 's>,
    display_state: Res<'w, MapDisplayState>,
    view_mode: Res<'w, ViewModeState>,
    layer_registry: Res<'w, LayerRegistry>,
    layer_runtime: Res<'w, LayerRuntime>,
    exact_lookups: Res<'w, ExactLookupCache>,
    tile_cache: Res<'w, RasterTileCache>,
    vector_runtime: Res<'w, VectorLayerRuntime>,
    evidence_zone_filter: Res<'w, EvidenceZoneFilter>,
    fish: Res<'w, FishCatalog>,
    points: ResMut<'w, PointsState>,
    ring_assets: Res<'w, PointRingAssets>,
    pool: ResMut<'w, PointMarkerPool>,
    icon_cache: ResMut<'w, PointIconCache>,
    remote_image_epoch: Res<'w, RemoteImageEpoch>,
    remote_images: ResMut<'w, RemoteImageCache>,
    camera_q: Query<'w, 's, &'static Projection, With<Map2dCamera>>,
    rings: PointRingQuery<'w, 's>,
    icons: PointIconQuery<'w, 's>,
}

fn icon_handle_for_point(
    point: &RenderPoint,
    cache: &mut PointIconCache,
    fish: &FishCatalog,
    remote_images: &mut RemoteImageCache,
) -> Option<Handle<Image>> {
    let fish_id = point.fish_id?;
    let Some(item_id) = fish.item_id_for_fish(fish_id) else {
        cache.missing_catalog_ids.insert(fish_id);
        cache.loading_ids.remove(&fish_id);
        cache.loaded_ids.remove(&fish_id);
        cache.failed_ids.remove(&fish_id);
        return None;
    };
    let Some(url) = fish_item_icon_url(item_id) else {
        cache.loading_ids.remove(&fish_id);
        cache.loaded_ids.remove(&fish_id);
        cache
            .failed_ids
            .insert(fish_id, "invalid item icon url".to_string());
        return None;
    };
    cache.requested_urls.insert(fish_id, url);
    let url = cache
        .requested_urls
        .get(&fish_id)
        .cloned()
        .unwrap_or_default();
    match remote_image_handle(&url, remote_images) {
        RemoteImageStatus::Ready(handle) => {
            cache.loading_ids.remove(&fish_id);
            cache.loaded_ids.insert(fish_id);
            cache.missing_catalog_ids.remove(&fish_id);
            cache.failed_ids.remove(&fish_id);
            Some(handle)
        }
        RemoteImageStatus::Pending => {
            cache.loading_ids.insert(fish_id);
            cache.loaded_ids.remove(&fish_id);
            cache.missing_catalog_ids.remove(&fish_id);
            cache.failed_ids.remove(&fish_id);
            None
        }
        RemoteImageStatus::Failed(error) => {
            cache.loading_ids.remove(&fish_id);
            cache.loaded_ids.remove(&fish_id);
            cache.missing_catalog_ids.remove(&fish_id);
            cache.failed_ids.insert(fish_id, error);
            None
        }
    }
}

pub(super) fn view_bbox_map_px(
    windows: &Query<&Window>,
    camera_q: &Query<(&Camera, &Transform), With<Map2dCamera>>,
) -> Option<MapBboxPx> {
    let window = windows.single().ok()?;
    let (camera, camera_transform) = camera_q.single().ok()?;
    let global = GlobalTransform::from(*camera_transform);
    let min_world = camera
        .viewport_to_world_2d(&global, Vec2::new(0.0, 0.0))
        .ok()?;
    let max_world = camera
        .viewport_to_world_2d(&global, Vec2::new(window.width(), window.height()))
        .ok()?;

    let world_min_x = min_world.x.min(max_world.x) as f64;
    let world_max_x = min_world.x.max(max_world.x) as f64;
    let world_min_z = min_world.y.min(max_world.y) as f64;
    let world_max_z = min_world.y.max(max_world.y) as f64;
    let map_to_world = MapToWorld::default();

    let map_min = map_to_world.world_to_map(WorldPoint::new(world_min_x, world_min_z));
    let map_max = map_to_world.world_to_map(WorldPoint::new(world_max_x, world_max_z));
    let mut min_x = map_min.x.min(map_max.x).floor() as i32;
    let mut max_x = map_min.x.max(map_max.x).ceil() as i32;
    let mut min_y = map_min.y.min(map_max.y).floor() as i32;
    let mut max_y = map_min.y.max(map_max.y).ceil() as i32;

    let map_max_x = map_to_world.image_size_x as i32 - 1;
    let map_max_y = map_to_world.image_size_y as i32 - 1;
    min_x = min_x.clamp(0, map_max_x);
    max_x = max_x.clamp(0, map_max_x);
    min_y = min_y.clamp(0, map_max_y);
    max_y = max_y.clamp(0, map_max_y);
    if min_x > max_x || min_y > max_y {
        return None;
    }

    Some(MapBboxPx {
        min_x,
        min_y,
        max_x,
        max_y,
    })
}

fn point_icon_world_size(
    display_state: &MapDisplayState,
    camera_q: &Query<&Projection, With<Map2dCamera>>,
) -> f32 {
    let current_scale = camera_q
        .single()
        .ok()
        .and_then(|projection| match projection {
            Projection::Orthographic(ortho) => Some(ortho.scale),
            _ => None,
        })
        .unwrap_or(1.0)
        .max(f32::EPSILON);
    let user_scale = display_state
        .point_icon_scale
        .clamp(POINT_ICON_SCALE_MIN, POINT_ICON_SCALE_MAX);
    ICON_SIZE_SCREEN_PX * current_scale * user_scale
}

fn ring_style_for_point(point: &RenderPoint) -> (f32, f32) {
    if !point.aggregated {
        return (1.0, 1.0);
    }
    let count = point.sample_count.max(1) as f32;
    let scale = (1.0 + count.log2() / 5.0).clamp(1.1, 3.0);
    let alpha = (AGGREGATE_RING_ALPHA_MIN + count.log10() * 0.22)
        .clamp(AGGREGATE_RING_ALPHA_MIN, AGGREGATE_RING_ALPHA_MAX);
    (scale, alpha)
}

fn map_point_to_world(point: &RenderPoint) -> WorldPoint {
    if let (Some(world_x), Some(world_z)) = (point.world_x, point.world_z) {
        return WorldPoint::new(world_x as f64, world_z as f64);
    }
    let map_to_world = MapToWorld::default();
    map_to_world.map_to_world(MapPoint::new(
        point.map_px_x as f64 + 0.5,
        point.map_px_y as f64 + 0.5,
    ))
}

fn world_point_visible_in_layer_clip(
    layer_id: crate::map::layers::LayerId,
    world_point: WorldPoint,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
    exact_lookups: &ExactLookupCache,
    tile_cache: &RasterTileCache,
    vector_runtime: &VectorLayerRuntime,
    evidence_zone_filter: &EvidenceZoneFilter,
) -> bool {
    !matches!(
        clip_mask_allows_world_point(
            layer_id,
            world_point,
            layer_registry,
            layer_runtime,
            exact_lookups,
            tile_cache,
            vector_runtime,
            evidence_zone_filter,
            layer_registry.map_version_id(),
        ),
        Some(false)
    )
}

pub(super) fn build_ring_texture() -> Image {
    let size = RING_TEXTURE_SIZE_PX;
    let center = (size as f32 - 1.0) * 0.5;
    let radius = center - 1.0;
    let half_thickness = RING_TEXTURE_THICKNESS_PX * 0.5;
    let feather = 1.0_f32;

    let mut texture_data = vec![0_u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let ring_distance = (dist - radius).abs();
            let alpha = if ring_distance <= half_thickness {
                1.0
            } else if ring_distance <= half_thickness + feather {
                1.0 - (ring_distance - half_thickness) / feather
            } else {
                0.0
            };

            let offset = (y * size + x) * 4;
            texture_data[offset] = RING_COLOR[0];
            texture_data[offset + 1] = RING_COLOR[1];
            texture_data[offset + 2] = RING_COLOR[2];
            texture_data[offset + 3] = (alpha * 255.0).round() as u8;
        }
    }

    Image::new_fill(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[cfg(test)]
mod tests {
    use bevy::app::TaskPoolPlugin;
    use bevy::prelude::App;

    use super::{icon_handle_for_point, PointIconCache, RenderPoint};
    use crate::plugins::api::{FishCatalog, FishEntry, RemoteImageCache};

    #[test]
    fn aggregated_points_still_request_icons_for_representative_fish() {
        let mut app = App::new();
        app.add_plugins(TaskPoolPlugin::default());

        let mut fish = FishCatalog::default();
        fish.replace(vec![FishEntry {
            id: 88,
            item_id: 8289,
            encyclopedia_key: Some(88),
            encyclopedia_id: Some(8588),
            name: "Barbel Steed".to_string(),
            name_lower: "barbel steed".to_string(),
            grade: Some("Rare".to_string()),
            is_prize: false,
        }]);
        let point = RenderPoint {
            map_px_x: 100,
            map_px_y: 200,
            world_x: None,
            world_z: None,
            fish_id: Some(88),
            zone_rgb_u32: None,
            sample_count: 4,
            aggregated: true,
        };
        let mut cache = PointIconCache::default();
        let mut remote_images = RemoteImageCache::default();

        let handle = icon_handle_for_point(&point, &mut cache, &fish, &mut remote_images);

        assert!(handle.is_none());
        assert_eq!(
            cache.requested_urls.get(&88).map(String::as_str),
            Some("https://cdn.fishystuff.fish/images/items/00008289.webp")
        );
        assert!(cache.loading_ids.contains(&88));
    }
}
