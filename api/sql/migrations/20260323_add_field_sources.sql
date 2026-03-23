ALTER TABLE layers ADD COLUMN field_source_url VARCHAR(512) NULL;
ALTER TABLE layers ADD COLUMN field_source_revision VARCHAR(128) NULL;
ALTER TABLE layers ADD COLUMN field_color_mode VARCHAR(32) NOT NULL DEFAULT 'rgb_u24';

UPDATE layers
SET
  field_source_url = '/fields/region_groups.{map_version}.bin',
  field_source_revision = 'rg-field-v1',
  field_color_mode = 'debug_hash'
WHERE layer_id = 'region_groups';

UPDATE layers
SET
  field_source_url = '/fields/regions.{map_version}.bin',
  field_source_revision = 'r-field-v1',
  field_color_mode = 'debug_hash'
WHERE layer_id = 'regions';
