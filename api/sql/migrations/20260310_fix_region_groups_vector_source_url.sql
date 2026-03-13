UPDATE layers
SET vector_source_url = REPLACE(vector_source_url, '/map/region_groups/', '/region_groups/')
WHERE layer_id = 'region_groups'
  AND vector_source_url LIKE '/map/region_groups/%';

UPDATE layer_configs
SET vector_source_url_override = REPLACE(
        vector_source_url_override,
        '/map/region_groups/',
        '/region_groups/'
    )
WHERE layer_id = 'region_groups'
  AND vector_source_url_override LIKE '/map/region_groups/%';
