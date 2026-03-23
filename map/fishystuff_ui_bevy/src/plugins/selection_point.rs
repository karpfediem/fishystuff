use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::bridge::contract::FishyMapSelectionPointKind;
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::plugins::api::SelectionState;
use crate::plugins::camera::Map2dCamera;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};

const SELECTION_POINT_Z: f32 = 40.5;
const CLICKED_MARKER_SIZE_SCREEN_PX: f32 = 24.0;
const WAYPOINT_MARKER_SIZE_SCREEN_PX: f32 = 28.0;

const MARKER_TEXTURE_WIDTH_PX: usize = 48;
const MARKER_TEXTURE_HEIGHT_PX: usize = 48;
const EDGE_FEATHER_PX: f32 = 1.4;

const CLICKED_MARKER_COLOR: [u8; 3] = [255, 244, 214];
const CLICKED_ACCENT_COLOR: [u8; 3] = [70, 200, 255];
const WAYPOINT_MARKER_COLOR: [u8; 3] = [255, 196, 66];
const WAYPOINT_CORE_COLOR: [u8; 3] = [255, 244, 214];

pub struct SelectionPointPlugin;

impl Plugin for SelectionPointPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectionPointAssets>().add_systems(
            Update,
            (ensure_selection_point_assets, sync_selection_point_marker),
        );
    }
}

#[derive(Resource, Default)]
struct SelectionPointAssets {
    clicked_texture: Option<Handle<Image>>,
    waypoint_texture: Option<Handle<Image>>,
    marker_entity: Option<Entity>,
}

#[derive(Component)]
struct SelectionPointMarker;

fn ensure_selection_point_assets(
    mut commands: Commands,
    mut assets: ResMut<SelectionPointAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    if assets.clicked_texture.is_none() {
        assets.clicked_texture = Some(images.add(build_clicked_marker_texture()));
    }
    if assets.waypoint_texture.is_none() {
        assets.waypoint_texture = Some(images.add(build_waypoint_marker_texture()));
    }
    if assets.marker_entity.is_none() {
        let Some(clicked_texture) = assets.clicked_texture.clone() else {
            return;
        };
        let entity = commands
            .spawn((
                SelectionPointMarker,
                World2dRenderEntity,
                world_2d_layers(),
                Sprite {
                    image: clicked_texture,
                    custom_size: Some(Vec2::splat(CLICKED_MARKER_SIZE_SCREEN_PX)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, SELECTION_POINT_Z),
                Visibility::Hidden,
            ))
            .id();
        assets.marker_entity = Some(entity);
    }
}

fn sync_selection_point_marker(
    selection: Res<SelectionState>,
    view_mode: Res<ViewModeState>,
    assets: Res<SelectionPointAssets>,
    camera_q: Query<&Projection, With<Map2dCamera>>,
    mut marker_q: Query<(&mut Transform, &mut Visibility, &mut Sprite), With<SelectionPointMarker>>,
) {
    let Some(marker_entity) = assets.marker_entity else {
        return;
    };
    let Ok((mut transform, mut visibility, mut sprite)) = marker_q.get_mut(marker_entity) else {
        return;
    };

    if view_mode.mode != ViewMode::Map2D {
        *visibility = Visibility::Hidden;
        return;
    }

    let Some(info) = selection.info.as_ref() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let Some((world_x, world_z)) = info.effective_world_point() else {
        *visibility = Visibility::Hidden;
        return;
    };

    match info
        .point_kind
        .unwrap_or(FishyMapSelectionPointKind::Clicked)
    {
        FishyMapSelectionPointKind::Bookmark => {
            *visibility = Visibility::Hidden;
        }
        FishyMapSelectionPointKind::Clicked | FishyMapSelectionPointKind::Waypoint => {
            let scale = camera_q
                .single()
                .ok()
                .map(|projection| match projection {
                    Projection::Orthographic(ortho) => ortho.scale.max(f32::EPSILON),
                    _ => 1.0,
                })
                .unwrap_or(1.0);
            let (texture, size_px) = marker_visual(
                info.point_kind
                    .unwrap_or(FishyMapSelectionPointKind::Clicked),
                &assets,
            );
            transform.translation = Vec3::new(world_x as f32, world_z as f32, SELECTION_POINT_Z);
            sprite.image = texture;
            sprite.custom_size = Some(Vec2::splat(size_px * scale));
            *visibility = Visibility::Visible;
        }
    }
}

fn marker_visual(
    point_kind: FishyMapSelectionPointKind,
    assets: &SelectionPointAssets,
) -> (Handle<Image>, f32) {
    match point_kind {
        FishyMapSelectionPointKind::Waypoint => (
            assets
                .waypoint_texture
                .clone()
                .or_else(|| assets.clicked_texture.clone())
                .expect("selection point waypoint texture"),
            WAYPOINT_MARKER_SIZE_SCREEN_PX,
        ),
        _ => (
            assets
                .clicked_texture
                .clone()
                .expect("selection point clicked texture"),
            CLICKED_MARKER_SIZE_SCREEN_PX,
        ),
    }
}

fn build_clicked_marker_texture() -> Image {
    let width = MARKER_TEXTURE_WIDTH_PX;
    let height = MARKER_TEXTURE_HEIGHT_PX;
    let center_x = (width as f32 - 1.0) * 0.5;
    let center_y = (height as f32 - 1.0) * 0.5;
    let outer_radius = 14.0;
    let ring_thickness = 2.6;
    let gap_radius = 6.0;
    let crosshair_thickness = 1.8;
    let line_extent = 18.0;
    let mut data = vec![0u8; width * height * 4];

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance = dx.hypot(dy);
            let ring_distance = (distance - outer_radius).abs();
            let on_ring = smooth_alpha(ring_distance, ring_thickness * 0.5, EDGE_FEATHER_PX);
            let on_vertical = if dx.abs() <= crosshair_thickness
                && dy.abs() >= gap_radius
                && dy.abs() <= line_extent
            {
                smooth_alpha(dx.abs(), crosshair_thickness, EDGE_FEATHER_PX)
            } else {
                0.0
            };
            let on_horizontal = if dy.abs() <= crosshair_thickness
                && dx.abs() >= gap_radius
                && dx.abs() <= line_extent
            {
                smooth_alpha(dy.abs(), crosshair_thickness, EDGE_FEATHER_PX)
            } else {
                0.0
            };
            let accent_alpha = on_vertical.max(on_horizontal);
            let alpha = on_ring.max(accent_alpha);
            if alpha <= 0.0 {
                continue;
            }
            let color = if accent_alpha > on_ring {
                CLICKED_ACCENT_COLOR
            } else {
                CLICKED_MARKER_COLOR
            };
            write_pixel(&mut data, width, x, y, color, alpha);
        }
    }

    build_image(width as u32, height as u32, data)
}

fn build_waypoint_marker_texture() -> Image {
    let width = MARKER_TEXTURE_WIDTH_PX;
    let height = MARKER_TEXTURE_HEIGHT_PX;
    let center_x = (width as f32 - 1.0) * 0.5;
    let center_y = (height as f32 - 1.0) * 0.5;
    let outer_radius = 13.5;
    let ring_thickness = 4.2;
    let inner_radius = 4.2;
    let mut data = vec![0u8; width * height * 4];

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance = dx.hypot(dy);
            let ring_distance = (distance - outer_radius).abs();
            let ring_alpha = smooth_alpha(ring_distance, ring_thickness * 0.5, EDGE_FEATHER_PX);
            let core_alpha = smooth_alpha(distance, inner_radius, EDGE_FEATHER_PX);
            let alpha = ring_alpha.max(core_alpha);
            if alpha <= 0.0 {
                continue;
            }
            let color = if core_alpha > ring_alpha {
                WAYPOINT_CORE_COLOR
            } else {
                WAYPOINT_MARKER_COLOR
            };
            write_pixel(&mut data, width, x, y, color, alpha);
        }
    }

    build_image(width as u32, height as u32, data)
}

fn smooth_alpha(distance: f32, threshold: f32, feather: f32) -> f32 {
    let fade_start = (threshold - feather).max(0.0);
    if distance <= fade_start {
        return 1.0;
    }
    if distance >= threshold + feather {
        return 0.0;
    }
    1.0 - ((distance - fade_start) / ((threshold + feather) - fade_start)).clamp(0.0, 1.0)
}

fn write_pixel(data: &mut [u8], width: usize, x: usize, y: usize, color: [u8; 3], alpha: f32) {
    let idx = (y * width + x) * 4;
    data[idx] = color[0];
    data[idx + 1] = color[1];
    data[idx + 2] = color[2];
    data[idx + 3] = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
}

fn build_image(width: u32, height: u32, data: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
