INSERT INTO layers (
  layer_id, name, enabled, ui_display_order, visible_default, opacity_default, z_base,
  transform_kind, affine_a, affine_b, affine_tx, affine_c, affine_d, affine_ty,
  tileset_manifest_url, tile_url_template, tileset_version,
  tile_px, max_level, y_flip, request_weight, pick_mode,
  layer_kind, vector_source_url, vector_source_revision, vector_geometry_space, vector_style_mode,
  vector_feature_id_property, vector_color_property,
  lod_target_tiles, lod_hysteresis_hi, lod_hysteresis_lo, lod_margin_tiles,
  lod_enable_refine, lod_refine_debounce_ms, lod_max_detail_tiles
) VALUES (
  'regions', 'Regions', 1, 31, 0, 0.35, 31.0,
  'identity_map_space', NULL, NULL, NULL, NULL, NULL, NULL,
  '', '', '',
  512, 0, 0, 0.45, 'none',
  'vector_geojson', '/region_groups/regions.{map_version}.geojson', 'r-v1', 'map_pixels', 'feature_property_palette',
  'r', 'c',
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
  layer_kind = VALUES(layer_kind),
  vector_source_url = VALUES(vector_source_url),
  vector_source_revision = VALUES(vector_source_revision),
  vector_geometry_space = VALUES(vector_geometry_space),
  vector_style_mode = VALUES(vector_style_mode),
  vector_feature_id_property = VALUES(vector_feature_id_property),
  vector_color_property = VALUES(vector_color_property),
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
  tileset_manifest_url_override,
  tile_url_template_override,
  tileset_version_override,
  vector_source_url_override,
  vector_source_revision_override
) VALUES (
  'v1', 'regions', 31.0,
  '', '', '',
  '/region_groups/regions.v1.geojson',
  'r-v1'
)
ON DUPLICATE KEY UPDATE
  z_base_override = VALUES(z_base_override),
  tileset_manifest_url_override = VALUES(tileset_manifest_url_override),
  tile_url_template_override = VALUES(tile_url_template_override),
  tileset_version_override = VALUES(tileset_version_override),
  vector_source_url_override = VALUES(vector_source_url_override),
  vector_source_revision_override = VALUES(vector_source_revision_override);
