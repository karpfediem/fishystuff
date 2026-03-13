use std::collections::HashMap;

use fishystuff_api::models::fish::{FishListResponse, FishTableResponse};

use crate::prelude::*;

#[derive(Resource)]
pub struct FishCatalog {
    pub status: String,
    pub entries: Vec<FishEntry>,
    pub icon_by_id: HashMap<i32, String>,
}

impl Default for FishCatalog {
    fn default() -> Self {
        Self {
            status: "fish: pending".to_string(),
            entries: Vec::new(),
            icon_by_id: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FishEntry {
    pub id: i32,
    pub item_id: i32,
    pub name: String,
    pub name_lower: String,
    pub icon_url: Option<String>,
    pub is_prize: bool,
}

#[derive(Debug)]
pub(crate) struct FishCatalogPayload {
    pub(crate) fish: FishListResponse,
    pub(crate) fish_table: FishTableResponse,
}

#[derive(Debug, Default)]
pub(crate) struct FishTableIndex {
    pub(crate) canonical_by_item_id: HashMap<i32, i32>,
    pub(crate) fallback_by_canonical_id: HashMap<i32, FishTableFallback>,
    pub(crate) icon_by_id: HashMap<i32, String>,
}

#[derive(Debug, Clone)]
pub(crate) struct FishTableFallback {
    pub(crate) item_id: i32,
    pub(crate) name: String,
}
