use bevy::prelude::*;
use std::collections::HashMap;

use crate::map::events::cluster::{
    point_sample_summary_for_event, sort_point_samples_by_cluster_order,
};
use crate::map::events::EventsSnapshotState;
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
    snapshot: &EventsSnapshotState,
    display_state: &MapDisplayState,
    camera_q: &Query<'_, '_, &'static Projection, With<Map2dCamera>>,
) -> Vec<PointSampleSummary> {
    let hits = point_hits_at_world_point(world_point, points, display_state, camera_q);
    let Some(best) = hits.first() else {
        return Vec::new();
    };
    if best.point.aggregated {
        return aggregated_point_samples(best.point, snapshot);
    }
    exact_raw_point_samples(best.point, &hits, snapshot)
}

pub fn point_hover_samples_at_world_point(
    world_point: WorldPoint,
    points: &PointsState,
    snapshot: &EventsSnapshotState,
    display_state: &MapDisplayState,
    camera_q: &Query<'_, '_, &'static Projection, With<Map2dCamera>>,
) -> Vec<PointSampleSummary> {
    let hits = point_hits_at_world_point(world_point, points, display_state, camera_q);
    let Some(best) = hits.first() else {
        return Vec::new();
    };
    if best.point.aggregated {
        if !best.point.point_samples.is_empty() {
            return summarize_hover_samples(&best.point.point_samples);
        }
        return summarize_hover_event_indices(&best.point.event_indices, snapshot);
    }
    let samples = exact_raw_point_samples(best.point, &hits, snapshot);
    summarize_hover_samples(&samples)
}

fn aggregated_point_samples(
    point: &RenderPoint,
    snapshot: &EventsSnapshotState,
) -> Vec<PointSampleSummary> {
    if !point.point_samples.is_empty() {
        return point.point_samples.clone();
    }
    let mut fish_counts: HashMap<i32, u32> = HashMap::new();
    let mut samples = Vec::with_capacity(point.event_indices.len());
    for idx in &point.event_indices {
        let Some(event) = snapshot.events.get(*idx) else {
            continue;
        };
        *fish_counts.entry(event.fish_id).or_insert(0) += 1;
        samples.push(point_sample_summary_for_event(event));
    }
    sort_point_samples_by_cluster_order(&mut samples, &fish_counts);
    samples
}

fn samples_for_point(
    point: &RenderPoint,
    snapshot: &EventsSnapshotState,
) -> Vec<PointSampleSummary> {
    if !point.point_samples.is_empty() {
        return point.point_samples.clone();
    }
    let mut samples = Vec::with_capacity(point.event_indices.len());
    for idx in &point.event_indices {
        let Some(event) = snapshot.events.get(*idx) else {
            continue;
        };
        samples.push(point_sample_summary_for_event(event));
    }
    samples
}

fn summarize_hover_event_indices(
    event_indices: &[usize],
    snapshot: &EventsSnapshotState,
) -> Vec<PointSampleSummary> {
    let mut summaries: HashMap<i32, PointSampleSummary> = HashMap::new();
    for idx in event_indices {
        let Some(event) = snapshot.events.get(*idx) else {
            continue;
        };
        let sample = point_sample_summary_for_event(event);
        if let Some(current) = summaries.get_mut(&sample.fish_id) {
            current.sample_count = current.sample_count.saturating_add(1);
            current.last_ts_utc = current.last_ts_utc.max(sample.last_ts_utc);
            merge_zone_rgbs(&mut current.zone_rgbs, &sample.zone_rgbs);
            merge_zone_rgbs(&mut current.full_zone_rgbs, &sample.full_zone_rgbs);
        } else {
            summaries.insert(
                sample.fish_id,
                PointSampleSummary {
                    sample_id: None,
                    ..sample
                },
            );
        }
    }
    let mut summaries = summaries.into_values().collect::<Vec<_>>();
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
        if point.point_samples.is_empty() && point.event_indices.is_empty() {
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
    snapshot: &EventsSnapshotState,
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
        samples.extend(samples_for_point(point, snapshot));
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
    use fishystuff_api::models::events::EventPointCompact;

    use super::{
        aggregated_point_samples, exact_raw_point_samples, summarize_hover_event_indices,
        summarize_hover_samples, PointHit,
    };
    use crate::map::events::EventsSnapshotState;
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
            event_indices: Vec::new(),
            point_samples: vec![sample],
        }
    }

    fn event(
        event_id: i64,
        fish_id: i32,
        ts_utc: i64,
        zone_rgbs: Vec<u32>,
        full_zone_rgbs: Vec<u32>,
    ) -> EventPointCompact {
        EventPointCompact {
            event_id,
            fish_id,
            ts_utc,
            map_px_x: 10,
            map_px_y: 20,
            length_milli: 1000,
            world_x: None,
            world_z: None,
            zone_rgb_u32: None,
            zone_rgbs,
            full_zone_rgbs,
            source_kind: None,
            source_id: None,
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

        let snapshot = EventsSnapshotState::default();
        let samples = exact_raw_point_samples(&first, &hits, &snapshot);

        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].sample_id, Some(41755));
        assert_eq!(samples[1].sample_id, Some(1674));
    }

    #[test]
    fn aggregated_point_samples_expand_lazily_from_snapshot() {
        let snapshot = EventsSnapshotState {
            events: vec![
                event(1, 10, 100, vec![0x39e58d], vec![0x39e58d]),
                event(2, 20, 130, vec![0x123456], vec![0x123456]),
                event(3, 10, 140, vec![0x39e58d, 0x654321], Vec::new()),
                event(4, 30, 160, vec![0x0abcde], vec![0x0abcde]),
            ],
            ..Default::default()
        };
        let point = RenderPoint {
            map_px_x: 10,
            map_px_y: 20,
            world_x: None,
            world_z: None,
            fish_id: Some(10),
            zone_rgb_u32: None,
            sample_count: 4,
            aggregated: true,
            event_indices: vec![0, 1, 2, 3],
            point_samples: Vec::new(),
        };

        let samples = aggregated_point_samples(&point, &snapshot);
        let hover = summarize_hover_event_indices(&point.event_indices, &snapshot);

        assert_eq!(samples.len(), 4);
        assert_eq!(samples[0].fish_id, 10);
        assert_eq!(samples[0].sample_id, Some(3));
        assert_eq!(samples[0].zone_rgbs, vec![0x39e58d, 0x654321]);
        assert!(samples[0].full_zone_rgbs.is_empty());
        assert_eq!(samples[1].fish_id, 10);
        assert_eq!(samples[1].sample_id, Some(1));
        assert_eq!(hover[0].fish_id, 10);
        assert_eq!(hover[0].sample_count, 2);
        assert_eq!(hover[0].sample_id, None);
        assert_eq!(hover[0].zone_rgbs, vec![0x39e58d, 0x654321]);
        assert_eq!(hover[0].full_zone_rgbs, vec![0x39e58d]);
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
