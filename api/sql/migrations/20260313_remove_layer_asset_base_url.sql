UPDATE layers
SET tile_url_template = CONCAT('/images', tile_url_template)
WHERE tile_url_template LIKE '/tiles/%';

UPDATE layers
SET tile_url_template = CONCAT('images/', SUBSTRING(tile_url_template, 7))
WHERE tile_url_template LIKE 'tiles/%';

UPDATE layer_configs
SET tile_url_template_override = CONCAT('/images', tile_url_template_override)
WHERE tile_url_template_override LIKE '/tiles/%';

UPDATE layer_configs
SET tile_url_template_override = CONCAT('images/', SUBSTRING(tile_url_template_override, 7))
WHERE tile_url_template_override LIKE 'tiles/%';

ALTER TABLE layers
  DROP COLUMN asset_base_url;

ALTER TABLE layer_configs
  DROP COLUMN asset_base_url_override;
