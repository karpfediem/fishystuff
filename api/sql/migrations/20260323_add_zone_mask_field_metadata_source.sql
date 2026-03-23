UPDATE layers
SET
  field_metadata_source_url = '/fields/zone_mask.{map_version}.meta.json',
  field_metadata_source_revision = 'zone-meta-v1'
WHERE layer_id = 'zone_mask';
