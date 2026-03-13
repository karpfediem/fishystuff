use bevy::camera::visibility::RenderLayers;
use bevy::ui::IsDefaultUiCamera;

use crate::map::camera::map2d::{apply_map2d_camera_state, Map2dViewState};
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::camera::terrain3d::{apply_terrain3d_camera_state, Terrain3dViewState};
use crate::plugins::camera::{Map2dCamera, Terrain3dCamera, UiCamera};
#[cfg(debug_assertions)]
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};
#[cfg(debug_assertions)]
use crate::plugins::render_domain::{world_3d_layers, World3dRenderEntity};
use crate::prelude::*;

#[derive(Component)]
pub(super) struct TerrainLightTag;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::map::terrain) struct CameraActivationState {
    pub map2d_active: bool,
    pub terrain3d_active: bool,
    pub ui_active: bool,
}

pub(in crate::map::terrain) fn camera_activation_for_mode(mode: ViewMode) -> CameraActivationState {
    match mode {
        ViewMode::Map2D => CameraActivationState {
            map2d_active: true,
            terrain3d_active: false,
            ui_active: true,
        },
        ViewMode::Terrain3D => CameraActivationState {
            map2d_active: false,
            terrain3d_active: true,
            ui_active: true,
        },
    }
}

pub(in crate::map::terrain) fn terrain3d_controls_should_run(
    mode: ViewMode,
    camera_active: bool,
) -> bool {
    mode == ViewMode::Terrain3D && camera_active
}

pub(in crate::map::terrain) fn clear_camera_control_mutation_flags(
    mut flags: ResMut<CameraControlMutationFlags>,
) {
    *flags = CameraControlMutationFlags::default();
}

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct CameraControlMutationFlags {
    pub map2d_updated: bool,
    pub terrain3d_updated: bool,
}

#[derive(Resource, Debug, Clone, Copy, Default)]
pub(super) struct AppliedViewMode {
    pub mode: Option<ViewMode>,
}

pub(in crate::map::terrain) fn ensure_terrain3d_projection(projection: &mut Projection) {
    let mut perspective = match projection {
        Projection::Perspective(existing) => existing.clone(),
        _ => PerspectiveProjection::default(),
    };
    perspective.fov = 55.0_f32.to_radians();
    perspective.near = 1.0;
    perspective.far = 12_000_000.0;
    *projection = Projection::Perspective(perspective);
}

fn enter_map2d(
    map_view: &Map2dViewState,
    map_camera: &mut Camera,
    map_transform: &mut Transform,
    map_projection: &mut Projection,
    terrain_camera: &mut Camera,
    ambient: &mut GlobalAmbientLight,
    light_q: &mut Query<&mut Visibility, With<TerrainLightTag>>,
) {
    enter_map2d_cameras_only(
        map_view,
        map_camera,
        map_transform,
        map_projection,
        terrain_camera,
    );
    for mut vis in light_q.iter_mut() {
        *vis = Visibility::Hidden;
    }
    ambient.brightness = 300.0;
}

fn enter_terrain3d(
    terrain_view: &Terrain3dViewState,
    map_camera: &mut Camera,
    terrain_camera: &mut Camera,
    terrain_transform: &mut Transform,
    terrain_projection: &mut Projection,
    ambient: &mut GlobalAmbientLight,
    light_q: &mut Query<&mut Visibility, With<TerrainLightTag>>,
) {
    enter_terrain3d_cameras_only(
        terrain_view,
        map_camera,
        terrain_camera,
        terrain_transform,
        terrain_projection,
    );
    for mut vis in light_q.iter_mut() {
        *vis = Visibility::Visible;
    }
    ambient.brightness = 100.0;
}

pub(in crate::map::terrain) fn enter_map2d_cameras_only(
    map_view: &Map2dViewState,
    map_camera: &mut Camera,
    map_transform: &mut Transform,
    map_projection: &mut Projection,
    terrain_camera: &mut Camera,
) {
    let activation = camera_activation_for_mode(ViewMode::Map2D);
    map_camera.is_active = activation.map2d_active;
    terrain_camera.is_active = activation.terrain3d_active;
    apply_map2d_camera_state(map_view, map_transform, map_projection);
}

pub(in crate::map::terrain) fn enter_terrain3d_cameras_only(
    terrain_view: &Terrain3dViewState,
    map_camera: &mut Camera,
    terrain_camera: &mut Camera,
    terrain_transform: &mut Transform,
    terrain_projection: &mut Projection,
) {
    let activation = camera_activation_for_mode(ViewMode::Terrain3D);
    map_camera.is_active = activation.map2d_active;
    terrain_camera.is_active = activation.terrain3d_active;
    ensure_terrain3d_projection(terrain_projection);
    apply_terrain3d_camera_state(terrain_view, terrain_transform);
}

pub(in crate::map::terrain) fn apply_mode_to_camera_and_lighting(
    mode: Res<ViewModeState>,
    map_view: Res<Map2dViewState>,
    terrain_view: Res<Terrain3dViewState>,
    mut applied_mode: ResMut<AppliedViewMode>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut map_camera_q: Query<
        (&mut Camera, &mut Transform, &mut Projection),
        (With<Map2dCamera>, Without<Terrain3dCamera>),
    >,
    mut terrain_camera_q: Query<
        (&mut Camera, &mut Transform, &mut Projection),
        (With<Terrain3dCamera>, Without<Map2dCamera>),
    >,
    mut light_q: Query<&mut Visibility, With<TerrainLightTag>>,
) {
    if applied_mode.mode == Some(mode.mode) {
        return;
    }

    let Ok((mut map_camera, mut map_transform, mut map_projection)) = map_camera_q.single_mut()
    else {
        return;
    };
    let Ok((mut terrain_camera, mut terrain_transform, mut terrain_projection)) =
        terrain_camera_q.single_mut()
    else {
        return;
    };

    match mode.mode {
        ViewMode::Map2D => enter_map2d(
            &map_view,
            &mut map_camera,
            &mut map_transform,
            &mut map_projection,
            &mut terrain_camera,
            &mut ambient,
            &mut light_q,
        ),
        ViewMode::Terrain3D => enter_terrain3d(
            &terrain_view,
            &mut map_camera,
            &mut terrain_camera,
            &mut terrain_transform,
            &mut terrain_projection,
            &mut ambient,
            &mut light_q,
        ),
    }
    applied_mode.mode = Some(mode.mode);
}

pub(in crate::map::terrain) fn log_camera_activation_state(
    mode: Res<ViewModeState>,
    applied_mode: Res<AppliedViewMode>,
    map_q: Query<(&Camera, &RenderLayers), With<Map2dCamera>>,
    terrain_q: Query<(&Camera, &RenderLayers), With<Terrain3dCamera>>,
    ui_q: Query<(&Camera, Option<&IsDefaultUiCamera>, &RenderLayers), With<UiCamera>>,
) {
    if !applied_mode.is_changed() {
        return;
    }

    let Ok((map_camera, map_layers)) = map_q.single() else {
        return;
    };
    let Ok((terrain_camera, terrain_layers)) = terrain_q.single() else {
        return;
    };
    let Ok((ui_camera, ui_default, ui_layers)) = ui_q.single() else {
        return;
    };
    let expected = camera_activation_for_mode(mode.mode);

    let active_world = if map_camera.is_active {
        "Map2dCamera"
    } else if terrain_camera.is_active {
        "Terrain3dCamera"
    } else {
        "none"
    };
    let ui_owner = if ui_default.is_some() {
        "UiCamera (IsDefaultUiCamera)"
    } else {
        "none"
    };

    bevy::log::info!(
        "view_mode={:?} world_active={} ui_owner={} | map(active={},order={},clear={:?},layers={:?}) terrain(active={},order={},clear={:?},layers={:?}) ui(active={},expected_ui_active={},order={},clear={:?},layers={:?})",
        mode.mode,
        active_world,
        ui_owner,
        map_camera.is_active,
        map_camera.order,
        map_camera.clear_color,
        map_layers,
        terrain_camera.is_active,
        terrain_camera.order,
        terrain_camera.clear_color,
        terrain_layers,
        ui_camera.is_active,
        expected.ui_active,
        ui_camera.order,
        ui_camera.clear_color,
        ui_layers
    );
}

#[cfg(debug_assertions)]
pub(in crate::map::terrain) fn debug_assert_camera_control_mode_gating(
    mode: Res<ViewModeState>,
    control_mutations: Res<CameraControlMutationFlags>,
) {
    match mode.mode {
        ViewMode::Map2D => {
            debug_assert!(
                !control_mutations.terrain3d_updated,
                "Terrain3D camera controls updated while mode was Map2D"
            );
        }
        ViewMode::Terrain3D => {
            debug_assert!(
                !control_mutations.map2d_updated,
                "Map2D camera controls updated while mode was Terrain3D"
            );
        }
    }
}

#[cfg(not(debug_assertions))]
pub(in crate::map::terrain) fn debug_assert_camera_control_mode_gating() {}

#[cfg(debug_assertions)]
pub(in crate::map::terrain) fn debug_assert_render_isolation(
    mode: Res<ViewModeState>,
    terrain_view: Res<Terrain3dViewState>,
    map_q: Query<(&Camera, &Transform), With<Map2dCamera>>,
    terrain_q: Query<(&Camera, &Transform), With<Terrain3dCamera>>,
    ui_camera_q: Query<&Camera, With<UiCamera>>,
    ui_default_q: Query<Entity, (With<Camera>, With<IsDefaultUiCamera>)>,
    world_2d_q: Query<&RenderLayers, With<World2dRenderEntity>>,
    world_3d_q: Query<&RenderLayers, With<World3dRenderEntity>>,
) {
    let Ok((map_camera, map_transform)) = map_q.single() else {
        return;
    };
    let Ok((terrain_camera, terrain_transform)) = terrain_q.single() else {
        return;
    };
    let Ok(ui_camera) = ui_camera_q.single() else {
        return;
    };

    let active_world_count =
        usize::from(map_camera.is_active) + usize::from(terrain_camera.is_active);
    debug_assert_eq!(
        active_world_count, 1,
        "exactly one world camera must be active (mode={:?})",
        mode.mode
    );
    debug_assert_eq!(
        map_camera.is_active,
        mode.mode == ViewMode::Map2D,
        "Map2dCamera active state diverged from ViewMode"
    );
    debug_assert_eq!(
        terrain_camera.is_active,
        mode.mode == ViewMode::Terrain3D,
        "Terrain3dCamera active state diverged from ViewMode"
    );
    if mode.mode == ViewMode::Map2D {
        let top_down_dot = map_transform.rotation.dot(Quat::IDENTITY).abs();
        debug_assert!(
            top_down_dot > 0.9999,
            "Map2dCamera rotation deviated from identity (dot={top_down_dot})"
        );
    }
    if mode.mode == ViewMode::Terrain3D {
        let to_pivot = terrain_view.pivot_world - terrain_transform.translation;
        if to_pivot.length_squared() > 1e-6 {
            let forward = (terrain_transform.rotation * -Vec3::Z).normalize_or_zero();
            let look_alignment = forward.dot(to_pivot.normalize_or_zero());
            debug_assert!(
                look_alignment > 0.999,
                "Terrain3dCamera stopped looking at pivot (alignment={look_alignment})"
            );
        }
    }

    let ui_owner_count = ui_default_q.iter().count();
    debug_assert_eq!(
        ui_owner_count, 1,
        "exactly one camera must own UI rendering via IsDefaultUiCamera"
    );
    debug_assert!(ui_camera.is_active, "UiCamera must remain active");

    let world_2d = world_2d_layers();
    let world_3d = world_3d_layers();
    for layers in &world_2d_q {
        debug_assert!(
            layers.intersects(&world_2d),
            "2D entity missing LAYER_WORLD_2D"
        );
        debug_assert!(
            !layers.intersects(&world_3d),
            "2D entity illegally intersects LAYER_WORLD_3D"
        );
    }
    for layers in &world_3d_q {
        debug_assert!(
            layers.intersects(&world_3d),
            "3D entity missing LAYER_WORLD_3D"
        );
        debug_assert!(
            !layers.intersects(&world_2d),
            "3D entity illegally intersects LAYER_WORLD_2D"
        );
    }
}

#[cfg(not(debug_assertions))]
pub(in crate::map::terrain) fn debug_assert_render_isolation() {}
