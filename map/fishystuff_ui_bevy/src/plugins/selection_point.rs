use bevy::prelude::*;

use crate::bridge::contract::FishyMapSelectionPointKind;
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::plugins::api::SelectionState;
use crate::plugins::camera::Map2dCamera;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};
use crate::plugins::svg_icons::{UiSvgIconAssets, UiSvgIconKind};

const SELECTION_POINT_Z: f32 = 40.5;
const CLICKED_MARKER_SIZE_SCREEN_PX: f32 = 24.0;

const CLICKED_ACCENT_COLOR: [u8; 3] = [239, 92, 31];

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
    marker_entity: Option<Entity>,
}

#[derive(Component)]
struct SelectionPointMarker;

fn ensure_selection_point_assets(
    mut commands: Commands,
    mut assets: ResMut<SelectionPointAssets>,
    svg_icon_assets: Res<UiSvgIconAssets>,
) {
    if assets.marker_entity.is_none() {
        let Some(default_texture) = svg_icon_assets.handle(UiSvgIconKind::Crosshair) else {
            return;
        };
        let entity = commands
            .spawn((
                SelectionPointMarker,
                World2dRenderEntity,
                world_2d_layers(),
                Sprite {
                    image: default_texture,
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
    svg_icon_assets: Res<UiSvgIconAssets>,
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
        FishyMapSelectionPointKind::Bookmark | FishyMapSelectionPointKind::Waypoint => {
            *visibility = Visibility::Hidden;
        }
        FishyMapSelectionPointKind::Clicked => {
            let scale = camera_q
                .single()
                .ok()
                .map(|projection| match projection {
                    Projection::Orthographic(ortho) => ortho.scale.max(f32::EPSILON),
                    _ => 1.0,
                })
                .unwrap_or(1.0);
            let Some(texture) = svg_icon_assets.handle(UiSvgIconKind::Crosshair) else {
                *visibility = Visibility::Hidden;
                return;
            };
            transform.translation = Vec3::new(world_x as f32, world_z as f32, SELECTION_POINT_Z);
            sprite.image = texture;
            sprite.color = color_from_rgb(CLICKED_ACCENT_COLOR);
            sprite.custom_size = Some(Vec2::splat(CLICKED_MARKER_SIZE_SCREEN_PX * scale));
            *visibility = Visibility::Visible;
        }
    }
}

fn color_from_rgb(rgb: [u8; 3]) -> Color {
    Color::srgb_u8(rgb[0], rgb[1], rgb[2])
}
