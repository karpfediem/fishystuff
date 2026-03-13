use std::collections::{HashMap, HashSet};

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use fishystuff_api::models::events::MapBboxPx;

use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::{MapPoint, WorldPoint};
use crate::plugins::api::{
    bevy_public_asset_path, FishCatalog, MapDisplayState, POINT_ICON_SCALE_MAX,
    POINT_ICON_SCALE_MIN,
};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};

use super::query::{PointsState, RenderPoint};

pub(super) const RING_RADIUS_GAME_UNITS: f32 = 500.0;
const RING_Z: f32 = 40.0;
const ICON_Z: f32 = 40.2;
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
pub(super) struct PointIconCache {
    handles: HashMap<i32, Handle<Image>>,
    missing: HashSet<i32>,
}

pub(super) fn sync_point_markers(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    display_state: Res<MapDisplayState>,
    view_mode: Res<ViewModeState>,
    fish: Res<FishCatalog>,
    mut points: ResMut<PointsState>,
    ring_assets: Res<PointRingAssets>,
    mut pool: ResMut<PointMarkerPool>,
    mut icon_cache: ResMut<PointIconCache>,
    camera_q: Query<&Projection, With<Map2dCamera>>,
    mut rings: Query<
        (&mut Transform, &mut Visibility, &mut Sprite),
        (With<EventPointRingMarker>, Without<EventPointIconMarker>),
    >,
    mut icons: Query<
        (&mut Transform, &mut Visibility, &mut Sprite),
        (With<EventPointIconMarker>, Without<EventPointRingMarker>),
    >,
) {
    if !display_state.show_points || view_mode.mode != ViewMode::Map2D {
        if !pool.markers.is_empty() {
            for pair in pool.markers.drain(..) {
                commands.entity(pair.ring).despawn();
                commands.entity(pair.icon).despawn();
            }
        }
        return;
    }

    if fish.is_changed() {
        icon_cache.missing.clear();
        icon_cache.handles.clear();
    }

    let icons_mode_changed = points.icons_enabled != display_state.show_point_icons;
    points.icons_enabled = display_state.show_point_icons;
    let icon_size_world_units = point_icon_world_size(&display_state, &camera_q);
    let icon_size_changed = (points.icon_size_world_units - icon_size_world_units).abs() > 0.01;
    points.icon_size_world_units = icon_size_world_units;

    if pool.markers.is_empty() && !points.points.is_empty() && !points.dirty {
        points.dirty = true;
    }

    let needs_refresh = points.dirty
        || icons_mode_changed
        || (points.icons_enabled && (fish.is_changed() || icon_size_changed));
    if !needs_refresh {
        return;
    }

    let Some(texture) = ring_assets.texture.as_ref() else {
        return;
    };
    if ring_assets.diameter_map_px <= 0.0 {
        return;
    }

    while pool.markers.len() < points.points.len() {
        let ring = commands
            .spawn((
                EventPointRingMarker,
                World2dRenderEntity,
                world_2d_layers(),
                Sprite {
                    image: texture.clone(),
                    custom_size: Some(Vec2::splat(ring_assets.diameter_map_px)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, RING_Z),
                Visibility::Hidden,
            ))
            .id();
        let icon = commands
            .spawn((
                EventPointIconMarker,
                World2dRenderEntity,
                world_2d_layers(),
                Sprite {
                    custom_size: Some(Vec2::splat(icon_size_world_units)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, ICON_Z),
                Visibility::Hidden,
            ))
            .id();
        pool.markers.push(MarkerPair { ring, icon });
    }

    for (idx, point) in points.points.iter().enumerate() {
        let world = map_point_to_world(point);
        let pair = pool.markers[idx];
        let (ring_scale, ring_alpha) = ring_style_for_point(point);
        let ring_diameter_world = ring_assets.diameter_map_px * ring_scale;
        let icon_diameter_world = icon_size_world_units.max(ring_diameter_world);
        if let Ok((mut transform, mut visibility, mut sprite)) = rings.get_mut(pair.ring) {
            transform.translation.x = world.x as f32;
            transform.translation.y = world.z as f32;
            transform.translation.z = RING_Z;
            sprite.custom_size = Some(Vec2::splat(ring_diameter_world));
            sprite.color = Color::srgba(1.0, 1.0, 1.0, ring_alpha);
            *visibility = Visibility::Visible;
        }

        if let Ok((mut transform, mut visibility, mut sprite)) = icons.get_mut(pair.icon) {
            transform.translation.x = world.x as f32;
            transform.translation.y = world.z as f32;
            transform.translation.z = ICON_Z;

            if points.icons_enabled {
                if let Some(handle) =
                    icon_handle_for_fish(point.fish_id, &mut icon_cache, &fish, &asset_server)
                {
                    if sprite.image != handle {
                        sprite.image = handle;
                    }
                    sprite.color = Color::WHITE;
                    sprite.custom_size = Some(Vec2::splat(icon_diameter_world));
                    *visibility = Visibility::Visible;
                } else {
                    *visibility = Visibility::Hidden;
                }
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }

    for pair in pool.markers.iter().skip(points.points.len()) {
        if let Ok((_, mut visibility, _)) = rings.get_mut(pair.ring) {
            *visibility = Visibility::Hidden;
        }
        if let Ok((_, mut visibility, _)) = icons.get_mut(pair.icon) {
            *visibility = Visibility::Hidden;
        }
    }

    points.dirty = false;
}

fn icon_handle_for_fish(
    fish_id: Option<i32>,
    cache: &mut PointIconCache,
    fish: &FishCatalog,
    asset_server: &AssetServer,
) -> Option<Handle<Image>> {
    let fish_id = fish_id?;
    if let Some(handle) = cache.handles.get(&fish_id) {
        return Some(handle.clone());
    }
    if cache.missing.contains(&fish_id) {
        return None;
    }

    let Some(url) = fish.icon_url_for_fish(fish_id) else {
        cache.missing.insert(fish_id);
        return None;
    };

    let handle: Handle<Image> = asset_server.load(bevy_public_asset_path(&url));
    cache.handles.insert(fish_id, handle.clone());
    Some(handle)
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
