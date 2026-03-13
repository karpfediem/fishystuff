CREATE TABLE IF NOT EXISTS region_group_meta (
  map_version_id VARCHAR(64) NOT NULL,
  region_group_id INT NOT NULL,
  color_rgb_u32 INT UNSIGNED NULL,
  feature_count INT NOT NULL DEFAULT 0,
  region_count INT NOT NULL DEFAULT 0,
  accessible_region_count INT NOT NULL DEFAULT 0,
  bbox_min_x DOUBLE NULL,
  bbox_min_y DOUBLE NULL,
  bbox_max_x DOUBLE NULL,
  bbox_max_y DOUBLE NULL,
  graph_world_x DOUBLE NULL,
  graph_world_z DOUBLE NULL,
  source VARCHAR(64) NOT NULL DEFAULT '',
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (map_version_id, region_group_id)
);

CREATE TABLE IF NOT EXISTS region_group_regions (
  map_version_id VARCHAR(64) NOT NULL,
  region_group_id INT NOT NULL,
  region_id INT NOT NULL,
  trade_origin_region INT NULL,
  is_accessible TINYINT(1) NOT NULL DEFAULT 0,
  waypoint INT NULL,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (map_version_id, region_group_id, region_id),
  KEY idx_region_group_regions_region (map_version_id, region_id)
);

INSERT INTO layers (
  layer_id, name, enabled, ui_display_order, visible_default, opacity_default, z_base,
  transform_kind, affine_a, affine_b, affine_tx, affine_c, affine_d, affine_ty,
  tileset_manifest_url, tile_url_template, tileset_version,
  tile_px, max_level, y_flip, request_weight, pick_mode,
  lod_target_tiles, lod_hysteresis_hi, lod_hysteresis_lo, lod_margin_tiles,
  lod_enable_refine, lod_refine_debounce_ms, lod_max_detail_tiles
) VALUES (
  'region_groups', 'Region Groups', 1, 30, 0, 0.50, 30.0,
  'identity_map_space', NULL, NULL, NULL, NULL, NULL, NULL,
  '/images/tiles/region_groups/{map_version}/tileset.json', '/images/tiles/region_groups/{map_version}/{level}/{x}_{y}.png', 'v1',
  512, 0, 0, 0.4, 'none',
  220, 280.0, 160.0, 1,
  0, 0, 0
)
ON DUPLICATE KEY UPDATE
  name = VALUES(name),
  enabled = VALUES(enabled),
  ui_display_order = VALUES(ui_display_order),
  visible_default = VALUES(visible_default),
  opacity_default = VALUES(opacity_default),
  z_base = VALUES(z_base),
  transform_kind = VALUES(transform_kind),
  affine_a = VALUES(affine_a),
  affine_b = VALUES(affine_b),
  affine_tx = VALUES(affine_tx),
  affine_c = VALUES(affine_c),
  affine_d = VALUES(affine_d),
  affine_ty = VALUES(affine_ty),
  tileset_manifest_url = VALUES(tileset_manifest_url),
  tile_url_template = VALUES(tile_url_template),
  tileset_version = VALUES(tileset_version),
  tile_px = VALUES(tile_px),
  max_level = VALUES(max_level),
  y_flip = VALUES(y_flip),
  request_weight = VALUES(request_weight),
  pick_mode = VALUES(pick_mode),
  lod_target_tiles = VALUES(lod_target_tiles),
  lod_hysteresis_hi = VALUES(lod_hysteresis_hi),
  lod_hysteresis_lo = VALUES(lod_hysteresis_lo),
  lod_margin_tiles = VALUES(lod_margin_tiles),
  lod_enable_refine = VALUES(lod_enable_refine),
  lod_refine_debounce_ms = VALUES(lod_refine_debounce_ms),
  lod_max_detail_tiles = VALUES(lod_max_detail_tiles);

INSERT INTO layer_configs (
  map_version_id, layer_id,
  z_base_override,
  tileset_manifest_url_override, tile_url_template_override, tileset_version_override
) VALUES (
  'v1', 'region_groups', 30.0,
  '/images/tiles/region_groups/v1/tileset.json', '/images/tiles/region_groups/v1/{level}/{x}_{y}.png', 'v1'
)
ON DUPLICATE KEY UPDATE
  z_base_override = VALUES(z_base_override),
  tileset_manifest_url_override = VALUES(tileset_manifest_url_override),
  tile_url_template_override = VALUES(tile_url_template_override),
  tileset_version_override = VALUES(tileset_version_override);
