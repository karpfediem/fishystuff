use fishystuff_api::models::fish::FishListResponse;
use fishystuff_core::fish_icons::fish_item_icon_path;

use super::state::FishEntry;

const PROD_CDN_BASE_URL: &str = "https://cdn.fishystuff.fish";

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
                grade: entry.grade,
                is_prize: entry.is_prize.unwrap_or(false),
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.name_lower.cmp(&right.name_lower));
    entries
}

pub(crate) fn fish_item_icon_url(item_id: i32) -> Option<String> {
    let path = fish_item_icon_path(item_id);
    if path.is_empty() {
        return None;
    }
    let base_url = configured_cdn_base_url().unwrap_or_else(|| PROD_CDN_BASE_URL.to_string());
    Some(format!("{}{}", base_url.trim_end_matches('/'), path))
}

#[cfg(target_arch = "wasm32")]
fn configured_cdn_base_url() -> Option<String> {
    use wasm_bindgen::JsValue;

    let window = web_sys::window()?;
    let value = js_sys::Reflect::get(
        window.as_ref(),
        &JsValue::from_str("__fishystuffCdnBaseUrl"),
    )
    .ok()?;
    let value = value.as_string()?;
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn configured_cdn_base_url() -> Option<String> {
    Some(PROD_CDN_BASE_URL.to_string())
}

#[cfg(test)]
mod tests {
    use super::{build_fish_catalog_entries, fish_item_icon_url};
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
    fn fish_item_icon_urls_use_the_cdn_base() {
        assert_eq!(
            fish_item_icon_url(8289).as_deref(),
            Some("https://cdn.fishystuff.fish/images/items/00008289.webp")
        );
    }
}
