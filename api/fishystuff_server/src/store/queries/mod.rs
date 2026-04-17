pub const PATCHES_SQL: &str =
    "SELECT patch_id, start_ts_utc, patch_name FROM `patches` ORDER BY start_ts_utc";

pub const MAP_VERSIONS_SQL: &str =
    "SELECT map_version_id, name, is_default FROM `map_versions` ORDER BY map_version_id";

pub const REGION_GROUP_META_SQL: &str = "
SELECT
  map_version_id,
  region_group_id,
  color_rgb_u32,
  feature_count,
  region_count,
  accessible_region_count,
  bbox_min_x,
  bbox_min_y,
  bbox_max_x,
  bbox_max_y,
  graph_world_x,
  graph_world_z,
  source
FROM region_group_meta
WHERE map_version_id = ?
ORDER BY region_group_id";

pub const REGION_GROUP_REGIONS_SQL: &str = "
SELECT
  region_group_id,
  region_id
FROM region_group_regions
WHERE map_version_id = ?
ORDER BY region_group_id, region_id";

pub const ZONES_SQL: &str =
    "SELECT R, G, B, name, active, confirmed, `index`, bite_time_min, bite_time_max FROM `zones_merged`";

pub const EVENT_ZONE_ASSIGNMENT_COUNT_SQL: &str =
    "SELECT COUNT(1) FROM event_zone_assignment WHERE layer_revision_id = ?";

pub const EVENT_ZONE_RING_SUPPORT_COUNT_SQL: &str =
    "SELECT COUNT(1) FROM event_zone_ring_support WHERE layer_revision_id = ?";

pub const EVENTS_SNAPSHOT_BASE_SQL: &str = "
SELECT
  e.event_id,
  e.fish_id,
  CAST(TIMESTAMPDIFF(SECOND, '1970-01-01 00:00:00', e.ts_utc) AS SIGNED) AS ts_utc,
  e.length_milli,
  e.map_px_x,
  e.map_px_y,
  e.world_x,
  e.world_z,
  e.source_kind,
  e.source_id
FROM events e
WHERE e.water_ok = 1
  AND e.source_kind = ?
ORDER BY e.ts_utc, e.event_id";

pub const EVENTS_SNAPSHOT_ASSIGNMENT_SQL: &str = "
SELECT
  z.event_id,
  z.zone_rgb
FROM event_zone_assignment z
JOIN events e ON e.event_id = z.event_id
WHERE z.layer_revision_id = ?
  AND e.water_ok = 1
  AND e.source_kind = ?
ORDER BY z.event_id";

pub const EVENTS_SNAPSHOT_RING_SUPPORT_SQL: &str = "
SELECT
  ring.event_id,
  ring.zone_rgb
FROM event_zone_ring_support ring
JOIN events e ON e.event_id = ring.event_id
WHERE ring.layer_revision_id = ?
  AND e.water_ok = 1
  AND e.source_kind = ?
ORDER BY ring.event_id, ring.zone_rgb";

pub const WATER_TILES_SQL: &str =
    "SELECT tile_x, tile_y, water_count FROM water_tiles WHERE map_version = ? AND tile_px = ?";

pub const RANKING_EVENTS_WITH_ZONE_SQL: &str = "
SELECT
  CAST(TIMESTAMPDIFF(SECOND, '1970-01-01 00:00:00', e.ts_utc) AS SIGNED) AS ts_utc,
  e.fish_id,
  e.snap_px_x,
  e.snap_px_y,
  z.zone_rgb
FROM events e
JOIN event_zone_assignment z ON z.event_id = e.event_id AND z.layer_revision_id = ?
WHERE e.water_ok = 1
  AND e.source_kind = ?
  AND e.ts_utc >= ?
  AND e.ts_utc < ?";

pub const RANKING_RING_SUPPORT_BY_ZONE_SQL: &str = "
SELECT
  e.fish_id,
  CAST(SUM(CASE WHEN ring.ring_fully_contained = 1 THEN 1 ELSE 0 END) AS SIGNED) AS full_count,
  CAST(SUM(CASE WHEN ring.ring_fully_contained = 0 THEN 1 ELSE 0 END) AS SIGNED) AS partial_count
FROM events e
JOIN event_zone_ring_support ring ON ring.event_id = e.event_id AND ring.layer_revision_id = ?
WHERE e.water_ok = 1
  AND e.source_kind = ?
  AND ring.zone_rgb = ?
GROUP BY e.fish_id";

pub const RANKING_EVENTS_WITH_RING_SUPPORT_SQL: &str = "
SELECT
  e.event_id,
  CAST(TIMESTAMPDIFF(SECOND, '1970-01-01 00:00:00', e.ts_utc) AS SIGNED) AS ts_utc,
  e.fish_id,
  ring.zone_rgb
FROM events e
JOIN event_zone_ring_support ring ON ring.event_id = e.event_id AND ring.layer_revision_id = ?
WHERE e.water_ok = 1
  AND e.source_kind = ?
  AND e.ts_utc >= ?
  AND e.ts_utc < ?
ORDER BY e.event_id, ring.zone_rgb";

pub const HEALTHCHECK_SQL: &str = "SELECT 1";
