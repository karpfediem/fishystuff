ALTER TABLE layers ADD COLUMN field_metadata_source_url VARCHAR(512) NULL;
ALTER TABLE layers ADD COLUMN field_metadata_source_revision VARCHAR(128) NULL;

UPDATE layers
SET
  field_metadata_source_url = '/fields/region_groups.{map_version}.meta.json',
  field_metadata_source_revision = 'rg-meta-v1'
WHERE layer_id = 'region_groups';

UPDATE layers
SET
  field_metadata_source_url = '/fields/regions.{map_version}.meta.json',
  field_metadata_source_revision = 'r-meta-v1'
WHERE layer_id = 'regions';
