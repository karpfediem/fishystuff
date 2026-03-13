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

pub fn normalize_public_asset_reference(value: &str) -> String {
    let raw = value.trim();
    if raw.is_empty() || raw.starts_with("data:") {
        return raw.to_string();
    }

    if let Some(path) = extract_absolute_asset_path(raw) {
        if let Some(normalized) = normalize_known_public_asset_path(&path) {
            return normalized;
        }
        return raw.to_string();
    }

    normalize_known_public_asset_path(raw).unwrap_or_else(|| raw.to_string())
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

fn normalize_known_public_asset_path(raw: &str) -> Option<String> {
    if let Some(path) = normalize_fish_icon_path(raw) {
        return Some(path);
    }

    let normalized = normalize_site_asset_path(raw);
    if normalized != raw {
        return Some(normalized);
    }

    if raw.starts_with("/images/") || raw.starts_with("/region_groups/") {
        return Some(raw.to_string());
    }
    if raw.starts_with("images/") || raw.starts_with("region_groups/") {
        return Some(format!("/{raw}"));
    }

    None
}

fn normalize_fish_icon_path(raw: &str) -> Option<String> {
    if raw.starts_with("/images/FishIcons/") {
        return Some(raw.to_string());
    }
    if let Some(rest) = raw.strip_prefix("images/FishIcons/") {
        return Some(format!("/images/FishIcons/{rest}"));
    }
    if let Some(rest) = raw.strip_prefix("/FishIcons/") {
        return Some(format!("/images/FishIcons/{rest}"));
    }
    if let Some(rest) = raw.strip_prefix("FishIcons/") {
        return Some(format!("/images/FishIcons/{rest}"));
    }
    if looks_like_icon_filename(raw) && !raw.contains('/') {
        return Some(format!("/images/FishIcons/{raw}"));
    }
    None
}

fn extract_absolute_asset_path(raw: &str) -> Option<String> {
    if !(raw.starts_with("http://") || raw.starts_with("https://")) {
        return None;
    }
    let (_, rest) = raw.split_once("://")?;
    let (_, path_and_more) = rest.split_once('/')?;
    let path = path_and_more
        .split(['?', '#'])
        .next()
        .map(str::trim)
        .filter(|path| !path.is_empty())?;
    Some(format!("/{path}"))
}

fn looks_like_icon_filename(raw: &str) -> bool {
    matches!(
        raw.to_ascii_lowercase()
            .rsplit_once('.')
            .map(|(_, ext)| ext),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "avif" | "svg")
    )
}

#[cfg(test)]
mod tests {
    use super::{normalize_public_asset_reference, normalize_site_asset_path};

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

    #[test]
    fn normalizes_public_asset_references_to_relative_paths() {
        assert_eq!(
            normalize_public_asset_reference(
                "https://cdn.example.com/images/terrain/v1/manifest.json"
            ),
            "/images/terrain/v1/manifest.json"
        );
        assert_eq!(
            normalize_public_asset_reference("https://cdn.example.com/region_groups/v1.geojson"),
            "/region_groups/v1.geojson"
        );
        assert_eq!(
            normalize_public_asset_reference(
                "https://cdn.example.com/images/FishIcons/00008475.png"
            ),
            "/images/FishIcons/00008475.png"
        );
        assert_eq!(
            normalize_public_asset_reference("00820994.png"),
            "/images/FishIcons/00820994.png"
        );
    }
}
