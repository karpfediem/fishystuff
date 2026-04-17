use std::collections::HashMap;

use fishystuff_api::models::events::EventPointCompact;

pub const SAMPLE_RING_RADIUS_WORLD_UNITS: f64 = 500.0;

pub struct EventZoneSetResolver {
    cache: HashMap<i64, Vec<u32>>,
}

impl EventZoneSetResolver {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn zone_rgbs(&mut self, event: &EventPointCompact) -> &[u32] {
        self.cache
            .entry(event.event_id)
            .or_insert_with(|| resolve_event_zone_rgbs(event))
            .as_slice()
    }
}

fn resolve_event_zone_rgbs(event: &EventPointCompact) -> Vec<u32> {
    if !event.zone_rgbs.is_empty() {
        let mut zones = event.zone_rgbs.clone();
        zones.sort_unstable();
        zones.dedup();
        return zones;
    }
    event.zone_rgb_u32.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use fishystuff_api::models::events::EventPointCompact;

    use super::EventZoneSetResolver;

    #[test]
    fn resolver_prefers_stored_zone_support_set() {
        let mut resolver = EventZoneSetResolver::new();
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
            zone_rgbs: vec![2, 1, 2],
            source_kind: None,
            source_id: None,
        };

        assert_eq!(resolver.zone_rgbs(&event), &[1, 2]);
    }

    #[test]
    fn resolver_falls_back_to_stored_single_zone() {
        let mut resolver = EventZoneSetResolver::new();
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
            zone_rgbs: Vec::new(),
            source_kind: None,
            source_id: None,
        };

        assert_eq!(resolver.zone_rgbs(&event), &[2]);
    }
}
