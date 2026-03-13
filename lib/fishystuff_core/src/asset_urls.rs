pub fn normalize_site_asset_path(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }

    if let Some(rest) = trimmed.strip_prefix("/map/") {
        return normalize_prefixed_site_asset_path(rest, true)
            .unwrap_or_else(|| trimmed.to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("map/") {
        return normalize_prefixed_site_asset_path(rest, false)
            .unwrap_or_else(|| trimmed.to_string());
    }
    if let Some(rest) = trimmed.strip_prefix('/') {
        return normalize_prefixed_site_asset_path(rest, true)
            .unwrap_or_else(|| trimmed.to_string());
    }

    normalize_prefixed_site_asset_path(trimmed, false).unwrap_or_else(|| trimmed.to_string())
}

fn normalize_prefixed_site_asset_path(path: &str, absolute: bool) -> Option<String> {
    let is_legacy_static_path = path.starts_with("tiles/")
        || path.starts_with("terrain/")
        || path.starts_with("terrain_drape/");
    if !is_legacy_static_path {
        return None;
    }

    let prefix = if absolute { "/" } else { "" };
    Some(format!("{prefix}images/{path}"))
}

#[cfg(test)]
mod tests {
    use super::normalize_site_asset_path;

    #[test]
    fn normalizes_legacy_site_asset_paths() {
        assert_eq!(
            normalize_site_asset_path("/tiles/mask/v1/0/22_3.png"),
            "/images/tiles/mask/v1/0/22_3.png"
        );
        assert_eq!(
            normalize_site_asset_path("tiles/mask/v1/0/22_3.png"),
            "images/tiles/mask/v1/0/22_3.png"
        );
        assert_eq!(
            normalize_site_asset_path("/terrain/v1/manifest.json"),
            "/images/terrain/v1/manifest.json"
        );
        assert_eq!(
            normalize_site_asset_path("/terrain_drape/minimap/v1/manifest.json"),
            "/images/terrain_drape/minimap/v1/manifest.json"
        );
        assert_eq!(
            normalize_site_asset_path("/map/terrain/v1/manifest.json"),
            "/images/terrain/v1/manifest.json"
        );
    }

    #[test]
    fn leaves_non_legacy_paths_unchanged() {
        assert_eq!(
            normalize_site_asset_path("/images/tiles/minimap/v1/tileset.json"),
            "/images/tiles/minimap/v1/tileset.json"
        );
        assert_eq!(
            normalize_site_asset_path("/region_groups/v1.geojson"),
            "/region_groups/v1.geojson"
        );
        assert_eq!(
            normalize_site_asset_path("https://cdn.example.com/images/terrain/v1/manifest.json"),
            "https://cdn.example.com/images/terrain/v1/manifest.json"
        );
    }
}
