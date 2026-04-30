use bevy::prelude::*;

use crate::map::spaces::WorldPoint;
use crate::plugins::api::{MapDisplayState, PointSampleSummary};
use crate::plugins::camera::Map2dCamera;

use super::query::{PointsState, RenderPoint};
use super::render::{map_point_to_world, point_icon_world_size, ring_style_for_point};

const POINT_HIT_PADDING_WORLD_UNITS: f64 = 120.0;
const POINT_HOVER_SAMPLE_LIMIT: usize = 5;

pub fn point_samples_at_world_point(
    world_point: WorldPoint,
    points: &PointsState,
    display_state: &MapDisplayState,
    camera_q: &Query<'_, '_, &'static Projection, With<Map2dCamera>>,
) -> Vec<PointSampleSummary> {
    let hits = point_hits_at_world_point(world_point, points, display_state, camera_q);
    let Some(best) = hits.first() else {
        return Vec::new();
    };
    if best.point.aggregated {
        return best.point.point_samples.clone();
    }
    exact_raw_point_samples(best.point, &hits)
}

pub fn point_hover_samples_at_world_point(
    world_point: WorldPoint,
    points: &PointsState,
    display_state: &MapDisplayState,
    camera_q: &Query<'_, '_, &'static Projection, With<Map2dCamera>>,
) -> Vec<PointSampleSummary> {
    let hits = point_hits_at_world_point(world_point, points, display_state, camera_q);
    let Some(best) = hits.first() else {
        return Vec::new();
    };
    let samples = if best.point.aggregated {
        best.point.point_samples.clone()
    } else {
        exact_raw_point_samples(best.point, &hits)
    };
    summarize_hover_samples(&samples)
}

#[derive(Debug, Clone, Copy)]
struct PointHit<'a> {
    point: &'a RenderPoint,
    distance_sq: f64,
}

fn point_hits_at_world_point<'a>(
    world_point: WorldPoint,
    points: &'a PointsState,
    display_state: &MapDisplayState,
    camera_q: &Query<'_, '_, &'static Projection, With<Map2dCamera>>,
) -> Vec<PointHit<'a>> {
    if !display_state.show_points || points.points.is_empty() {
        return Vec::new();
    }

    let icon_size_world_units = point_icon_world_size(display_state, camera_q);
    let mut hits = Vec::new();
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
        hits.push(PointHit { point, distance_sq });
    }
    hits.sort_by(|left, right| {
        left.distance_sq
            .total_cmp(&right.distance_sq)
            .then_with(|| right.point.sample_count.cmp(&left.point.sample_count))
            .then_with(|| right.point.aggregated.cmp(&left.point.aggregated))
            .then_with(|| left.point.map_px_x.cmp(&right.point.map_px_x))
            .then_with(|| left.point.map_px_y.cmp(&right.point.map_px_y))
    });
    hits
}

fn exact_raw_point_samples(
    best_point: &RenderPoint,
    hits: &[PointHit<'_>],
) -> Vec<PointSampleSummary> {
    let mut samples = Vec::new();
    for hit in hits {
        let point = hit.point;
        if point.aggregated {
            continue;
        }
        if point.map_px_x != best_point.map_px_x || point.map_px_y != best_point.map_px_y {
            continue;
        }
        samples.extend(point.point_samples.iter().cloned());
    }
    samples.sort_by(|left, right| {
        right
            .last_ts_utc
            .cmp(&left.last_ts_utc)
            .then_with(|| left.fish_id.cmp(&right.fish_id))
            .then_with(|| left.sample_id.cmp(&right.sample_id))
    });
    samples
}

fn summarize_hover_samples(samples: &[PointSampleSummary]) -> Vec<PointSampleSummary> {
    let mut summaries: Vec<PointSampleSummary> = Vec::new();
    for sample in samples {
        if let Some(current) = summaries
            .iter_mut()
            .find(|current| current.fish_id == sample.fish_id)
        {
            current.sample_count = current
                .sample_count
                .saturating_add(sample.sample_count.max(1));
            current.last_ts_utc = current.last_ts_utc.max(sample.last_ts_utc);
            merge_zone_rgbs(&mut current.zone_rgbs, &sample.zone_rgbs);
            merge_zone_rgbs(&mut current.full_zone_rgbs, &sample.full_zone_rgbs);
        } else {
            summaries.push(PointSampleSummary {
                fish_id: sample.fish_id,
                sample_count: sample.sample_count.max(1),
                last_ts_utc: sample.last_ts_utc,
                sample_id: None,
                zone_rgbs: sample.zone_rgbs.clone(),
                full_zone_rgbs: sample.full_zone_rgbs.clone(),
            });
        }
    }
    summaries.sort_by(|left, right| {
        right
            .sample_count
            .cmp(&left.sample_count)
            .then_with(|| right.last_ts_utc.cmp(&left.last_ts_utc))
            .then_with(|| left.fish_id.cmp(&right.fish_id))
            .then_with(|| left.zone_rgbs.cmp(&right.zone_rgbs))
            .then_with(|| left.full_zone_rgbs.cmp(&right.full_zone_rgbs))
    });
    summaries.truncate(POINT_HOVER_SAMPLE_LIMIT);
    summaries
}

fn merge_zone_rgbs(target: &mut Vec<u32>, source: &[u32]) {
    target.extend(source.iter().copied());
    target.sort_unstable();
    target.dedup();
}

#[cfg(test)]
mod tests {
    use super::{exact_raw_point_samples, summarize_hover_samples, PointHit};
    use crate::plugins::api::PointSampleSummary;
    use crate::plugins::points::query::RenderPoint;

    fn sample(event_id: i64, fish_id: i32, ts_utc: i64) -> PointSampleSummary {
        PointSampleSummary {
            fish_id,
            sample_count: 1,
            last_ts_utc: ts_utc,
            sample_id: Some(event_id),
            zone_rgbs: vec![0x39e58d],
            full_zone_rgbs: vec![0x39e58d],
        }
    }

    fn raw_point(map_px_x: i32, map_px_y: i32, sample: PointSampleSummary) -> RenderPoint {
        RenderPoint {
            map_px_x,
            map_px_y,
            world_x: None,
            world_z: None,
            fish_id: Some(sample.fish_id),
            zone_rgb_u32: None,
            sample_count: 1,
            aggregated: false,
            point_samples: vec![sample],
        }
    }

    #[test]
    fn exact_raw_point_samples_keeps_overlapping_same_pixel_samples() {
        let first = raw_point(2418, 2740, sample(1674, 116, 1_629_098_820));
        let second = raw_point(2418, 2740, sample(41755, 116, 1_629_098_858));
        let nearby = raw_point(2419, 2740, sample(99, 116, 1_629_098_900));
        let hits = vec![
            PointHit {
                point: &first,
                distance_sq: 0.0,
            },
            PointHit {
                point: &second,
                distance_sq: 0.0,
            },
            PointHit {
                point: &nearby,
                distance_sq: 1.0,
            },
        ];

        let samples = exact_raw_point_samples(&first, &hits);

        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].sample_id, Some(41755));
        assert_eq!(samples[1].sample_id, Some(1674));
    }

    #[test]
    fn summarize_hover_samples_caps_rows_and_omits_sample_ids() {
        let samples = vec![
            sample(1, 10, 100),
            sample(2, 10, 120),
            sample(3, 20, 130),
            sample(4, 30, 140),
            sample(5, 40, 150),
            sample(6, 50, 160),
            sample(7, 60, 170),
        ];

        let summaries = summarize_hover_samples(&samples);

        assert_eq!(summaries.len(), 5);
        assert_eq!(summaries[0].fish_id, 10);
        assert_eq!(summaries[0].sample_count, 2);
        assert_eq!(summaries[0].last_ts_utc, 120);
        assert!(summaries.iter().all(|sample| sample.sample_id.is_none()));
    }
}
