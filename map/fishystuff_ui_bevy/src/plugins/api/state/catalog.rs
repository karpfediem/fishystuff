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

impl FishCatalog {
    pub fn icon_url_for_fish(&self, fish_id: i32) -> Option<String> {
        self.entries
            .iter()
            .find(|entry| entry.id == fish_id)
            .and_then(|entry| entry.icon_url.clone())
            .or_else(|| self.icon_by_id.get(&fish_id).cloned())
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|entry| entry.item_id == fish_id)
                    .and_then(|entry| entry.icon_url.clone())
            })
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{FishCatalog, FishEntry};

    #[test]
    fn icon_lookup_prefers_catalog_entry_icon_for_canonical_ids() {
        let catalog = FishCatalog {
            status: "ok".to_string(),
            entries: vec![FishEntry {
                id: 88,
                item_id: 8289,
                name: "Barbel Steed".to_string(),
                name_lower: "barbel steed".to_string(),
                icon_url: Some("/images/FishIcons/IC_08588.png".to_string()),
                is_prize: false,
            }],
            icon_by_id: HashMap::from([
                (88, "/images/FishIcons/00008289.png".to_string()),
                (8289, "/images/FishIcons/IC_08588.png".to_string()),
            ]),
        };

        assert_eq!(
            catalog.icon_url_for_fish(88).as_deref(),
            Some("/images/FishIcons/IC_08588.png")
        );
    }

    #[test]
    fn icon_lookup_falls_back_to_item_id_mappings() {
        let catalog = FishCatalog {
            status: "ok".to_string(),
            entries: vec![FishEntry {
                id: 88,
                item_id: 8289,
                name: "Barbel Steed".to_string(),
                name_lower: "barbel steed".to_string(),
                icon_url: Some("/images/FishIcons/IC_08588.png".to_string()),
                is_prize: false,
            }],
            icon_by_id: HashMap::new(),
        };

        assert_eq!(
            catalog.icon_url_for_fish(8289).as_deref(),
            Some("/images/FishIcons/IC_08588.png")
        );
    }
}
