use std::collections::{HashMap, HashSet};

use fishystuff_api::models::events::EventPointCompact;

use crate::map::field_view::{FieldLayerView, LoadedFieldLayer};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::{MapPoint, WorldPoint};

pub const SAMPLE_RING_RADIUS_WORLD_UNITS: f64 = 500.0;
const SAMPLE_RING_STEP_MAP_PX: f64 = 0.25;
const SAMPLE_RING_MIN_STEPS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EventZoneFootprintKey {
    map_px_x: i32,
    map_px_y: i32,
    world_x: Option<i32>,
    world_z: Option<i32>,
}

pub struct EventZoneSetResolver<'a> {
    zone_mask_field: Option<LoadedFieldLayer<'a>>,
    cache: HashMap<EventZoneFootprintKey, Vec<u32>>,
}

impl<'a> EventZoneSetResolver<'a> {
    pub fn new(zone_mask_field: Option<LoadedFieldLayer<'a>>) -> Self {
        Self {
            zone_mask_field,
            cache: HashMap::new(),
        }
    }

    pub fn zone_rgbs(&mut self, event: &EventPointCompact) -> &[u32] {
        let key = EventZoneFootprintKey {
            map_px_x: event.map_px_x,
            map_px_y: event.map_px_y,
            world_x: event.world_x,
            world_z: event.world_z,
        };
        self.cache
            .entry(key)
            .or_insert_with(|| resolve_event_zone_rgbs(event, self.zone_mask_field))
            .as_slice()
    }
}

fn resolve_event_zone_rgbs(
    event: &EventPointCompact,
    zone_mask_field: Option<LoadedFieldLayer<'_>>,
) -> Vec<u32> {
    let Some(zone_mask_field) = zone_mask_field else {
        return event.zone_rgb_u32.into_iter().collect();
    };

    let map_to_world = MapToWorld::default();
    let center_world = event_center_world_point(event, map_to_world);
    let radius_map_px = SAMPLE_RING_RADIUS_WORLD_UNITS / map_to_world.distance_per_pixel.max(1.0);
    let circumference_map_px = std::f64::consts::TAU * radius_map_px;
    let steps = ((circumference_map_px / SAMPLE_RING_STEP_MAP_PX).ceil() as usize)
        .max(SAMPLE_RING_MIN_STEPS);

    let mut zones = HashSet::new();
    for step_idx in 0..steps {
        let theta = std::f64::consts::TAU * step_idx as f64 / steps as f64;
        let ring_world = WorldPoint::new(
            center_world.x + SAMPLE_RING_RADIUS_WORLD_UNITS * theta.cos(),
            center_world.z + SAMPLE_RING_RADIUS_WORLD_UNITS * theta.sin(),
        );
        let ring_map = map_to_world.world_to_map(ring_world);
        let map_px_x = ring_map.x.floor() as i32;
        let map_px_y = ring_map.y.floor() as i32;
        if let Some(zone_rgb) = zone_mask_field
            .field_id_at_map_px(map_px_x, map_px_y)
            .filter(|zone_rgb| *zone_rgb != 0)
        {
            zones.insert(zone_rgb);
        }
    }

    let mut zones = zones.into_iter().collect::<Vec<_>>();
    zones.sort_unstable();
    zones
}

fn event_center_world_point(event: &EventPointCompact, map_to_world: MapToWorld) -> WorldPoint {
    if let (Some(world_x), Some(world_z)) = (event.world_x, event.world_z) {
        return WorldPoint::new(world_x as f64, world_z as f64);
    }
    map_to_world.map_to_world(MapPoint::new(
        event.map_px_x as f64 + 0.5,
        event.map_px_y as f64 + 0.5,
    ))
}

#[cfg(test)]
mod tests {
    use fishystuff_api::models::events::EventPointCompact;
    use fishystuff_core::field::DiscreteFieldRows;

    use super::EventZoneSetResolver;
    use crate::map::field_view::LoadedFieldLayer;
    use crate::map::layers::FieldColorMode;

    #[test]
    fn ring_footprint_resolver_returns_multiple_touched_zones() {
        let field = DiscreteFieldRows::from_u32_grid(
            5,
            5,
            &[
                1, 1, 2, 2, 2, 1, 1, 2, 2, 2, 1, 1, 2, 2, 2, 1, 1, 2, 2, 2, 1, 1, 2, 2, 2,
            ],
        )
        .expect("field");
        let zone_mask_field = LoadedFieldLayer::from_parts(&field, FieldColorMode::RgbU24);
        let mut resolver = EventZoneSetResolver::new(Some(zone_mask_field));
        let event = EventPointCompact {
            event_id: 1,
            fish_id: 10,
            ts_utc: 100,
            map_px_x: 2,
            map_px_y: 2,
            length_milli: 1,
            world_x: None,
            world_z: None,
            zone_rgb_u32: Some(2),
            source_kind: None,
            source_id: None,
        };

        assert_eq!(resolver.zone_rgbs(&event), &[1, 2]);
    }

    #[test]
    fn resolver_falls_back_to_stored_single_zone_without_zone_mask_field() {
        let mut resolver = EventZoneSetResolver::new(None);
        let event = EventPointCompact {
            event_id: 1,
            fish_id: 10,
            ts_utc: 100,
            map_px_x: 2,
            map_px_y: 2,
            length_milli: 1,
            world_x: None,
            world_z: None,
            zone_rgb_u32: Some(2),
            source_kind: None,
            source_id: None,
        };

        assert_eq!(resolver.zone_rgbs(&event), &[2]);
    }
}
