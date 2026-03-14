use fishystuff_core::asset_urls::normalize_public_asset_reference;

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
    use super::normalize_public_asset_url;

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
        assert_eq!(
            normalize_public_asset_url("https://cdn.example.com/region_groups/v1.geojson"),
            "/region_groups/v1.geojson"
        );
    }
}
