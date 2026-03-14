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
pub struct FishListResponse {
    pub revision: String,
    pub count: usize,
    #[serde(default)]
    pub fish: Vec<FishEntry>,
}
