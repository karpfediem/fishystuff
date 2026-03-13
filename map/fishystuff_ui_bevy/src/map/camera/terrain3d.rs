use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::{WorldPoint, WorldRect};
use crate::prelude::*;

// Keep orbit azimuth conventional and apply explicit camera-space mirroring for parity with 2D.
const DEFAULT_YAW: f32 = 0.0;
const DEFAULT_PITCH: f32 = -0.58;
const MIRROR_CAMERA_X: bool = true;
const MIN_PITCH: f32 = -1.48;
const MAX_PITCH: f32 = 1.48;
const MIN_DISTANCE: f32 = 2000.0;
const MAX_DISTANCE: f32 = 900_000.0;

#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct Terrain3dViewState {
    pub pivot_world: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Default for Terrain3dViewState {
    fn default() -> Self {
        reset_for_world_bounds(MapToWorld::default().world_bounds())
    }
}

impl Terrain3dViewState {
    pub fn camera_position(self) -> Vec3 {
        self.camera_transform().translation
    }

    pub fn camera_transform(self) -> Transform {
        let mut transform = Transform::default();
        apply_terrain3d_camera_state(&self, &mut transform);
        transform
    }

    pub fn orbit(&mut self, delta: Vec2, sensitivity: f32) {
        self.yaw -= delta.x * sensitivity;
        self.pitch = (self.pitch - delta.y * sensitivity).clamp(MIN_PITCH, MAX_PITCH);
    }

    pub fn pan(&mut self, delta: Vec2, viewport_size: Vec2, fov_y_radians: f32) {
        let view_h = viewport_size.y.max(1.0);
        let units_per_px = (2.0 * self.distance * (0.5 * fov_y_radians).tan()) / view_h;

        let transform = self.camera_transform();
        let right = transform.right();
        let up = transform.up();
        let delta_world = (-delta.x * right + delta.y * up) * units_per_px;
        self.pivot_world += delta_world;
    }

    pub fn dolly(&mut self, delta: f32, speed: f32) {
        let factor = (1.0 - delta * speed).clamp(0.1, 10.0);
        self.distance = (self.distance * factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }

    pub fn set_distance_clamped(&mut self, distance: f32) {
        self.distance = distance.clamp(MIN_DISTANCE, MAX_DISTANCE);
    }
}

pub fn apply_terrain3d_camera_state(state: &Terrain3dViewState, transform: &mut Transform) {
    let cp = state.pitch.cos();
    let sp = state.pitch.sin();
    let cy = state.yaw.cos();
    let sy = state.yaw.sin();
    let forward = Vec3::new(cp * sy, sp, cp * cy).normalize_or_zero();
    let camera_position = state.pivot_world - forward * state.distance;
    let mut orbit =
        Transform::from_translation(camera_position).looking_at(state.pivot_world, Vec3::Y);
    if MIRROR_CAMERA_X {
        orbit.scale = Vec3::new(-1.0, 1.0, 1.0);
    }
    *transform = orbit;
}

pub fn camera_controls_x_mirrored() -> bool {
    MIRROR_CAMERA_X
}

pub fn reset_for_world_bounds(bounds: WorldRect) -> Terrain3dViewState {
    let center = Vec3::new(
        ((bounds.min.x + bounds.max.x) * 0.5) as f32,
        0.0,
        ((bounds.min.z + bounds.max.z) * 0.5) as f32,
    );
    let span_x = (bounds.max.x - bounds.min.x) as f32;
    let span_z = (bounds.max.z - bounds.min.z) as f32;
    let span = span_x.max(span_z).max(1.0);
    let distance = (span * 1.35).clamp(MIN_DISTANCE, MAX_DISTANCE);
    Terrain3dViewState {
        pivot_world: center,
        yaw: DEFAULT_YAW,
        pitch: DEFAULT_PITCH,
        distance,
    }
}

pub fn reset_terrain3d_view(state: &mut Terrain3dViewState) {
    *state = reset_for_world_bounds(MapToWorld::default().world_bounds());
}

pub fn estimate_view_world_rect(state: Terrain3dViewState, window_size: Vec2) -> WorldRect {
    let aspect = if window_size.y > 1.0 {
        (window_size.x / window_size.y).clamp(0.4, 3.0)
    } else {
        1.6
    };
    let half_h = state.distance * 0.8;
    let half_w = half_h * aspect;
    WorldRect {
        min: WorldPoint::new(
            (state.pivot_world.x - half_w) as f64,
            (state.pivot_world.z - half_h) as f64,
        ),
        max: WorldPoint::new(
            (state.pivot_world.x + half_w) as f64,
            (state.pivot_world.z + half_h) as f64,
        ),
    }
}

pub type OrbitCameraState = Terrain3dViewState;

#[cfg(test)]
mod tests {
    use super::{apply_terrain3d_camera_state, reset_for_world_bounds, Terrain3dViewState};
    use crate::map::spaces::{WorldPoint, WorldRect};

    #[test]
    fn orbit_camera_preserves_distance() {
        let mut state = Terrain3dViewState {
            pivot_world: bevy::math::Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            distance: 1000.0,
        };
        let before = state.camera_position().distance(state.pivot_world);
        state.orbit(bevy::math::Vec2::new(20.0, -10.0), 0.01);
        let after = state.camera_position().distance(state.pivot_world);
        assert!((after - before).abs() < 1e-3);
    }

    #[test]
    fn dolly_is_clamped() {
        let mut state = Terrain3dViewState {
            pivot_world: bevy::math::Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            distance: 200.0,
        };
        state.dolly(100.0, 0.2);
        assert!(state.distance >= 2000.0);
    }

    #[test]
    fn orbit_pitch_is_clamped() {
        let mut state = Terrain3dViewState {
            pivot_world: bevy::math::Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            distance: 10_000.0,
        };
        state.orbit(bevy::math::Vec2::new(0.0, -100_000.0), 0.01);
        assert!(state.pitch <= 1.48);
        state.orbit(bevy::math::Vec2::new(0.0, 100_000.0), 0.01);
        assert!(state.pitch >= -1.48);
    }

    #[test]
    fn camera_transform_looks_at_pivot() {
        let state = Terrain3dViewState {
            pivot_world: bevy::math::Vec3::new(120.0, 80.0, -400.0),
            yaw: 0.9,
            pitch: -0.7,
            distance: 24_000.0,
        };
        let transform = state.camera_transform();
        let forward = (transform.rotation * -bevy::math::Vec3::Z).normalize_or_zero();
        let to_pivot = (state.pivot_world - transform.translation).normalize_or_zero();
        assert!(forward.dot(to_pivot) > 0.999);
    }

    #[test]
    fn apply_function_looks_at_pivot() {
        let state = Terrain3dViewState {
            pivot_world: bevy::math::Vec3::new(-50.0, 20.0, 300.0),
            yaw: -0.4,
            pitch: 0.35,
            distance: 12_000.0,
        };
        let mut transform = bevy::prelude::Transform::default();
        apply_terrain3d_camera_state(&state, &mut transform);
        let forward = (transform.rotation * -bevy::math::Vec3::Z).normalize_or_zero();
        let to_pivot = (state.pivot_world - transform.translation).normalize_or_zero();
        assert!(forward.dot(to_pivot) > 0.999);
    }

    #[test]
    fn reset_frames_bounds() {
        let bounds = WorldRect {
            min: WorldPoint::new(-1000.0, -2000.0),
            max: WorldPoint::new(3000.0, 2000.0),
        };
        let state = reset_for_world_bounds(bounds);
        assert!(state.distance > 1000.0);
        assert!(state.pitch < 0.0);
    }
}
