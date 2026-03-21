CREATE TABLE IF NOT EXISTS community_zone_fish_support (
  source_id VARCHAR(64) NOT NULL,
  source_label VARCHAR(128) NOT NULL,
  source_sha256 CHAR(64) NULL,
  zone_rgb INT UNSIGNED NOT NULL,
  zone_r TINYINT UNSIGNED NOT NULL,
  zone_g TINYINT UNSIGNED NOT NULL,
  zone_b TINYINT UNSIGNED NOT NULL,
  region_name TEXT NULL,
  zone_name TEXT NULL,
  item_id BIGINT NOT NULL,
  fish_name TEXT NULL,
  support_status VARCHAR(32) NOT NULL,
  claim_count INT NOT NULL DEFAULT 1,
  notes TEXT NULL,

  PRIMARY KEY (source_id, zone_rgb, item_id),
  KEY idx_community_zone_fish_support_rgb (zone_rgb),
  KEY idx_community_zone_fish_support_status (support_status),
  KEY idx_community_zone_fish_support_item (item_id)
);
