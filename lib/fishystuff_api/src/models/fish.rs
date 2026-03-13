use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FishEntry {
    pub fish_id: i32,
    #[serde(default)]
    pub encyclopedia_key: Option<i32>,
    pub name: String,
    pub grade: Option<String>,
    pub is_prize: Option<bool>,
    pub icon_url: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FishTableEntry {
    pub encyclopedia_key: i32,
    pub item_key: i32,
    pub name: Option<String>,
    pub icon: Option<String>,
    pub encyclopedia_icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FishTableResponse {
    #[serde(default)]
    pub fish: Vec<FishTableEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FishMapResponse {
    pub encyclopedia_key: i32,
    pub item_key: i32,
    pub name: Option<String>,
    pub icon: Option<String>,
    pub encyclopedia_icon: Option<String>,
}
