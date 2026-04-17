use serde::{Deserialize, Serialize};

use crate::ids::Timestamp;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventSourceKind {
    #[default]
    Ranking,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventsQueryMode {
    Raw,
    GridAggregate,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MapBboxPx {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventPointCompact {
    pub event_id: i64,
    pub fish_id: i32,
    pub ts_utc: Timestamp,
    pub map_px_x: i32,
    pub map_px_y: i32,
    pub length_milli: i32,
    #[serde(default)]
    pub world_x: Option<i32>,
    #[serde(default)]
    pub world_z: Option<i32>,
    #[serde(default)]
    pub zone_rgb_u32: Option<u32>,
    #[serde(default)]
    pub zone_rgbs: Vec<u32>,
    #[serde(default)]
    pub full_zone_rgbs: Vec<u32>,
    #[serde(default)]
    pub source_kind: Option<EventSourceKind>,
    #[serde(default)]
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventsSnapshotMetaResponse {
    pub revision: String,
    pub event_count: usize,
    pub source_kind: EventSourceKind,
    #[serde(default)]
    pub last_updated_utc: Option<String>,
    pub snapshot_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventsSnapshotResponse {
    pub revision: String,
    pub event_count: usize,
    pub source_kind: EventSourceKind,
    #[serde(default)]
    pub events: Vec<EventPointCompact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventPoint {
    pub event_id: i64,
    pub fish_id: i32,
    pub ts_utc: Timestamp,
    pub map_px_x: i32,
    pub map_px_y: i32,
    pub length_milli: i32,
    #[serde(default)]
    pub world_x: Option<i32>,
    #[serde(default)]
    pub world_z: Option<i32>,
    #[serde(default)]
    pub zone_rgb_u32: Option<u32>,
    #[serde(default)]
    pub zone_rgbs: Vec<u32>,
    #[serde(default)]
    pub full_zone_rgbs: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AggregatedEventPoint {
    pub map_px_x: i32,
    pub map_px_y: i32,
    #[serde(default)]
    pub world_x: Option<i32>,
    #[serde(default)]
    pub world_z: Option<i32>,
    pub sample_count: u32,
    #[serde(default)]
    pub top_fish_id: Option<i32>,
    #[serde(default)]
    pub ts_min_utc: Option<String>,
    #[serde(default)]
    pub ts_max_utc: Option<String>,
}
