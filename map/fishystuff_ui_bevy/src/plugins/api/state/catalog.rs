use std::collections::HashMap;

use fishystuff_api::models::fish::FishListResponse;
use fishystuff_core::fish_icons::fish_item_icon_path;

use crate::prelude::*;

#[derive(Resource)]
pub struct FishCatalog {
    pub status: String,
    pub entries: Vec<FishEntry>,
    aliases: HashMap<i32, usize>,
}

impl Default for FishCatalog {
    fn default() -> Self {
        Self {
            status: "fish: pending".to_string(),
            entries: Vec::new(),
            aliases: HashMap::new(),
        }
    }
}

impl FishCatalog {
    pub fn replace(&mut self, entries: Vec<FishEntry>) {
        self.aliases.clear();
        self.aliases.reserve(entries.len() * 2);
        for (idx, entry) in entries.iter().enumerate() {
            self.aliases.insert(entry.item_id, idx);
            if let Some(encyclopedia_key) = entry.encyclopedia_key {
                self.aliases.entry(encyclopedia_key).or_insert(idx);
            }
        }
        self.entries = entries;
    }

    pub fn entry_for_fish(&self, fish_id: i32) -> Option<&FishEntry> {
        self.aliases
            .get(&fish_id)
            .and_then(|idx| self.entries.get(*idx))
            .or_else(|| self.entries.iter().find(|entry| entry.id == fish_id))
    }

    pub fn item_icon_path_for_fish(&self, fish_id: i32) -> Option<String> {
        self.entry_for_fish(fish_id)
            .map(|entry| fish_item_icon_path(entry.item_id))
    }

    pub fn item_id_for_fish(&self, fish_id: i32) -> Option<i32> {
        self.entry_for_fish(fish_id).map(|entry| entry.item_id)
    }
}

#[derive(Clone, Debug)]
pub struct FishEntry {
    pub id: i32,
    pub item_id: i32,
    pub encyclopedia_key: Option<i32>,
    pub encyclopedia_id: Option<i32>,
    pub name: String,
    pub name_lower: String,
    pub grade: Option<String>,
    pub is_prize: bool,
}

#[derive(Debug)]
pub(crate) struct FishCatalogPayload {
    pub(crate) fish: FishListResponse,
}

#[cfg(test)]
mod tests {
    use super::{FishCatalog, FishEntry};

    #[test]
    fn icon_lookup_resolves_item_and_encyclopedia_aliases() {
        let mut catalog = FishCatalog::default();
        catalog.replace(vec![FishEntry {
            id: 88,
            item_id: 8289,
            encyclopedia_key: Some(88),
            encyclopedia_id: Some(8588),
            name: "Barbel Steed".to_string(),
            name_lower: "barbel steed".to_string(),
            grade: Some("Rare".to_string()),
            is_prize: false,
        }]);

        assert_eq!(
            catalog.item_icon_path_for_fish(88).as_deref(),
            Some("/images/FishIcons/00008289.png")
        );
        assert_eq!(
            catalog.item_icon_path_for_fish(8289).as_deref(),
            Some("/images/FishIcons/00008289.png")
        );
    }
}
