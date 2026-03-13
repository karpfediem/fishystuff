-- Legacy events schema rebuild helper.
-- Use this only when upgrading from the old sqlite-style `events` table
-- (missing map_px_x/map_px_y/event_id/sample coordinates).
--
-- WARNING: this drops evidence tables and requires re-importing ranking CSV.

DROP TABLE IF EXISTS event_zone_assignment;
DROP TABLE IF EXISTS layer_revisions;
DROP TABLE IF EXISTS event_zone;
DROP TABLE IF EXISTS ingest_runs;
DROP TABLE IF EXISTS events;

CREATE TABLE IF NOT EXISTS events (
  event_id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
  event_uid CHAR(32) NOT NULL,
  source_kind TINYINT NOT NULL,
  source_id VARCHAR(64) NULL,
  ts_utc DATETIME(6) NOT NULL,
  fish_id INT NOT NULL,
  length_milli INT NOT NULL,
  world_x INT NOT NULL,
  world_y INT NOT NULL,
  world_z INT NOT NULL,
  map_px_x INT NOT NULL,
  map_px_y INT NOT NULL,
  sample_px_x INT NOT NULL,
  sample_px_y INT NOT NULL,
  position_method TINYINT NOT NULL DEFAULT 1,
  snap_px_x INT NOT NULL,
  snap_px_y INT NOT NULL,
  snap_dist_px INT NOT NULL,
  water_ok TINYINT NOT NULL,
  ingested_at DATETIME(6) NOT NULL,
  UNIQUE KEY events_uid_uq (event_uid),
  KEY events_ts_idx (ts_utc),
  KEY events_fish_ts_idx (fish_id, ts_utc),
  KEY events_map_px_idx (map_px_x, map_px_y),
  KEY events_sample_px_idx (sample_px_x, sample_px_y),
  KEY events_snap_px_idx (snap_px_x, snap_px_y)
);

CREATE TABLE IF NOT EXISTS layer_revisions (
  layer_revision_id VARCHAR(64) NOT NULL PRIMARY KEY,
  layer_id VARCHAR(64) NOT NULL,
  label VARCHAR(128) NOT NULL,
  created_at DATETIME(6) NOT NULL,
  effective_from_utc DATETIME(6) NULL,
  effective_to_utc DATETIME(6) NULL,
  patch_id VARCHAR(64) NULL,
  revision_hash VARCHAR(128) NULL,
  notes TEXT NULL,
  KEY layer_revisions_layer_patch_idx (layer_id, patch_id),
  KEY layer_revisions_layer_effective_idx (layer_id, effective_from_utc, effective_to_utc)
);

CREATE TABLE IF NOT EXISTS event_zone_assignment (
  layer_revision_id VARCHAR(64) NOT NULL,
  event_id BIGINT NOT NULL,
  zone_rgb INT UNSIGNED NOT NULL,
  zone_r TINYINT UNSIGNED NOT NULL,
  zone_g TINYINT UNSIGNED NOT NULL,
  zone_b TINYINT UNSIGNED NOT NULL,
  sample_px_x INT NOT NULL,
  sample_px_y INT NOT NULL,
  PRIMARY KEY (layer_revision_id, event_id),
  KEY event_zone_assignment_rgb_idx (layer_revision_id, zone_rgb),
  KEY event_zone_assignment_rgb_event_idx (layer_revision_id, zone_rgb, event_id)
);

CREATE TABLE IF NOT EXISTS ingest_runs (
  ingest_run_id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
  source_kind TINYINT NOT NULL,
  map_version VARCHAR(32) NOT NULL,
  input_sha256 CHAR(64) NOT NULL,
  started_at DATETIME(6) NOT NULL,
  finished_at DATETIME(6) NULL,
  rows_seen INT NOT NULL DEFAULT 0,
  rows_inserted INT NOT NULL DEFAULT 0,
  rows_deduped INT NOT NULL DEFAULT 0,
  notes TEXT NULL
);
