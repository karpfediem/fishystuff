use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::bridge::contract::FishyMapBookmarkEntry;
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};

const BOOKMARK_MARKER_SIZE_SCREEN_PX: f32 = 22.0;
const BOOKMARK_MARKER_Z: f32 = 40.4;
const BOOKMARK_TEXTURE_WIDTH_PX: usize = 32;
const BOOKMARK_TEXTURE_HEIGHT_PX: usize = 32;
const BOOKMARK_RING_RADIUS_PX: f32 = 12.0;
const BOOKMARK_RING_THICKNESS_PX: f32 = 4.0;
const BOOKMARK_CORE_RADIUS_PX: f32 = 5.0;
const BOOKMARK_COLOR: [u8; 3] = [239, 92, 31];
const BOOKMARK_CORE_COLOR: [u8; 3] = [255, 242, 214];
const EDGE_FEATHER_PX: f32 = 1.2;

pub struct BookmarksPlugin;

impl Plugin for BookmarksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BookmarkState>()
            .init_resource::<BookmarkMarkerAssets>()
            .init_resource::<BookmarkMarkerPool>()
            .add_systems(
                Update,
                (ensure_bookmark_marker_assets, sync_bookmark_markers).chain(),
            );
    }
}

#[derive(Resource, Default)]
pub struct BookmarkState {
    pub entries: Vec<FishyMapBookmarkEntry>,
}

#[derive(Component)]
struct BookmarkMarker;

#[derive(Resource, Default)]
struct BookmarkMarkerAssets {
    texture: Option<Handle<Image>>,
}

#[derive(Resource, Default)]
struct BookmarkMarkerPool {
    markers: Vec<Entity>,
}

fn ensure_bookmark_marker_assets(
    mut marker_assets: ResMut<BookmarkMarkerAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    if marker_assets.texture.is_some() {
        return;
    }

    marker_assets.texture = Some(images.add(build_bookmark_marker_texture()));
}

fn sync_bookmark_markers(
    mut commands: Commands,
    bookmarks: Res<BookmarkState>,
    view_mode: Res<ViewModeState>,
    marker_assets: Res<BookmarkMarkerAssets>,
    mut marker_pool: ResMut<BookmarkMarkerPool>,
    camera_q: Query<&Projection, With<Map2dCamera>>,
    mut markers: Query<(&mut Transform, &mut Visibility, &mut Sprite), With<BookmarkMarker>>,
) {
    if view_mode.mode != ViewMode::Map2D || bookmarks.entries.is_empty() {
        for entity in &marker_pool.markers {
            if let Ok((_, mut visibility, _)) = markers.get_mut(*entity) {
                *visibility = Visibility::Hidden;
            }
        }
        return;
    }

    let Some(texture) = marker_assets.texture.as_ref() else {
        return;
    };
    let marker_size_world = bookmark_marker_world_size(&camera_q);

    while marker_pool.markers.len() < bookmarks.entries.len() {
        let marker = commands
            .spawn((
                BookmarkMarker,
                World2dRenderEntity,
                world_2d_layers(),
                Sprite {
                    image: texture.clone(),
                    custom_size: Some(Vec2::splat(marker_size_world)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, BOOKMARK_MARKER_Z),
                Visibility::Hidden,
            ))
            .id();
        marker_pool.markers.push(marker);
    }

    for (index, bookmark) in bookmarks.entries.iter().enumerate() {
        let entity = marker_pool.markers[index];
        if let Ok((mut transform, mut visibility, mut sprite)) = markers.get_mut(entity) {
            transform.translation.x = bookmark.world_x as f32;
            transform.translation.y = bookmark.world_z as f32;
            transform.translation.z = BOOKMARK_MARKER_Z;
            sprite.image = texture.clone();
            sprite.custom_size = Some(Vec2::splat(marker_size_world));
            *visibility = Visibility::Visible;
        }
    }

    for entity in marker_pool.markers.iter().skip(bookmarks.entries.len()) {
        if let Ok((_, mut visibility, _)) = markers.get_mut(*entity) {
            *visibility = Visibility::Hidden;
        }
    }
}

fn bookmark_marker_world_size(camera_q: &Query<&Projection, With<Map2dCamera>>) -> f32 {
    let current_scale = camera_q
        .single()
        .ok()
        .and_then(|projection| match projection {
            Projection::Orthographic(ortho) => Some(ortho.scale),
            _ => None,
        })
        .unwrap_or(1.0)
        .max(f32::EPSILON);
    BOOKMARK_MARKER_SIZE_SCREEN_PX * current_scale
}

fn build_bookmark_marker_texture() -> Image {
    let width = BOOKMARK_TEXTURE_WIDTH_PX;
    let height = BOOKMARK_TEXTURE_HEIGHT_PX;
    let center_x = (width as f32 - 1.0) * 0.5;
    let center_y = (height as f32 - 1.0) * 0.5;

    let mut texture_data = vec![0_u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance = (dx * dx + dy * dy).sqrt();

            let ring_alpha = ring_alpha(distance);
            let core_alpha = circle_alpha(distance, BOOKMARK_CORE_RADIUS_PX);
            let alpha = ring_alpha.max(core_alpha);
            if alpha <= 0.0 {
                continue;
            }

            let color = if core_alpha > 0.0 {
                BOOKMARK_CORE_COLOR
            } else {
                BOOKMARK_COLOR
            };
            let offset = (y * width + x) * 4;
            texture_data[offset] = color[0];
            texture_data[offset + 1] = color[1];
            texture_data[offset + 2] = color[2];
            texture_data[offset + 3] = (alpha * 255.0).round() as u8;
        }
    }

    Image::new_fill(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn ring_alpha(distance: f32) -> f32 {
    let half_thickness = BOOKMARK_RING_THICKNESS_PX * 0.5;
    let edge_distance = (distance - BOOKMARK_RING_RADIUS_PX).abs();
    if edge_distance <= half_thickness {
        return 1.0;
    }
    if edge_distance <= half_thickness + EDGE_FEATHER_PX {
        return 1.0 - (edge_distance - half_thickness) / EDGE_FEATHER_PX;
    }
    0.0
}

fn circle_alpha(distance: f32, radius: f32) -> f32 {
    if distance <= radius {
        return 1.0;
    }
    if distance <= radius + EDGE_FEATHER_PX {
        return 1.0 - (distance - radius) / EDGE_FEATHER_PX;
    }
    0.0
}
