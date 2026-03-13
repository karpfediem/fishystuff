use std::path::Path;

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection};

use fishystuff_core::constants::{MAP_HEIGHT, MAP_WIDTH};
use fishystuff_core::tile::tile_dimensions;

use crate::{Event, EventPoint, EventZoneRow, WaterEvent, WaterTile};

const SCHEMA_VERSION: i64 = 1;

pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut conn = Connection::open(path).context("open sqlite db")?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .context("enable foreign_keys")?;
        migrate(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory().context("open sqlite memory db")?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .context("enable foreign_keys")?;
        migrate(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn insert_events(&mut self, events: &[Event]) -> Result<()> {
        let tx = self.conn.transaction().context("start tx")?;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO events (ts_utc, fish_id, world_x, world_z, px, py, water_px, water_py, tile_x, tile_y, water_ok) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                )
                .context("prepare insert events")?;
            for ev in events {
                stmt.execute(params![
                    ev.ts_utc,
                    ev.fish_id,
                    ev.world_x,
                    ev.world_z,
                    ev.px,
                    ev.py,
                    ev.water_px,
                    ev.water_py,
                    ev.tile_x,
                    ev.tile_y,
                    if ev.water_ok { 1 } else { 0 }
                ])
                .context("insert event")?;
            }
        }
        tx.commit().context("commit events")?;
        Ok(())
    }

    pub fn load_water_events(&self) -> Result<Vec<WaterEvent>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, water_px, water_py \
                 FROM events \
                 WHERE water_ok=1 AND water_px IS NOT NULL AND water_py IS NOT NULL",
            )
            .context("prepare water events")?;
        let iter = stmt
            .query_map([], |row| {
                Ok(WaterEvent {
                    id: row.get(0)?,
                    water_px: row.get(1)?,
                    water_py: row.get(2)?,
                })
            })
            .context("query water events")?;
        let mut events = Vec::new();
        for ev in iter {
            events.push(ev.context("row water event")?);
        }
        Ok(events)
    }

    pub fn load_events_with_zone_in_window(
        &self,
        map_version: &str,
        from_ts_utc: i64,
        to_ts_utc: i64,
    ) -> Result<Vec<EventZoneRow>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT e.ts_utc, e.fish_id, e.tile_x, e.tile_y, z.zone_rgb_u32 \
                 FROM events e \
                 JOIN event_zone z ON z.event_id = e.id AND z.map_version = ?1 \
                 WHERE e.water_ok = 1 \
                   AND e.ts_utc >= ?2 AND e.ts_utc < ?3 \
                   AND e.tile_x IS NOT NULL AND e.tile_y IS NOT NULL",
            )
            .context("prepare events with zone")?;
        let iter = stmt
            .query_map(params![map_version, from_ts_utc, to_ts_utc], |row| {
                Ok(EventZoneRow {
                    ts_utc: row.get(0)?,
                    fish_id: row.get(1)?,
                    tile_x: row.get(2)?,
                    tile_y: row.get(3)?,
                    zone_rgb_u32: row.get(4)?,
                })
            })
            .context("query events with zone")?;
        let mut out = Vec::new();
        for row in iter {
            out.push(row.context("row event with zone")?);
        }
        Ok(out)
    }

    pub fn load_event_points_in_window(
        &self,
        from_ts_utc: i64,
        to_ts_utc: i64,
        fish_id: Option<i32>,
    ) -> Result<Vec<EventPoint>> {
        let mut out = Vec::new();
        if let Some(fish_id) = fish_id {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT water_px, water_py, fish_id \
                     FROM events \
                     WHERE water_ok = 1 \
                       AND water_px IS NOT NULL AND water_py IS NOT NULL \
                       AND ts_utc >= ?1 AND ts_utc < ?2 \
                       AND fish_id = ?3 \
                     ORDER BY id",
                )
                .context("prepare event points (fish)")?;
            let iter = stmt
                .query_map(params![from_ts_utc, to_ts_utc, fish_id], |row| {
                    Ok(EventPoint {
                        water_px: row.get(0)?,
                        water_py: row.get(1)?,
                        fish_id: row.get(2)?,
                    })
                })
                .context("query event points (fish)")?;
            for row in iter {
                out.push(row.context("row event point")?);
            }
        } else {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT water_px, water_py, fish_id \
                     FROM events \
                     WHERE water_ok = 1 \
                       AND water_px IS NOT NULL AND water_py IS NOT NULL \
                       AND ts_utc >= ?1 AND ts_utc < ?2 \
                     ORDER BY id",
                )
                .context("prepare event points")?;
            let iter = stmt
                .query_map(params![from_ts_utc, to_ts_utc], |row| {
                    Ok(EventPoint {
                        water_px: row.get(0)?,
                        water_py: row.get(1)?,
                        fish_id: row.get(2)?,
                    })
                })
                .context("query event points")?;
            for row in iter {
                out.push(row.context("row event point")?);
            }
        }
        Ok(out)
    }

    pub fn load_water_tiles(&self, tile_px: i32) -> Result<(i32, i32, Vec<u32>)> {
        if tile_px <= 0 {
            bail!("tile_px must be > 0");
        }
        let (grid_w, grid_h) = tile_dimensions(MAP_WIDTH, MAP_HEIGHT, tile_px);
        let len = (grid_w * grid_h) as usize;
        let mut values: Vec<Option<u32>> = vec![None; len];

        let mut stmt = self
            .conn
            .prepare(
                "SELECT tile_x, tile_y, water_count \
                 FROM water_tiles \
                 WHERE tile_px = ?1",
            )
            .context("prepare water tiles")?;
        let iter = stmt
            .query_map(params![tile_px], |row| {
                Ok((
                    row.get::<_, i32>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, i32>(2)?,
                ))
            })
            .context("query water tiles")?;

        let mut rows = 0usize;
        for row in iter {
            let (tile_x, tile_y, water_count) = row.context("row water tile")?;
            if tile_x < 0 || tile_y < 0 || tile_x >= grid_w || tile_y >= grid_h {
                bail!(
                    "water_tiles out of bounds: tile_x={}, tile_y={}, grid={}x{}",
                    tile_x,
                    tile_y,
                    grid_w,
                    grid_h
                );
            }
            let idx = (tile_y * grid_w + tile_x) as usize;
            if values[idx].is_some() {
                bail!(
                    "duplicate water_tiles entry at tile_x={}, tile_y={}",
                    tile_x,
                    tile_y
                );
            }
            values[idx] = Some(water_count as u32);
            rows += 1;
        }

        if rows == 0 {
            bail!(
                "water_tiles missing for tile_px={}; run fishystuff_ingest index-water-tiles",
                tile_px
            );
        }

        if values.iter().any(|v| v.is_none()) {
            bail!(
                "water_tiles incomplete for tile_px={}; run fishystuff_ingest index-water-tiles",
                tile_px
            );
        }

        let out = values.into_iter().map(|v| v.unwrap()).collect();
        Ok((grid_w, grid_h, out))
    }

    pub fn has_event_zone(&self, map_version: &str) -> Result<bool> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(1) FROM event_zone WHERE map_version=?1",
                params![map_version],
                |row| row.get(0),
            )
            .context("count event_zone rows")?;
        Ok(count > 0)
    }

    pub fn insert_event_zones(
        &mut self,
        map_version: &str,
        rows: &[(i64, u32)],
        overwrite: bool,
    ) -> Result<usize> {
        let sql = if overwrite {
            "INSERT OR REPLACE INTO event_zone (map_version, event_id, zone_rgb_u32) VALUES (?1, ?2, ?3)"
        } else {
            "INSERT OR IGNORE INTO event_zone (map_version, event_id, zone_rgb_u32) VALUES (?1, ?2, ?3)"
        };
        let tx = self.conn.transaction().context("start tx")?;
        let mut affected = 0usize;
        {
            let mut stmt = tx.prepare_cached(sql).context("prepare event zones")?;
            for (event_id, rgb) in rows {
                affected += stmt
                    .execute(params![map_version, event_id, *rgb])
                    .context("insert event zone")?;
            }
        }
        tx.commit().context("commit event zones")?;
        Ok(affected)
    }

    pub fn load_event_zones(&self, map_version: &str) -> Result<Vec<(i64, u32)>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT event_id, zone_rgb_u32 \
                 FROM event_zone \
                 WHERE map_version=?1 \
                 ORDER BY event_id",
            )
            .context("prepare event zones")?;
        let iter = stmt
            .query_map(params![map_version], |row| Ok((row.get(0)?, row.get(1)?)))
            .context("query event zones")?;
        let mut out = Vec::new();
        for row in iter {
            out.push(row.context("row event zone")?);
        }
        Ok(out)
    }

    pub fn upsert_water_tiles(&mut self, tiles: &[WaterTile]) -> Result<()> {
        let tx = self.conn.transaction().context("start tx")?;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO water_tiles (tile_px, tile_x, tile_y, water_count) \
                     VALUES (?1, ?2, ?3, ?4) \
                     ON CONFLICT(tile_px, tile_x, tile_y) DO UPDATE SET water_count=excluded.water_count",
                )
                .context("prepare water tiles")?;
            for tile in tiles {
                stmt.execute(params![
                    tile.tile_px,
                    tile.tile_x,
                    tile.tile_y,
                    tile.water_count
                ])
                .context("upsert water tile")?;
            }
        }
        tx.commit().context("commit water tiles")?;
        Ok(())
    }
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let current: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .context("read user_version")?;
    if current == SCHEMA_VERSION {
        return Ok(());
    }
    if current != 0 {
        bail!("unsupported schema version: {}", current);
    }

    let tx = conn.transaction().context("start migration tx")?;
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY,
            ts_utc INTEGER NOT NULL,
            fish_id INTEGER NOT NULL,
            world_x REAL NOT NULL,
            world_z REAL NOT NULL,
            px INTEGER,
            py INTEGER,
            water_px INTEGER,
            water_py INTEGER,
            tile_x INTEGER,
            tile_y INTEGER,
            water_ok INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS events_ts_idx ON events(ts_utc);
        CREATE INDEX IF NOT EXISTS events_fish_idx ON events(fish_id);
        CREATE INDEX IF NOT EXISTS events_water_ok_idx ON events(water_ok);

        CREATE TABLE IF NOT EXISTS event_zone (
            map_version TEXT NOT NULL,
            event_id INTEGER NOT NULL,
            zone_rgb_u32 INTEGER NOT NULL,
            PRIMARY KEY (map_version, event_id)
        );
        CREATE INDEX IF NOT EXISTS event_zone_zone_idx ON event_zone(map_version, zone_rgb_u32);

        CREATE TABLE IF NOT EXISTS water_tiles (
            tile_px INTEGER NOT NULL,
            tile_x INTEGER NOT NULL,
            tile_y INTEGER NOT NULL,
            water_count INTEGER NOT NULL,
            PRIMARY KEY (tile_px, tile_x, tile_y)
        );
        CREATE INDEX IF NOT EXISTS water_tiles_px_idx ON water_tiles(tile_px);
        ",
    )
    .context("create schema")?;
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)
        .context("set user_version")?;
    tx.commit().context("commit migration")?;
    Ok(())
}
