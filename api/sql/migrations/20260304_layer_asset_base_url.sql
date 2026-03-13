ALTER TABLE layers
  ADD COLUMN asset_base_url VARCHAR(512) NULL;

ALTER TABLE layer_configs
  ADD COLUMN asset_base_url_override VARCHAR(512) NULL;

UPDATE layers
SET asset_base_url = NULL
WHERE asset_base_url IS NOT NULL
  AND TRIM(asset_base_url) = '';

UPDATE layer_configs
SET asset_base_url_override = NULL
WHERE asset_base_url_override IS NOT NULL
  AND TRIM(asset_base_url_override) = '';
