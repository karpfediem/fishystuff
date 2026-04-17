use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FishEntry {
    pub item_id: i32,
    #[serde(default)]
    pub encyclopedia_key: Option<i32>,
    #[serde(default)]
    pub encyclopedia_id: Option<i32>,
    pub name: String,
    pub grade: Option<String>,
    pub is_prize: Option<bool>,
    #[serde(default)]
    pub is_dried: bool,
    #[serde(default)]
    pub catch_methods: Vec<String>,
    pub vendor_price: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FishBestSpotEntry {
    pub zone_rgb: String,
    pub zone_name: String,
    #[serde(default)]
    pub db_groups: Vec<String>,
    #[serde(default)]
    pub community_groups: Vec<String>,
    #[serde(default)]
    pub has_ranking_presence: bool,
    #[serde(default)]
    pub ranking_observation_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FishBestSpotsResponse {
    pub revision: String,
    pub item_id: i32,
    pub count: usize,
    #[serde(default)]
    pub spots: Vec<FishBestSpotEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommunityFishZoneSupportEntry {
    pub item_id: i32,
    #[serde(default)]
    pub zone_rgbs: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommunityFishZoneSupportResponse {
    pub revision: String,
    pub count: usize,
    #[serde(default)]
    pub fish: Vec<CommunityFishZoneSupportEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FishListResponse {
    pub revision: String,
    pub count: usize,
    #[serde(default)]
    pub fish: Vec<FishEntry>,
}
