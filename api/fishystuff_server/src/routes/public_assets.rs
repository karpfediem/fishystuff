use fishystuff_api::models::fish::{FishListResponse, FishMapResponse, FishTableResponse};
use fishystuff_api::models::zone_stats::ZoneStatsResponse;
use fishystuff_core::asset_urls::normalize_public_asset_reference;

pub(crate) fn normalize_fish_list_icons(response: &mut FishListResponse) {
    for entry in &mut response.fish {
        if let Some(icon_url) = entry.icon_url.as_deref() {
            entry.icon_url = Some(normalize_public_asset_url(icon_url));
        }
    }
}

pub(crate) fn normalize_fish_table_icons(response: &mut FishTableResponse) {
    for entry in &mut response.fish {
        if let Some(icon) = entry.icon.as_deref() {
            entry.icon = Some(normalize_public_asset_url(icon));
        }
        if let Some(icon) = entry.encyclopedia_icon.as_deref() {
            entry.encyclopedia_icon = Some(normalize_public_asset_url(icon));
        }
    }
}

pub(crate) fn normalize_fish_map_icons(response: &mut FishMapResponse) {
    if let Some(icon) = response.icon.as_deref() {
        response.icon = Some(normalize_public_asset_url(icon));
    }
    if let Some(icon) = response.encyclopedia_icon.as_deref() {
        response.encyclopedia_icon = Some(normalize_public_asset_url(icon));
    }
}

pub(crate) fn normalize_zone_stats_icons(response: &mut ZoneStatsResponse) {
    for entry in &mut response.distribution {
        if let Some(icon_url) = entry.icon_url.as_deref() {
            entry.icon_url = Some(normalize_public_asset_url(icon_url));
        }
    }
}

pub(crate) fn normalize_public_asset_url(url: &str) -> String {
    let normalized = normalize_public_asset_reference(url);
    let trimmed = normalized.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("data:")
    {
        return trimmed.to_string();
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use fishystuff_api::models::fish::{FishMapResponse, FishTableEntry, FishTableResponse};
    use fishystuff_api::models::zone_stats::{ZoneFishEvidence, ZoneStatsResponse};

    use super::{
        normalize_fish_map_icons, normalize_fish_table_icons, normalize_public_asset_url,
        normalize_zone_stats_icons,
    };

    #[test]
    fn normalize_public_url_returns_relative_asset_paths() {
        assert_eq!(
            normalize_public_asset_url("/images/FishIcons/00008475.png"),
            "/images/FishIcons/00008475.png"
        );
        assert_eq!(
            normalize_public_asset_url("/tiles/mask/v1/0/1_17.png"),
            "/images/tiles/mask/v1/0/1_17.png"
        );
    }

    #[test]
    fn fish_map_and_zone_stats_icons_are_normalized() {
        let mut fish_map = FishMapResponse {
            encyclopedia_key: 8477,
            item_key: 8477,
            name: Some("Test Fish".to_string()),
            icon: Some("00008477.png".to_string()),
            encyclopedia_icon: Some("00821288.png".to_string()),
        };
        normalize_fish_map_icons(&mut fish_map);
        assert_eq!(
            fish_map.icon.as_deref(),
            Some("/images/FishIcons/00008477.png")
        );
        assert_eq!(
            fish_map.encyclopedia_icon.as_deref(),
            Some("/images/FishIcons/00821288.png")
        );

        let mut zone_stats = ZoneStatsResponse {
            distribution: vec![ZoneFishEvidence {
                fish_id: 8477,
                fish_name: Some("Test Fish".to_string()),
                icon_url: Some("00821288.png".to_string()),
                ..ZoneFishEvidence::default()
            }],
            ..ZoneStatsResponse::default()
        };
        normalize_zone_stats_icons(&mut zone_stats);
        assert_eq!(
            zone_stats.distribution[0].icon_url.as_deref(),
            Some("/images/FishIcons/00821288.png")
        );
    }

    #[test]
    fn fish_table_icons_are_normalized() {
        let mut response = FishTableResponse {
            fish: vec![FishTableEntry {
                encyclopedia_key: 8477,
                item_key: 8477,
                name: Some("Test Fish".to_string()),
                icon: Some("00008477.png".to_string()),
                encyclopedia_icon: Some("00821288.png".to_string()),
            }],
        };
        normalize_fish_table_icons(&mut response);
        assert_eq!(
            response.fish[0].icon.as_deref(),
            Some("/images/FishIcons/00008477.png")
        );
        assert_eq!(
            response.fish[0].encyclopedia_icon.as_deref(),
            Some("/images/FishIcons/00821288.png")
        );
    }
}
