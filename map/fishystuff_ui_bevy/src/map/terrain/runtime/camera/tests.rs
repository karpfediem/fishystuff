use crate::map::camera::map2d::{apply_map2d_camera_state, reset_map2d_view, Map2dViewState};
use crate::map::camera::mode::ViewMode;
use crate::map::camera::terrain3d::{
    apply_terrain3d_camera_state, reset_terrain3d_view, Terrain3dViewState,
};
use crate::map::spaces::world::MapToWorld;
use crate::map::terrain::mode::{
    camera_activation_for_mode, ensure_terrain3d_projection, enter_map2d_cameras_only,
    enter_terrain3d_cameras_only, terrain3d_controls_should_run, CameraActivationState,
};
use bevy::prelude::{
    Camera, OrthographicProjection, PerspectiveProjection, Projection, Quat, Transform, Vec3,
};

fn alpha_range(rgba: &[u8]) -> (u8, u8) {
    let mut min_alpha = u8::MAX;
    let mut max_alpha = u8::MIN;
    let mut saw_alpha = false;
    for alpha in rgba.chunks_exact(4).map(|pixel| pixel[3]) {
        min_alpha = min_alpha.min(alpha);
        max_alpha = max_alpha.max(alpha);
        saw_alpha = true;
    }
    if saw_alpha {
        (min_alpha, max_alpha)
    } else {
        (0, 0)
    }
}

#[test]
fn camera_activation_matches_view_mode() {
    assert_eq!(
        camera_activation_for_mode(ViewMode::Map2D),
        CameraActivationState {
            map2d_active: true,
            terrain3d_active: false,
            ui_active: true,
        }
    );
    assert_eq!(
        camera_activation_for_mode(ViewMode::Terrain3D),
        CameraActivationState {
            map2d_active: false,
            terrain3d_active: true,
            ui_active: true,
        }
    );
}

#[test]
fn terrain3d_controls_require_terrain_mode_and_active_camera() {
    assert!(terrain3d_controls_should_run(ViewMode::Terrain3D, true));
    assert!(!terrain3d_controls_should_run(ViewMode::Terrain3D, false));
    assert!(!terrain3d_controls_should_run(ViewMode::Map2D, true));
    assert!(!terrain3d_controls_should_run(ViewMode::Map2D, false));
}

#[test]
fn apply_map2d_camera_state_restores_flat_camera() {
    let map_state = Map2dViewState {
        center_world_x: 1234.0,
        center_world_z: -987.0,
        zoom: 2.75,
    };
    let mut transform =
        Transform::from_translation(Vec3::new(10.0, 20.0, 30.0)).looking_at(Vec3::Y, Vec3::X);
    let mut projection = Projection::Perspective(PerspectiveProjection::default());

    apply_map2d_camera_state(&map_state, &mut transform, &mut projection);

    assert_eq!(
        transform.translation,
        Vec3::new(map_state.center_world_x, map_state.center_world_z, 1000.0)
    );
    assert_eq!(transform.rotation, Quat::IDENTITY);
    match projection {
        Projection::Orthographic(ortho) => {
            assert!((ortho.scale - map_state.zoom).abs() < 1e-5);
        }
        _ => panic!("map2d projection must be orthographic"),
    }
}

#[test]
fn apply_terrain3d_camera_state_restores_orbit_camera() {
    let view = Terrain3dViewState {
        pivot_world: Vec3::new(100.0, 15.0, 200.0),
        yaw: 0.45,
        pitch: -0.6,
        distance: 12_500.0,
    };
    let mut transform = Transform::default();
    let mut projection = Projection::Orthographic(OrthographicProjection::default_2d());

    ensure_terrain3d_projection(&mut projection);
    apply_terrain3d_camera_state(&view, &mut transform);

    let expected = view.camera_transform();
    assert!((transform.translation - expected.translation).length() < 1e-3);
    let forward = (transform.rotation * -Vec3::Z).normalize_or_zero();
    let to_pivot = (view.pivot_world - transform.translation).normalize_or_zero();
    assert!(forward.dot(to_pivot) > 0.999);
    match projection {
        Projection::Perspective(perspective) => {
            assert!(perspective.far > 1_000_000.0);
            assert!(perspective.near <= 1.0);
        }
        _ => panic!("terrain3d projection must be perspective"),
    }
}

#[test]
fn mode_switching_2d_3d_2d_restores_original_2d_view() {
    let map_state = Map2dViewState {
        center_world_x: 333.0,
        center_world_z: 777.0,
        zoom: 1.5,
    };
    let terrain_state = Terrain3dViewState {
        pivot_world: Vec3::new(20.0, 50.0, 80.0),
        yaw: -0.9,
        pitch: -0.4,
        distance: 42_000.0,
    };

    let mut map_camera = Camera::default();
    let mut terrain_camera = Camera::default();
    let mut map_transform = Transform::from_translation(Vec3::new(9.0, 8.0, 7.0))
        .looking_at(Vec3::new(1.0, 2.0, 3.0), Vec3::Y);
    let mut map_projection = Projection::Perspective(PerspectiveProjection::default());
    let mut terrain_transform = Transform::default();
    let mut terrain_projection = Projection::Orthographic(OrthographicProjection::default_2d());

    enter_map2d_cameras_only(
        &map_state,
        &mut map_camera,
        &mut map_transform,
        &mut map_projection,
        &mut terrain_camera,
    );
    map_transform.rotation = Quat::from_rotation_z(0.7);
    map_transform.translation = Vec3::new(-100.0, -200.0, 9999.0);
    enter_terrain3d_cameras_only(
        &terrain_state,
        &mut map_camera,
        &mut terrain_camera,
        &mut terrain_transform,
        &mut terrain_projection,
    );
    enter_map2d_cameras_only(
        &map_state,
        &mut map_camera,
        &mut map_transform,
        &mut map_projection,
        &mut terrain_camera,
    );

    assert!(map_camera.is_active);
    assert!(!terrain_camera.is_active);
    assert_eq!(
        map_transform.translation,
        Vec3::new(map_state.center_world_x, map_state.center_world_z, 1000.0)
    );
    assert_eq!(map_transform.rotation, Quat::IDENTITY);
    match map_projection {
        Projection::Orthographic(ortho) => assert!((ortho.scale - map_state.zoom).abs() < 1e-5),
        _ => panic!("map2d projection must stay orthographic"),
    }
}

#[test]
fn mode_switching_3d_2d_3d_restores_original_3d_view() {
    let map_state = Map2dViewState {
        center_world_x: -500.0,
        center_world_z: 125.0,
        zoom: 2.2,
    };
    let terrain_state = Terrain3dViewState {
        pivot_world: Vec3::new(400.0, 75.0, -150.0),
        yaw: 0.85,
        pitch: -0.45,
        distance: 25_000.0,
    };

    let mut map_camera = Camera::default();
    let mut terrain_camera = Camera::default();
    let mut map_transform = Transform::default();
    let mut map_projection = Projection::Orthographic(OrthographicProjection::default_2d());
    let mut terrain_transform = Transform::from_translation(Vec3::new(1.0, 2.0, 3.0));
    let mut terrain_projection = Projection::Orthographic(OrthographicProjection::default_2d());

    enter_terrain3d_cameras_only(
        &terrain_state,
        &mut map_camera,
        &mut terrain_camera,
        &mut terrain_transform,
        &mut terrain_projection,
    );
    terrain_transform.rotation = Quat::IDENTITY;
    terrain_transform.translation = Vec3::new(999.0, 888.0, 777.0);
    enter_map2d_cameras_only(
        &map_state,
        &mut map_camera,
        &mut map_transform,
        &mut map_projection,
        &mut terrain_camera,
    );
    enter_terrain3d_cameras_only(
        &terrain_state,
        &mut map_camera,
        &mut terrain_camera,
        &mut terrain_transform,
        &mut terrain_projection,
    );

    assert!(!map_camera.is_active);
    assert!(terrain_camera.is_active);
    let expected = terrain_state.camera_transform();
    assert!((terrain_transform.translation - expected.translation).length() < 1e-3);
    let forward = (terrain_transform.rotation * -Vec3::Z).normalize_or_zero();
    let to_pivot = (terrain_state.pivot_world - terrain_transform.translation).normalize_or_zero();
    assert!(forward.dot(to_pivot) > 0.999);
    match terrain_projection {
        Projection::Perspective(_) => {}
        _ => panic!("terrain3d projection must stay perspective"),
    }
}

#[test]
fn reset_map2d_view_returns_valid_defaults() {
    let mut view = Map2dViewState {
        center_world_x: 999.0,
        center_world_z: 999.0,
        zoom: 99.0,
    };
    reset_map2d_view(&mut view);
    let bounds = MapToWorld::default().world_bounds();
    let expected_x = ((bounds.min.x + bounds.max.x) * 0.5) as f32;
    let expected_z = ((bounds.min.z + bounds.max.z) * 0.5) as f32;
    assert!((view.center_world_x - expected_x).abs() < 1e-4);
    assert!((view.center_world_z - expected_z).abs() < 1e-4);
    assert!((view.zoom - 1.0).abs() < 1e-6);
    assert!(view.center_world_x.is_finite());
    assert!(view.center_world_z.is_finite());
    assert!(view.zoom > 0.0);
}

#[test]
fn reset_terrain3d_view_returns_valid_defaults() {
    let mut view = Terrain3dViewState {
        pivot_world: Vec3::new(1.0, 2.0, 3.0),
        yaw: 2.0,
        pitch: 1.0,
        distance: 123.0,
    };
    reset_terrain3d_view(&mut view);
    assert!(view.pivot_world.x.is_finite());
    assert!(view.pivot_world.z.is_finite());
    assert!(view.distance > 0.0);
}

#[test]
fn alpha_range_reports_expected_bounds() {
    let rgba = vec![0, 0, 0, 0, 10, 20, 30, 200, 255, 255, 255, 255];
    assert_eq!(alpha_range(&rgba), (0, 255));
}
