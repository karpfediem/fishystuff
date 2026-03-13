use bevy::camera::ClearColorConfig;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ui::IsDefaultUiCamera;
use bevy::window::{PrimaryWindow, WindowResolution};

use crate::map::camera::map2d::{apply_map2d_camera_state, Map2dViewState};
use crate::map::camera::terrain3d::reset_for_world_bounds;
use crate::map::spaces::world::MapToWorld;
use crate::plugins::render_domain::{ui_layers, world_2d_layers, world_3d_layers};
use crate::prelude::*;

#[derive(Component, Debug)]
pub struct Map2dCamera;

#[derive(Component, Debug)]
pub struct Terrain3dCamera;

#[derive(Component, Debug)]
pub struct UiCamera;

#[derive(Resource, Default)]
pub struct CameraFitState {
    fitted: bool,
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct CameraZoomBounds {
    pub fit_scale: f32,
    pub min_scale: f32,
    pub max_scale: f32,
}

impl Default for CameraZoomBounds {
    fn default() -> Self {
        Self {
            fit_scale: 1.0,
            min_scale: 0.05,
            max_scale: 2.5,
        }
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraFitState>()
            .init_resource::<CameraZoomBounds>()
            .add_systems(Startup, spawn_cameras)
            .add_systems(Update, fit_camera_once);
    }
}

pub fn initial_resolution() -> WindowResolution {
    let (logical_width, logical_height) = viewport_logical_size().unwrap_or((1280, 720));
    WindowResolution::new(logical_width, logical_height)
}

fn viewport_logical_size() -> Option<(u32, u32)> {
    let window = web_sys::window()?;
    let width = window.inner_width().ok()?.as_f64()?.max(1.0);
    let height = window.inner_height().ok()?.as_f64()?.max(1.0);
    Some((width.round() as u32, height.round() as u32))
}

fn spawn_cameras(mut commands: Commands) {
    let world_bounds = MapToWorld::default().world_bounds();
    let center_x = ((world_bounds.min.x + world_bounds.max.x) * 0.5) as f32;
    let center_z = ((world_bounds.min.z + world_bounds.max.z) * 0.5) as f32;
    let terrain_view = reset_for_world_bounds(world_bounds);
    let mut perspective = PerspectiveProjection::default();
    perspective.fov = 55.0_f32.to_radians();
    perspective.near = 1.0;
    perspective.far = 12_000_000.0;

    commands.spawn((
        Map2dCamera,
        Camera2d,
        Tonemapping::None,
        Camera {
            order: 0,
            is_active: true,
            clear_color: ClearColorConfig::Default,
            ..default()
        },
        world_2d_layers(),
        Transform::from_xyz(center_x, center_z, 1000.0),
        GlobalTransform::default(),
    ));

    commands.spawn((
        Terrain3dCamera,
        Camera3d::default(),
        Tonemapping::None,
        Camera {
            order: 0,
            is_active: false,
            clear_color: ClearColorConfig::Default,
            ..default()
        },
        Projection::Perspective(perspective),
        world_3d_layers(),
        terrain_view.camera_transform(),
        GlobalTransform::default(),
    ));

    commands.spawn((
        UiCamera,
        Camera2d,
        Tonemapping::None,
        IsDefaultUiCamera,
        Camera {
            order: 100,
            is_active: true,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        ui_layers(),
        Transform::from_xyz(0.0, 0.0, 1000.0),
        GlobalTransform::default(),
    ));
}

fn fit_camera_once(
    mut fit_state: ResMut<CameraFitState>,
    mut zoom_bounds: ResMut<CameraZoomBounds>,
    mut map_view: ResMut<Map2dViewState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut camera_q: Query<(&mut Projection, &mut Transform), With<Map2dCamera>>,
) {
    if fit_state.fitted {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    if window.width() <= 1.0 || window.height() <= 1.0 {
        return;
    }
    let Ok((mut projection, mut transform)) = camera_q.single_mut() else {
        return;
    };
    let world_bounds = MapToWorld::default().world_bounds();
    let world_w = (world_bounds.max.x - world_bounds.min.x) as f32;
    let world_h = (world_bounds.max.z - world_bounds.min.z) as f32;
    let fit_scale = (world_w / window.width()).max(world_h / window.height());
    let min_scale = fit_scale * ZOOM_MIN_FACTOR_OF_FIT;
    let max_scale = fit_scale * ZOOM_MAX_FACTOR_OF_FIT;
    zoom_bounds.fit_scale = fit_scale;
    zoom_bounds.min_scale = min_scale;
    zoom_bounds.max_scale = max_scale;
    map_view.zoom = fit_scale.clamp(min_scale, max_scale);
    apply_map2d_camera_state(&map_view, &mut transform, &mut projection);
    fit_state.fitted = true;
}
