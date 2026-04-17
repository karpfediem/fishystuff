use anyhow::{bail, Context, Result};
use mysql::{params, prelude::Queryable, Params, Pool, TxOpts, Value};

use fishystuff_store::WaterTile;

pub struct MySqlIngestStore {
    pool: Pool,
}

const EVENT_ZONE_ASSIGNMENT_INSERT_CHUNK_ROWS: usize = 1024;
const EVENT_ZONE_RING_SUPPORT_INSERT_CHUNK_ROWS: usize = 1024;

#[derive(Debug, Clone)]
pub struct RankingEventRow {
    pub event_uid: String,
    pub source_kind: u8,
    pub source_id: Option<String>,
    pub ts_utc: String,
    pub fish_id: i32,
    pub length_milli: i32,
    pub world_x: i32,
    pub world_y: i32,
    pub world_z: i32,
    pub map_px_x: i32,
    pub map_px_y: i32,
    pub snap_px_x: i32,
    pub snap_px_y: i32,
    pub snap_dist_px: i32,
    pub water_ok: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct EventZoneSupportSampleRow {
    pub event_id: i64,
    pub assignment_sample_px_x: i32,
    pub assignment_sample_px_y: i32,
    pub ring_center_px_x: i32,
    pub ring_center_px_y: i32,
    pub has_assignment: bool,
    pub has_ring_support: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct EventZoneInsertRow {
    pub event_id: i64,
    pub zone_rgb: u32,
    pub sample_px_x: i32,
    pub sample_px_y: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct EventZoneRingSupportInsertRow {
    pub event_id: i64,
    pub zone_rgb: u32,
    pub ring_fully_contained: bool,
    pub ring_center_px_x: i32,
    pub ring_center_px_y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventZoneSupportCoverage {
    pub water_event_count: u64,
    pub assignment_event_count: u64,
    pub ring_event_count: u64,
}

#[derive(Debug, Clone)]
pub struct RegionGroupMetaRow {
    pub region_group_id: u32,
    pub color_rgb_u32: Option<u32>,
    pub feature_count: u32,
    pub region_count: u32,
    pub accessible_region_count: u32,
    pub bbox_min_x: Option<f64>,
    pub bbox_min_y: Option<f64>,
    pub bbox_max_x: Option<f64>,
    pub bbox_max_y: Option<f64>,
    pub graph_world_x: Option<f64>,
    pub graph_world_z: Option<f64>,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct RegionGroupRegionRow {
    pub region_group_id: u32,
    pub region_id: u32,
    pub trade_origin_region: Option<u32>,
    pub is_accessible: bool,
    pub waypoint: Option<u32>,
}

impl MySqlIngestStore {
    pub fn open(database_url: &str) -> Result<Self> {
        let pool = Pool::new(database_url).context("open mysql pool")?;
        let store = Self { pool };
        store.migrate()?;
        Ok(store)
    }

    pub fn start_ingest_run(
        &self,
        source_kind: u8,
        map_version: &str,
        input_sha256: &str,
    ) -> Result<u64> {
        let mut conn = self.pool.get_conn().context("get mysql conn")?;
        conn.exec_drop(
            "INSERT INTO ingest_runs (source_kind, map_version, input_sha256, started_at) \
             VALUES (:source_kind, :map_version, :input_sha256, UTC_TIMESTAMP(6))",
            params! {
                "source_kind" => i64::from(source_kind),
                "map_version" => map_version,
                "input_sha256" => input_sha256,
            },
        )
        .context("insert ingest run")?;
        Ok(conn.last_insert_id())
    }

    pub fn finish_ingest_run(
        &self,
        ingest_run_id: u64,
        rows_seen: u64,
        rows_inserted: u64,
        rows_deduped: u64,
        notes: Option<&str>,
    ) -> Result<()> {
        let mut conn = self.pool.get_conn().context("get mysql conn")?;
        conn.exec_drop(
            "UPDATE ingest_runs \
             SET rows_seen = :rows_seen, rows_inserted = :rows_inserted, rows_deduped = :rows_deduped, notes = :notes, finished_at = UTC_TIMESTAMP(6) \
             WHERE ingest_run_id = :ingest_run_id",
            params! {
                "rows_seen" => rows_seen,
                "rows_inserted" => rows_inserted,
                "rows_deduped" => rows_deduped,
                "notes" => notes,
                "ingest_run_id" => ingest_run_id,
            },
        )
        .context("update ingest run")?;
        Ok(())
    }

    pub fn insert_events(&self, events: &[RankingEventRow]) -> Result<u64> {
        if events.is_empty() {
            return Ok(0);
        }
        let mut conn = self.pool.get_conn().context("get mysql conn")?;
        conn.exec_batch(
            "INSERT IGNORE INTO events \
             (
               event_uid, source_kind, source_id, ts_utc, fish_id, length_milli,
               world_x, world_y, world_z, map_px_x, map_px_y,
               snap_px_x, snap_px_y, snap_dist_px, water_ok, ingested_at
             ) \
             VALUES \
             (
               :event_uid, :source_kind, :source_id, :ts_utc, :fish_id, :length_milli,
               :world_x, :world_y, :world_z, :map_px_x, :map_px_y,
               :snap_px_x, :snap_px_y, :snap_dist_px, :water_ok, UTC_TIMESTAMP(6)
             )",
            events.iter().map(|ev| {
                params! {
                    "event_uid" => &ev.event_uid,
                    "source_kind" => i64::from(ev.source_kind),
                    "source_id" => ev.source_id.as_deref(),
                    "ts_utc" => ev.ts_utc.as_str(),
                    "fish_id" => ev.fish_id,
                    "length_milli" => ev.length_milli,
                    "world_x" => ev.world_x,
                    "world_y" => ev.world_y,
                    "world_z" => ev.world_z,
                    "map_px_x" => ev.map_px_x,
                    "map_px_y" => ev.map_px_y,
                    "snap_px_x" => ev.snap_px_x,
                    "snap_px_y" => ev.snap_px_y,
                    "snap_dist_px" => ev.snap_dist_px,
                    "water_ok" => if ev.water_ok { 1 } else { 0 },
                }
            }),
        )
        .context("insert mysql events")?;
        Ok(conn.affected_rows())
    }

    pub fn load_events_after_id(
        &self,
        after_event_id: i64,
        limit: usize,
    ) -> Result<Vec<EventZoneSupportSampleRow>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let mut conn = self.pool.get_conn().context("get mysql conn")?;
        let rows: Vec<(i64, i64, i64, i64, i64)> = conn
            .exec(
                "SELECT e.event_id, e.snap_px_x, e.snap_px_y, e.map_px_x, e.map_px_y \
                 FROM events e \
                 WHERE e.water_ok = 1 \
                   AND e.event_id > :after_event_id \
                 ORDER BY e.event_id \
                 LIMIT :limit_rows",
                params! {
                    "after_event_id" => after_event_id,
                    "limit_rows" => limit as u64,
                },
            )
            .context("query mysql events after event id")?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    event_id,
                    assignment_sample_px_x,
                    assignment_sample_px_y,
                    ring_center_px_x,
                    ring_center_px_y,
                )| EventZoneSupportSampleRow {
                    event_id,
                    assignment_sample_px_x: assignment_sample_px_x as i32,
                    assignment_sample_px_y: assignment_sample_px_y as i32,
                    ring_center_px_x: ring_center_px_x as i32,
                    ring_center_px_y: ring_center_px_y as i32,
                    has_assignment: false,
                    has_ring_support: false,
                },
            )
            .collect())
    }

    pub fn load_events_with_zone_support_status_after_id(
        &self,
        layer_revision_id: &str,
        after_event_id: i64,
        limit: usize,
    ) -> Result<Vec<EventZoneSupportSampleRow>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let mut conn = self.pool.get_conn().context("get mysql conn")?;
        let rows: Vec<(i64, i64, i64, i64, i64, u8, u8)> = conn
            .exec(
                "SELECT \
                    e.event_id, \
                    e.snap_px_x, \
                    e.snap_px_y, \
                    e.map_px_x, \
                    e.map_px_y, \
                    EXISTS( \
                        SELECT 1 \
                        FROM event_zone_assignment z \
                        WHERE z.layer_revision_id = :layer_revision_id \
                          AND z.event_id = e.event_id \
                    ) AS has_assignment, \
                    EXISTS( \
                        SELECT 1 \
                        FROM event_zone_ring_support ring \
                        WHERE ring.layer_revision_id = :layer_revision_id \
                          AND ring.event_id = e.event_id \
                    ) AS has_ring_support \
                 FROM events e \
                 WHERE e.water_ok = 1 \
                   AND e.event_id > :after_event_id \
                 ORDER BY e.event_id \
                 LIMIT :limit_rows",
                params! {
                    "layer_revision_id" => layer_revision_id,
                    "after_event_id" => after_event_id,
                    "limit_rows" => limit as u64,
                },
            )
            .context("query mysql events with zone support status rows")?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    event_id,
                    assignment_sample_px_x,
                    assignment_sample_px_y,
                    ring_center_px_x,
                    ring_center_px_y,
                    has_assignment,
                    has_ring_support,
                )| EventZoneSupportSampleRow {
                    event_id,
                    assignment_sample_px_x: assignment_sample_px_x as i32,
                    assignment_sample_px_y: assignment_sample_px_y as i32,
                    ring_center_px_x: ring_center_px_x as i32,
                    ring_center_px_y: ring_center_px_y as i32,
                    has_assignment: has_assignment != 0,
                    has_ring_support: has_ring_support != 0,
                },
            )
            .collect())
    }

    pub fn zone_support_coverage(
        &self,
        layer_revision_id: &str,
    ) -> Result<EventZoneSupportCoverage> {
        let mut conn = self.pool.get_conn().context("get mysql conn")?;
        let row: Option<(u64, u64, u64)> = conn
            .exec_first(
                "SELECT \
                    ( \
                        SELECT COUNT(*) \
                        FROM events \
                        WHERE water_ok = 1 \
                    ), \
                    ( \
                        SELECT COUNT(*) \
                        FROM event_zone_assignment \
                        WHERE layer_revision_id = :layer_revision_id \
                    ), \
                    ( \
                        SELECT COUNT(DISTINCT event_id) \
                        FROM event_zone_ring_support \
                        WHERE layer_revision_id = :layer_revision_id \
                    )",
                params! {
                    "layer_revision_id" => layer_revision_id,
                },
            )
            .context("query mysql zone support coverage")?;
        let (water_event_count, assignment_event_count, ring_event_count) =
            row.unwrap_or((0, 0, 0));
        Ok(EventZoneSupportCoverage {
            water_event_count,
            assignment_event_count,
            ring_event_count,
        })
    }

    pub fn insert_event_zone_support_batch(
        &self,
        layer_revision_id: &str,
        assignment_rows: &[EventZoneInsertRow],
        ring_rows: &[EventZoneRingSupportInsertRow],
    ) -> Result<(u64, u64)> {
        if assignment_rows.is_empty() && ring_rows.is_empty() {
            return Ok((0, 0));
        }

        let mut tx = self
            .pool
            .start_transaction(TxOpts::default())
            .context("start mysql zone support transaction")?;
        let assigned =
            insert_event_zone_assignment_rows(&mut tx, layer_revision_id, assignment_rows)
                .context("insert mysql event_zone_assignment batch")?;
        let ring_assigned =
            insert_event_zone_ring_support_rows(&mut tx, layer_revision_id, ring_rows)
                .context("insert mysql event_zone_ring_support batch")?;
        tx.commit()
            .context("commit mysql zone support transaction")?;
        Ok((assigned, ring_assigned))
    }

    pub fn ensure_layer_revision(
        &self,
        layer_revision_id: &str,
        layer_id: &str,
        map_version_id: &str,
        label: &str,
        revision_hash: Option<&str>,
        notes: &str,
    ) -> Result<()> {
        if layer_revision_id.trim().is_empty() {
            bail!("layer_revision_id must be non-empty");
        }
        if layer_id.trim().is_empty() {
            bail!("layer_id must be non-empty");
        }
        if map_version_id.trim().is_empty() {
            bail!("map_version_id must be non-empty");
        }
        if label.trim().is_empty() {
            bail!("label must be non-empty");
        }
        let mut conn = self.pool.get_conn().context("get mysql conn")?;
        conn.exec_drop(
            "INSERT INTO layer_revisions (
                layer_revision_id,
                layer_id,
                map_version_id,
                label,
                created_at,
                effective_from_utc,
                effective_to_utc,
                patch_id,
                revision_hash,
                notes
            ) VALUES (
                :layer_revision_id,
                :layer_id,
                :map_version_id,
                :label,
                UTC_TIMESTAMP(6),
                NULL,
                NULL,
                NULL,
                :revision_hash,
                :notes
            ) ON DUPLICATE KEY UPDATE
                layer_id = VALUES(layer_id),
                map_version_id = VALUES(map_version_id),
                label = VALUES(label),
                revision_hash = VALUES(revision_hash),
                notes = VALUES(notes)",
            params! {
                "layer_revision_id" => layer_revision_id,
                "layer_id" => layer_id,
                "map_version_id" => map_version_id,
                "label" => label,
                "revision_hash" => revision_hash,
                "notes" => notes,
            },
        )
        .context("upsert layer revision")?;
        Ok(())
    }

    pub fn upsert_water_tiles(&self, map_version: &str, tiles: &[WaterTile]) -> Result<()> {
        if tiles.is_empty() {
            return Ok(());
        }
        let mut conn = self.pool.get_conn().context("get mysql conn")?;
        conn.exec_batch(
            "INSERT INTO water_tiles (map_version, tile_px, tile_x, tile_y, water_count) \
             VALUES (:map_version, :tile_px, :tile_x, :tile_y, :water_count) \
             ON DUPLICATE KEY UPDATE water_count = VALUES(water_count)",
            tiles.iter().map(|tile| {
                params! {
                    "map_version" => map_version,
                    "tile_px" => tile.tile_px,
                    "tile_x" => tile.tile_x,
                    "tile_y" => tile.tile_y,
                    "water_count" => tile.water_count,
                }
            }),
        )
        .context("upsert mysql water_tiles")?;
        Ok(())
    }

    pub fn replace_region_groups(
        &self,
        map_version_id: &str,
        meta_rows: &[RegionGroupMetaRow],
        region_rows: &[RegionGroupRegionRow],
    ) -> Result<()> {
        if map_version_id.trim().is_empty() {
            bail!("map_version_id must be non-empty");
        }

        let mut conn = self.pool.get_conn().context("get mysql conn")?;
        let mut tx = conn
            .start_transaction(mysql::TxOpts::default())
            .context("start mysql transaction")?;

        tx.exec_drop(
            "DELETE FROM region_group_regions WHERE map_version_id = :map_version_id",
            params! {
                "map_version_id" => map_version_id,
            },
        )
        .context("clear region_group_regions")?;
        tx.exec_drop(
            "DELETE FROM region_group_meta WHERE map_version_id = :map_version_id",
            params! {
                "map_version_id" => map_version_id,
            },
        )
        .context("clear region_group_meta")?;

        if !meta_rows.is_empty() {
            tx.exec_batch(
                "INSERT INTO region_group_meta (
                    map_version_id,
                    region_group_id,
                    color_rgb_u32,
                    feature_count,
                    region_count,
                    accessible_region_count,
                    bbox_min_x,
                    bbox_min_y,
                    bbox_max_x,
                    bbox_max_y,
                    graph_world_x,
                    graph_world_z,
                    source
                ) VALUES (
                    :map_version_id,
                    :region_group_id,
                    :color_rgb_u32,
                    :feature_count,
                    :region_count,
                    :accessible_region_count,
                    :bbox_min_x,
                    :bbox_min_y,
                    :bbox_max_x,
                    :bbox_max_y,
                    :graph_world_x,
                    :graph_world_z,
                    :source
                )",
                meta_rows.iter().map(|row| {
                    params! {
                        "map_version_id" => map_version_id,
                        "region_group_id" => i64::from(row.region_group_id),
                        "color_rgb_u32" => row.color_rgb_u32.map(i64::from),
                        "feature_count" => i64::from(row.feature_count),
                        "region_count" => i64::from(row.region_count),
                        "accessible_region_count" => i64::from(row.accessible_region_count),
                        "bbox_min_x" => row.bbox_min_x,
                        "bbox_min_y" => row.bbox_min_y,
                        "bbox_max_x" => row.bbox_max_x,
                        "bbox_max_y" => row.bbox_max_y,
                        "graph_world_x" => row.graph_world_x,
                        "graph_world_z" => row.graph_world_z,
                        "source" => row.source.as_str(),
                    }
                }),
            )
            .context("insert region_group_meta")?;
        }

        if !region_rows.is_empty() {
            tx.exec_batch(
                "INSERT INTO region_group_regions (
                    map_version_id,
                    region_group_id,
                    region_id,
                    trade_origin_region,
                    is_accessible,
                    waypoint
                ) VALUES (
                    :map_version_id,
                    :region_group_id,
                    :region_id,
                    :trade_origin_region,
                    :is_accessible,
                    :waypoint
                )",
                region_rows.iter().map(|row| {
                    params! {
                        "map_version_id" => map_version_id,
                        "region_group_id" => i64::from(row.region_group_id),
                        "region_id" => i64::from(row.region_id),
                        "trade_origin_region" => row.trade_origin_region.map(i64::from),
                        "is_accessible" => if row.is_accessible { 1_i64 } else { 0_i64 },
                        "waypoint" => row.waypoint.map(i64::from),
                    }
                }),
            )
            .context("insert region_group_regions")?;
        }

        tx.commit()
            .context("commit region-group import transaction")?;
        Ok(())
    }

    fn migrate(&self) -> Result<()> {
        let mut conn = self.pool.get_conn().context("get mysql conn")?;
        conn.query_drop(
            "CREATE TABLE IF NOT EXISTS events (
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
            )",
        )
        .context("create mysql events")?;

        conn.query_drop(
            "CREATE TABLE IF NOT EXISTS event_zone_assignment (
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
            )",
        )
        .context("create mysql event_zone_assignment")?;

        conn.query_drop(
            "CREATE TABLE IF NOT EXISTS event_zone_ring_support (
                layer_revision_id VARCHAR(64) NOT NULL,
                event_id BIGINT NOT NULL,
                zone_rgb INT UNSIGNED NOT NULL,
                zone_r TINYINT UNSIGNED NOT NULL,
                zone_g TINYINT UNSIGNED NOT NULL,
                zone_b TINYINT UNSIGNED NOT NULL,
                ring_fully_contained TINYINT(1) NOT NULL,
                ring_center_px_x INT NOT NULL,
                ring_center_px_y INT NOT NULL,
                PRIMARY KEY (layer_revision_id, event_id, zone_rgb),
                KEY event_zone_ring_support_rgb_idx (layer_revision_id, zone_rgb),
                KEY event_zone_ring_support_rgb_event_idx (layer_revision_id, zone_rgb, event_id)
            )",
        )
        .context("create mysql event_zone_ring_support")?;

        conn.query_drop(
            "CREATE TABLE IF NOT EXISTS layer_revisions (
                layer_revision_id VARCHAR(64) NOT NULL PRIMARY KEY,
                layer_id VARCHAR(64) NOT NULL,
                map_version_id VARCHAR(64) NULL,
                label VARCHAR(128) NOT NULL,
                created_at DATETIME(6) NOT NULL,
                effective_from_utc DATETIME(6) NULL,
                effective_to_utc DATETIME(6) NULL,
                patch_id VARCHAR(64) NULL,
                revision_hash VARCHAR(128) NULL,
                notes TEXT NULL,
                KEY layer_revisions_layer_patch_idx (layer_id, patch_id),
                KEY layer_revisions_layer_effective_idx (layer_id, effective_from_utc, effective_to_utc),
                KEY layer_revisions_layer_map_version_idx (layer_id, map_version_id, created_at)
            )",
        )
        .context("create mysql layer_revisions")?;

        let has_map_version_id: Option<String> = conn
            .query_first(
                "SELECT COLUMN_NAME \
                 FROM information_schema.columns \
                 WHERE table_schema = DATABASE() \
                   AND table_name = 'layer_revisions' \
                   AND column_name = 'map_version_id'",
            )
            .context("check mysql layer_revisions.map_version_id column")?;
        if has_map_version_id.is_none() {
            conn.query_drop(
                "ALTER TABLE layer_revisions \
                 ADD COLUMN map_version_id VARCHAR(64) NULL AFTER layer_id",
            )
            .context("add mysql layer_revisions.map_version_id column")?;
        }

        let has_layer_map_version_idx: Option<String> = conn
            .query_first(
                "SELECT INDEX_NAME \
                 FROM information_schema.statistics \
                 WHERE table_schema = DATABASE() \
                   AND table_name = 'layer_revisions' \
                   AND index_name = 'layer_revisions_layer_map_version_idx'",
            )
            .context("check mysql layer_revisions layer/map_version index")?;
        if has_layer_map_version_idx.is_none() {
            conn.query_drop(
                "CREATE INDEX layer_revisions_layer_map_version_idx \
                 ON layer_revisions (layer_id, map_version_id, created_at)",
            )
            .context("create mysql layer_revisions layer/map_version index")?;
        }

        let has_events_water_ok_event_idx: Option<String> = conn
            .query_first(
                "SELECT INDEX_NAME \
                 FROM information_schema.statistics \
                 WHERE table_schema = DATABASE() \
                   AND table_name = 'events' \
                   AND index_name = 'events_water_ok_event_idx'",
            )
            .context("check mysql events water_ok/event_id index")?;
        if has_events_water_ok_event_idx.is_none() {
            conn.query_drop(
                "CREATE INDEX events_water_ok_event_idx \
                 ON events (water_ok, event_id)",
            )
            .context("create mysql events water_ok/event_id index")?;
        }

        conn.query_drop(
            "CREATE TABLE IF NOT EXISTS water_tiles (
                map_version VARCHAR(64) NOT NULL,
                tile_px INT NOT NULL,
                tile_x INT NOT NULL,
                tile_y INT NOT NULL,
                water_count INT NOT NULL,
                PRIMARY KEY (map_version, tile_px, tile_x, tile_y),
                KEY water_tiles_px_idx (map_version, tile_px)
            )",
        )
        .context("create mysql water_tiles")?;

        conn.query_drop(
            "CREATE TABLE IF NOT EXISTS ingest_runs (
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
            )",
        )
        .context("create mysql ingest_runs")?;

        conn.query_drop(
            "CREATE TABLE IF NOT EXISTS region_group_meta (
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
            )",
        )
        .context("create mysql region_group_meta")?;

        conn.query_drop(
            "CREATE TABLE IF NOT EXISTS region_group_regions (
                map_version_id VARCHAR(64) NOT NULL,
                region_group_id INT NOT NULL,
                region_id INT NOT NULL,
                trade_origin_region INT NULL,
                is_accessible TINYINT(1) NOT NULL DEFAULT 0,
                waypoint INT NULL,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
                PRIMARY KEY (map_version_id, region_group_id, region_id),
                KEY idx_region_group_regions_region (map_version_id, region_id)
            )",
        )
        .context("create mysql region_group_regions")?;

        Ok(())
    }
}

fn append_values_placeholders(query: &mut String, row_placeholder: &str, row_count: usize) {
    for idx in 0..row_count {
        if idx > 0 {
            query.push_str(", ");
        }
        query.push_str(row_placeholder);
    }
}

fn insert_event_zone_assignment_rows<Q: Queryable>(
    conn: &mut Q,
    layer_revision_id: &str,
    rows: &[EventZoneInsertRow],
) -> Result<u64> {
    if rows.is_empty() {
        return Ok(0);
    }

    let mut inserted = 0u64;
    for chunk in rows.chunks(EVENT_ZONE_ASSIGNMENT_INSERT_CHUNK_ROWS) {
        let mut query = String::from(
            "INSERT IGNORE INTO event_zone_assignment \
             (layer_revision_id, event_id, zone_rgb, zone_r, zone_g, zone_b, sample_px_x, sample_px_y) \
             VALUES ",
        );
        append_values_placeholders(&mut query, "(?, ?, ?, ?, ?, ?, ?, ?)", chunk.len());

        let mut params = Vec::with_capacity(chunk.len() * 8);
        for row in chunk {
            let zone_r = ((row.zone_rgb >> 16) & 0xFF) as u8;
            let zone_g = ((row.zone_rgb >> 8) & 0xFF) as u8;
            let zone_b = (row.zone_rgb & 0xFF) as u8;
            params.push(Value::from(layer_revision_id));
            params.push(Value::from(row.event_id));
            params.push(Value::from(row.zone_rgb));
            params.push(Value::from(zone_r));
            params.push(Value::from(zone_g));
            params.push(Value::from(zone_b));
            params.push(Value::from(row.sample_px_x));
            params.push(Value::from(row.sample_px_y));
        }

        let result = conn
            .exec_iter(query, Params::Positional(params))
            .context("insert mysql event_zone_assignment rows")?;
        inserted += result.affected_rows();
    }

    Ok(inserted)
}

fn insert_event_zone_ring_support_rows<Q: Queryable>(
    conn: &mut Q,
    layer_revision_id: &str,
    rows: &[EventZoneRingSupportInsertRow],
) -> Result<u64> {
    if rows.is_empty() {
        return Ok(0);
    }

    let mut inserted = 0u64;
    for chunk in rows.chunks(EVENT_ZONE_RING_SUPPORT_INSERT_CHUNK_ROWS) {
        let mut query = String::from(
            "INSERT IGNORE INTO event_zone_ring_support \
             (layer_revision_id, event_id, zone_rgb, zone_r, zone_g, zone_b, ring_fully_contained, ring_center_px_x, ring_center_px_y) \
             VALUES ",
        );
        append_values_placeholders(&mut query, "(?, ?, ?, ?, ?, ?, ?, ?, ?)", chunk.len());

        let mut params = Vec::with_capacity(chunk.len() * 9);
        for row in chunk {
            let zone_r = ((row.zone_rgb >> 16) & 0xFF) as u8;
            let zone_g = ((row.zone_rgb >> 8) & 0xFF) as u8;
            let zone_b = (row.zone_rgb & 0xFF) as u8;
            params.push(Value::from(layer_revision_id));
            params.push(Value::from(row.event_id));
            params.push(Value::from(row.zone_rgb));
            params.push(Value::from(zone_r));
            params.push(Value::from(zone_g));
            params.push(Value::from(zone_b));
            params.push(Value::from(if row.ring_fully_contained {
                1_i64
            } else {
                0_i64
            }));
            params.push(Value::from(row.ring_center_px_x));
            params.push(Value::from(row.ring_center_px_y));
        }

        let result = conn
            .exec_iter(query, Params::Positional(params))
            .context("insert mysql event_zone_ring_support rows")?;
        inserted += result.affected_rows();
    }

    Ok(inserted)
}
