use fishystuff_api::models::fish::FishListResponse;

use super::state::FishEntry;

pub(crate) fn build_fish_catalog_entries(fish_response: FishListResponse) -> Vec<FishEntry> {
    let mut entries = fish_response
        .fish
        .into_iter()
        .map(|entry| {
            let canonical_id = entry.encyclopedia_key.unwrap_or(entry.item_id);
            let name = entry.name;
            FishEntry {
                id: canonical_id,
                item_id: entry.item_id,
                encyclopedia_key: entry.encyclopedia_key,
                encyclopedia_id: entry.encyclopedia_id,
                name_lower: name.to_lowercase(),
                name,
                is_prize: entry.is_prize.unwrap_or(false),
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.name_lower.cmp(&right.name_lower));
    entries
}

pub(crate) fn bevy_public_asset_path(raw: &str) -> String {
    raw.to_string()
}

#[cfg(test)]
mod tests {
    use super::{bevy_public_asset_path, build_fish_catalog_entries};
    use fishystuff_api::models::fish::{FishEntry as ApiFishEntry, FishListResponse};

    #[test]
    fn fish_catalog_uses_item_icons_and_keeps_both_ids() {
        let response = FishListResponse {
            fish: vec![ApiFishEntry {
                item_id: 820303,
                encyclopedia_key: Some(303),
                encyclopedia_id: Some(9434),
                name: "Pinecone Fish".to_string(),
                grade: Some("Rare".to_string()),
                is_prize: Some(false),
                is_dried: false,
                catch_methods: Vec::new(),
                vendor_price: None,
            }],
            ..FishListResponse::default()
        };

        let entries = build_fish_catalog_entries(response);
        let entry = entries.first().expect("expected fish entry");
        assert_eq!(entry.id, 303);
        assert_eq!(entry.item_id, 820303);
        assert_eq!(entry.encyclopedia_key, Some(303));
        assert_eq!(entry.encyclopedia_id, Some(9434));
    }

    #[test]
    fn bevy_public_asset_paths_are_passthrough() {
        assert_eq!(
            bevy_public_asset_path("/images/FishIcons/00008289.png"),
            "/images/FishIcons/00008289.png"
        );
    }
}
