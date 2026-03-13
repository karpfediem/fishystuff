pub mod sqlite;

#[derive(Debug, Clone)]
pub struct Event {
    pub ts_utc: i64,
    pub fish_id: i32,
    pub world_x: f64,
    pub world_z: f64,
    pub px: Option<i32>,
    pub py: Option<i32>,
    pub water_px: Option<i32>,
    pub water_py: Option<i32>,
    pub tile_x: Option<i32>,
    pub tile_y: Option<i32>,
    pub water_ok: bool,
}

#[derive(Debug, Clone)]
pub struct WaterTile {
    pub tile_px: i32,
    pub tile_x: i32,
    pub tile_y: i32,
    pub water_count: i32,
}

#[derive(Debug, Clone)]
pub struct WaterEvent {
    pub id: i64,
    pub water_px: i32,
    pub water_py: i32,
}

#[derive(Debug, Clone)]
pub struct EventZoneRow {
    pub ts_utc: i64,
    pub fish_id: i32,
    pub tile_x: i32,
    pub tile_y: i32,
    pub zone_rgb_u32: u32,
}

#[derive(Debug, Clone)]
pub struct EventPoint {
    pub water_px: i32,
    pub water_py: i32,
    pub fish_id: i32,
}
