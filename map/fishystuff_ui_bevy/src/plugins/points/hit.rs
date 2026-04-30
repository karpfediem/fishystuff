use bevy::prelude::*;

use crate::map::spaces::WorldPoint;
use crate::plugins::api::MapDisplayState;
use crate::plugins::camera::Map2dCamera;

use super::query::{PointsState, RenderPoint};
use super::render::{map_point_to_world, point_icon_world_size, ring_style_for_point};

const POINT_HIT_PADDING_WORLD_UNITS: f64 = 120.0;

pub fn point_at_world_point<'a>(
    world_point: WorldPoint,
    points: &'a PointsState,
    display_state: &MapDisplayState,
    camera_q: &Query<'_, '_, &'static Projection, With<Map2dCamera>>,
) -> Option<&'a RenderPoint> {
    if !display_state.show_points || points.points.is_empty() {
        return None;
    }

    let icon_size_world_units = point_icon_world_size(display_state, camera_q);
    let mut best: Option<(&RenderPoint, f64)> = None;
    for point in &points.points {
        if point.point_samples.is_empty() {
            continue;
        }
        let point_world = map_point_to_world(point);
        let dx = world_point.x - point_world.x;
        let dz = world_point.z - point_world.z;
        let distance_sq = dx * dx + dz * dz;
        let (ring_scale, _) = ring_style_for_point(point);
        let ring_diameter_world = super::render::RING_RADIUS_GAME_UNITS * 2.0 * ring_scale;
        let hit_radius = f64::from(icon_size_world_units.max(ring_diameter_world)) * 0.5
            + POINT_HIT_PADDING_WORLD_UNITS;
        if distance_sq > hit_radius * hit_radius {
            continue;
        }
        match best {
            None => best = Some((point, distance_sq)),
            Some((best_point, best_distance_sq)) => {
                if distance_sq < best_distance_sq
                    || (distance_sq == best_distance_sq
                        && point.sample_count > best_point.sample_count)
                {
                    best = Some((point, distance_sq));
                }
            }
        }
    }
    best.map(|(point, _)| point)
}
