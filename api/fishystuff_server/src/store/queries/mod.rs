pub const PATCHES_SQL: &str =
    "SELECT patch_id, start_ts_utc, patch_name FROM `patches` ORDER BY start_ts_utc";

pub const MAP_VERSIONS_SQL: &str =
    "SELECT map_version_id, name, is_default FROM `map_versions` ORDER BY map_version_id";

pub const LAYERS_SQL: &str = "
SELECT
  l.layer_id,
  l.name,
  l.enabled,
  l.transform_kind,
  l.affine_a,
  l.affine_b,
  l.affine_tx,
  l.affine_c,
  l.affine_d,
  l.affine_ty,
  l.tileset_manifest_url,
  l.tile_url_template,
  l.tileset_version,
  l.tile_px,
  l.max_level,
  l.y_flip,
  l.lod_target_tiles,
  l.lod_hysteresis_hi,
  l.lod_hysteresis_lo,
  l.lod_margin_tiles,
  l.lod_enable_refine,
  l.lod_refine_debounce_ms,
  l.lod_max_detail_tiles,
  l.visible_default,
  l.opacity_default,
  l.z_base,
  l.ui_display_order,
  l.request_weight,
  l.pick_mode,
  l.layer_kind,
  l.vector_source_url,
  l.vector_source_revision,
  l.vector_geometry_space,
  l.vector_style_mode,
  l.vector_feature_id_property,
  l.vector_color_property,
  l.asset_base_url
FROM layers l
WHERE l.enabled = 1
  AND l.layer_id <> 'water'
ORDER BY l.ui_display_order, l.layer_id";

pub const LAYERS_WITH_CONFIG_SQL: &str = "
SELECT
  l.layer_id,
  l.name,
  COALESCE(c.enabled_override, l.enabled) AS enabled,
  l.transform_kind,
  l.affine_a,
  l.affine_b,
  l.affine_tx,
  l.affine_c,
  l.affine_d,
  l.affine_ty,
  COALESCE(c.tileset_manifest_url_override, l.tileset_manifest_url) AS tileset_manifest_url,
  COALESCE(c.tile_url_template_override, l.tile_url_template) AS tile_url_template,
  COALESCE(c.tileset_version_override, l.tileset_version) AS tileset_version,
  l.tile_px,
  COALESCE(c.max_level_override, l.max_level) AS max_level,
  l.y_flip,
  l.lod_target_tiles,
  l.lod_hysteresis_hi,
  l.lod_hysteresis_lo,
  l.lod_margin_tiles,
  l.lod_enable_refine,
  l.lod_refine_debounce_ms,
  l.lod_max_detail_tiles,
  COALESCE(c.visible_default_override, l.visible_default) AS visible_default,
  COALESCE(c.opacity_default_override, l.opacity_default) AS opacity_default,
  COALESCE(c.z_base_override, l.z_base) AS z_base,
  l.ui_display_order,
  COALESCE(c.request_weight_override, l.request_weight) AS request_weight,
  l.pick_mode,
  l.layer_kind,
  COALESCE(c.vector_source_url_override, l.vector_source_url) AS vector_source_url,
  COALESCE(c.vector_source_revision_override, l.vector_source_revision) AS vector_source_revision,
  l.vector_geometry_space,
  l.vector_style_mode,
  l.vector_feature_id_property,
  l.vector_color_property,
  COALESCE(c.asset_base_url_override, l.asset_base_url) AS asset_base_url
FROM layers l
LEFT JOIN layer_configs c
  ON c.layer_id = l.layer_id
 AND c.map_version_id = ?
WHERE COALESCE(c.enabled_override, l.enabled) = 1
  AND l.layer_id <> 'water'
ORDER BY l.ui_display_order, l.layer_id";

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

pub const ZONES_SQL: &str = "SELECT R, G, B, name FROM `zones_merged`";

pub const EVENT_ZONE_ASSIGNMENT_COUNT_SQL: &str =
    "SELECT COUNT(1) FROM event_zone_assignment WHERE layer_revision_id = ?";

pub const WATER_TILES_SQL: &str =
    "SELECT tile_x, tile_y, water_count FROM water_tiles WHERE map_version = ? AND tile_px = ?";

pub const EVENTS_WITH_ZONE_SQL: &str = "
SELECT
  CAST(TIMESTAMPDIFF(SECOND, '1970-01-01 00:00:00', e.ts_utc) AS SIGNED) AS ts_utc,
  e.fish_id,
  e.snap_px_x,
  e.snap_px_y,
  z.zone_rgb
FROM events e
JOIN event_zone_assignment z ON z.event_id = e.event_id AND z.layer_revision_id = ?
WHERE e.water_ok = 1
  AND e.ts_utc >= ?
  AND e.ts_utc < ?";

pub const HEALTHCHECK_SQL: &str = "SELECT 1";
