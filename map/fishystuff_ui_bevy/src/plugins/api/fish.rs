use std::collections::{HashMap, HashSet};

use fishystuff_api::models::fish::{FishListResponse, FishTableResponse};

use super::state::{FishEntry, FishTableFallback, FishTableIndex};

pub(crate) fn build_fish_table_index(response: &FishTableResponse) -> FishTableIndex {
    let mut index = FishTableIndex::default();
    for entry in &response.fish {
        index
            .canonical_by_item_id
            .insert(entry.item_key, entry.encyclopedia_key);

        if let Some(name) = entry
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            index
                .fallback_by_canonical_id
                .entry(entry.encyclopedia_key)
                .or_insert_with(|| FishTableFallback {
                    item_id: entry.item_key,
                    name: name.to_string(),
                });
        }

        let icon_url = normalize_fish_icon_asset_url(entry.icon.as_deref())
            .or_else(|| normalize_fish_icon_asset_url(entry.encyclopedia_icon.as_deref()));
        if let Some(icon_url) = icon_url {
            index
                .icon_by_id
                .entry(entry.item_key)
                .or_insert_with(|| icon_url.clone());
            index
                .icon_by_id
                .entry(entry.encyclopedia_key)
                .or_insert(icon_url);
        }
    }
    index
}

pub(crate) fn build_fish_catalog_entries(
    fish_response: FishListResponse,
    fish_table_response: FishTableResponse,
) -> (Vec<FishEntry>, HashMap<i32, String>) {
    let table_index = build_fish_table_index(&fish_table_response);
    let mut entries =
        Vec::with_capacity(fish_response.fish.len() + table_index.fallback_by_canonical_id.len());
    let mut icon_by_id = table_index.icon_by_id.clone();
    let mut seen_canonical_ids = HashSet::with_capacity(entries.capacity());

    for entry in fish_response.fish {
        let raw_id = entry.fish_id;
        let canonical_id = entry
            .encyclopedia_key
            .or_else(|| table_index.canonical_by_item_id.get(&raw_id).copied())
            .unwrap_or(raw_id);
        let name = entry.name;
        let icon_url = normalize_fish_icon_asset_url(entry.icon_url.as_deref())
            .or_else(|| table_index.icon_by_id.get(&canonical_id).cloned())
            .or_else(|| table_index.icon_by_id.get(&raw_id).cloned());
        if let Some(url) = icon_url.clone() {
            icon_by_id.insert(raw_id, url.clone());
            icon_by_id.insert(canonical_id, url);
        }
        seen_canonical_ids.insert(canonical_id);
        entries.push(FishEntry {
            id: canonical_id,
            item_id: raw_id,
            name_lower: name.to_lowercase(),
            name,
            icon_url,
            is_prize: entry.is_prize.unwrap_or(false),
        });
    }

    for (canonical_id, fallback) in table_index.fallback_by_canonical_id {
        if seen_canonical_ids.contains(&canonical_id) {
            continue;
        }
        let icon_url = icon_by_id
            .get(&canonical_id)
            .cloned()
            .or_else(|| icon_by_id.get(&fallback.item_id).cloned());
        entries.push(FishEntry {
            id: canonical_id,
            item_id: fallback.item_id,
            name_lower: fallback.name.to_lowercase(),
            name: fallback.name,
            icon_url,
            is_prize: false,
        });
    }

    entries.sort_by(|left, right| left.name_lower.cmp(&right.name_lower));
    (entries, icon_by_id)
}

pub(crate) fn normalize_fish_icon_asset_url(value: Option<&str>) -> Option<String> {
    let raw = value?.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.starts_with("/images/") {
        return Some(raw.to_string());
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return Some(raw.to_string());
    }
    let lower = raw.to_ascii_lowercase();
    if matches!(
        lower.rsplit_once('.').map(|(_, ext)| ext),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "avif" | "svg")
    ) && !raw.contains('/')
    {
        return Some(format!("/images/FishIcons/{raw}"));
    }
    Some(raw.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        build_fish_catalog_entries, build_fish_table_index, normalize_fish_icon_asset_url,
    };
    use fishystuff_api::models::fish::{
        FishEntry as ApiFishEntry, FishListResponse, FishTableEntry, FishTableResponse,
    };

    #[test]
    fn fish_icon_urls_preserve_absolute_cdn_paths() {
        assert_eq!(
            normalize_fish_icon_asset_url(Some(
                "https://api.example.test/images/FishIcons/00008475.png"
            ))
            .as_deref(),
            Some("https://api.example.test/images/FishIcons/00008475.png")
        );
        assert_eq!(
            normalize_fish_icon_asset_url(Some("/images/FishIcons/00008475.png")).as_deref(),
            Some("/images/FishIcons/00008475.png")
        );
        assert_eq!(
            normalize_fish_icon_asset_url(Some("00008475.png")).as_deref(),
            Some("/images/FishIcons/00008475.png")
        );
    }

    #[test]
    fn fish_table_index_maps_item_and_encyclopedia_ids_to_icons() {
        let response = FishTableResponse {
            fish: vec![FishTableEntry {
                encyclopedia_key: 247,
                item_key: 820998,
                name: Some("Test".to_string()),
                icon: Some("00820998.png".to_string()),
                encyclopedia_icon: None,
            }],
        };

        let index = build_fish_table_index(&response);
        assert_eq!(index.canonical_by_item_id.get(&820998), Some(&247));
        assert_eq!(
            index.icon_by_id.get(&820998).map(String::as_str),
            Some("/images/FishIcons/00820998.png")
        );
        assert_eq!(
            index.icon_by_id.get(&247).map(String::as_str),
            Some("/images/FishIcons/00820998.png")
        );
    }

    #[test]
    fn fish_catalog_backfills_missing_encyclopedia_names_from_fish_table() {
        let fish = FishListResponse::default();
        let fish_table = FishTableResponse {
            fish: vec![FishTableEntry {
                encyclopedia_key: 303,
                item_key: 820303,
                name: Some("Pinecone Fish".to_string()),
                icon: Some("00820303.png".to_string()),
                encyclopedia_icon: None,
            }],
        };

        let (entries, icon_by_id) = build_fish_catalog_entries(fish, fish_table);
        let pinecone = entries
            .iter()
            .find(|entry| entry.id == 303)
            .expect("expected Pinecone Fish fallback entry");

        assert_eq!(pinecone.item_id, 820303);
        assert_eq!(pinecone.name, "Pinecone Fish");
        assert_eq!(
            pinecone.icon_url.as_deref(),
            Some("/images/FishIcons/00820303.png")
        );
        assert_eq!(
            icon_by_id.get(&303).map(String::as_str),
            Some("/images/FishIcons/00820303.png")
        );
    }

    #[test]
    fn fish_catalog_does_not_duplicate_existing_encyclopedia_entries() {
        let fish = FishListResponse {
            fish: vec![ApiFishEntry {
                fish_id: 820303,
                encyclopedia_key: Some(303),
                name: "Pinecone Fish".to_string(),
                grade: Some("Rare".to_string()),
                is_prize: Some(false),
                icon_url: None,
                is_dried: false,
                catch_methods: Vec::new(),
                vendor_price: None,
            }],
            ..FishListResponse::default()
        };
        let fish_table = FishTableResponse {
            fish: vec![FishTableEntry {
                encyclopedia_key: 303,
                item_key: 820303,
                name: Some("Pinecone Fish".to_string()),
                icon: Some("00820303.png".to_string()),
                encyclopedia_icon: None,
            }],
        };

        let (entries, _) = build_fish_catalog_entries(fish, fish_table);

        assert_eq!(entries.iter().filter(|entry| entry.id == 303).count(), 1);
    }
}
