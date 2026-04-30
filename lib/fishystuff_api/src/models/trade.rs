use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct TradeNpcCatalogSummary {
    pub character_function_trade_rows: usize,
    pub character_function_barter_rows: usize,
    pub character_function_trade_barter_overlap_rows: usize,
    pub selling_to_npc_rows: usize,
    pub title_trade_manager_rows: usize,
    pub candidate_npcs: usize,
    pub origin_regions: usize,
    pub destinations: usize,
    pub excluded_missing_spawn: usize,
    pub excluded_missing_trade_origin: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TradeNpcCatalogResponse {
    pub schema: String,
    pub version: u32,
    pub coordinate_space: String,
    #[serde(default)]
    pub sources: Vec<TradeNpcSourceDescriptor>,
    #[serde(default)]
    pub summary: TradeNpcCatalogSummary,
    #[serde(default)]
    pub origin_regions: Vec<TradeOriginRegion>,
    #[serde(default)]
    pub destinations: Vec<TradeNpcDestination>,
    #[serde(default)]
    pub excluded: Vec<ExcludedTradeNpc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TradeNpcSourceDescriptor {
    pub id: String,
    pub file: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TradeOriginRegion {
    pub region_id: u32,
    pub region_name: Option<String>,
    pub waypoint_id: Option<u32>,
    pub waypoint_name: Option<String>,
    pub world_x: f64,
    pub world_z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TradeNpcDestination {
    pub id: String,
    pub npc_key: u32,
    pub npc_name: String,
    pub role_source: String,
    #[serde(default)]
    pub source_tags: Vec<String>,
    pub trade: TradeNpcTradeInfo,
    pub npc_spawn: TradeNpcSpawn,
    pub assigned_region: TradeRegionWaypointRef,
    pub sell_destination_trade_origin: TradeRegionWaypointRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TradeNpcTradeInfo {
    pub item_main_group_key: Option<u32>,
    pub trade_group_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TradeNpcSpawn {
    pub region_id: u32,
    pub region_name: Option<String>,
    pub world_x: f64,
    pub world_y: f64,
    pub world_z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TradeRegionWaypointRef {
    pub region_id: Option<u32>,
    pub region_name: Option<String>,
    pub waypoint_id: Option<u32>,
    pub waypoint_name: Option<String>,
    pub world_x: Option<f64>,
    pub world_z: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExcludedTradeNpc {
    pub npc_key: u32,
    pub npc_name: String,
    pub reason: String,
    #[serde(default)]
    pub source_tags: Vec<String>,
}
