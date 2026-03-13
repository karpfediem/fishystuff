use axum::http::{HeaderMap, header};

use fishystuff_api::models::fish::{FishListResponse, FishMapResponse, FishTableResponse};
use fishystuff_api::models::zone_stats::ZoneStatsResponse;
use fishystuff_core::asset_urls::normalize_site_asset_path;

pub(crate) fn absolutize_fish_list_icons(
    headers: &HeaderMap,
    response: &mut FishListResponse,
    configured_base: Option<&str>,
) {
    for entry in &mut response.fish {
        if let Some(icon_url) = entry.icon_url.as_deref() {
            entry.icon_url = Some(resolve_public_url(headers, icon_url, configured_base));
        }
    }
}

pub(crate) fn absolutize_fish_table_icons(
    headers: &HeaderMap,
    response: &mut FishTableResponse,
    configured_base: Option<&str>,
) {
    for entry in &mut response.fish {
        if let Some(icon) = entry.icon.as_deref() {
            entry.icon = Some(resolve_public_url(headers, icon, configured_base));
        }
        if let Some(icon) = entry.encyclopedia_icon.as_deref() {
            entry.encyclopedia_icon = Some(resolve_public_url(headers, icon, configured_base));
        }
    }
}

pub(crate) fn absolutize_fish_map_icons(
    headers: &HeaderMap,
    response: &mut FishMapResponse,
    configured_base: Option<&str>,
) {
    if let Some(icon) = response.icon.as_deref() {
        response.icon = Some(resolve_public_url(headers, icon, configured_base));
    }
    if let Some(icon) = response.encyclopedia_icon.as_deref() {
        response.encyclopedia_icon = Some(resolve_public_url(headers, icon, configured_base));
    }
}

pub(crate) fn absolutize_zone_stats_icons(
    headers: &HeaderMap,
    response: &mut ZoneStatsResponse,
    configured_base: Option<&str>,
) {
    for entry in &mut response.distribution {
        if let Some(icon_url) = entry.icon_url.as_deref() {
            entry.icon_url = Some(resolve_public_url(headers, icon_url, configured_base));
        }
    }
}

pub(crate) fn resolve_public_url(
    headers: &HeaderMap,
    url: &str,
    configured_base: Option<&str>,
) -> String {
    let normalized = normalize_public_asset_path(url);
    let trimmed = normalized.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("data:")
    {
        return trimmed.to_string();
    }

    if let Some(base) = configured_base
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let base = base.trim_end_matches('/');
        if trimmed.starts_with('/') {
            return format!("{base}{trimmed}");
        }
        return format!("{base}/{}", trimmed.trim_start_matches('/'));
    }

    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("http");
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(header::HOST))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match host {
        Some(host) if trimmed.starts_with('/') => format!("{proto}://{host}{trimmed}"),
        Some(host) => format!("{proto}://{host}/{}", trimmed.trim_start_matches('/')),
        None => trimmed.to_string(),
    }
}

fn normalize_public_asset_path(value: &str) -> String {
    let raw = value.trim();
    if raw.is_empty() || raw.starts_with("data:") {
        return raw.to_string();
    }

    if let Some(path) = extract_absolute_asset_path(raw) {
        if let Some(normalized) = normalize_known_asset_path(&path) {
            return normalized;
        }
        return raw.to_string();
    }

    normalize_known_asset_path(raw).unwrap_or_else(|| raw.to_string())
}

fn normalize_known_asset_path(raw: &str) -> Option<String> {
    if let Some(path) = normalize_fish_icon_path(raw) {
        return Some(path);
    }

    let normalized = normalize_site_asset_path(raw);
    if normalized != raw {
        return Some(normalized);
    }
    if raw.starts_with("/images/") {
        return Some(raw.to_string());
    }
    if raw.starts_with("images/") {
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
    use axum::http::{HeaderMap, HeaderValue, header};

    use fishystuff_api::models::fish::{FishMapResponse, FishTableEntry, FishTableResponse};
    use fishystuff_api::models::zone_stats::{ZoneFishEvidence, ZoneStatsResponse};

    use super::{
        absolutize_fish_map_icons, absolutize_fish_table_icons, absolutize_zone_stats_icons,
        resolve_public_url,
    };

    fn headers(host: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_str(host).unwrap());
        headers
    }

    #[test]
    fn resolve_public_url_uses_configured_cdn_base() {
        let headers = headers("127.0.0.1:8080");
        assert_eq!(
            resolve_public_url(
                &headers,
                "/images/FishIcons/00008475.png",
                Some("http://127.0.0.1:4040"),
            ),
            "http://127.0.0.1:4040/images/FishIcons/00008475.png"
        );
        assert_eq!(
            resolve_public_url(
                &headers,
                "/tiles/mask/v1/0/1_17.png",
                Some("http://127.0.0.1:4040"),
            ),
            "http://127.0.0.1:4040/images/tiles/mask/v1/0/1_17.png"
        );
    }

    #[test]
    fn fish_map_and_zone_stats_icons_are_absolutized() {
        let headers = headers("127.0.0.1:8080");
        let mut fish_map = FishMapResponse {
            encyclopedia_key: 8477,
            item_key: 8477,
            name: Some("Test Fish".to_string()),
            icon: Some("00008477.png".to_string()),
            encyclopedia_icon: Some("00821288.png".to_string()),
        };
        absolutize_fish_map_icons(&headers, &mut fish_map, Some("http://127.0.0.1:4040"));
        assert_eq!(
            fish_map.icon.as_deref(),
            Some("http://127.0.0.1:4040/images/FishIcons/00008477.png")
        );
        assert_eq!(
            fish_map.encyclopedia_icon.as_deref(),
            Some("http://127.0.0.1:4040/images/FishIcons/00821288.png")
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
        absolutize_zone_stats_icons(&headers, &mut zone_stats, Some("http://127.0.0.1:4040"));
        assert_eq!(
            zone_stats.distribution[0].icon_url.as_deref(),
            Some("http://127.0.0.1:4040/images/FishIcons/00821288.png")
        );
    }

    #[test]
    fn fish_table_icons_are_absolutized() {
        let headers = headers("127.0.0.1:8080");
        let mut response = FishTableResponse {
            fish: vec![FishTableEntry {
                encyclopedia_key: 8477,
                item_key: 8477,
                name: Some("Test Fish".to_string()),
                icon: Some("00008477.png".to_string()),
                encyclopedia_icon: Some("00821288.png".to_string()),
            }],
        };
        absolutize_fish_table_icons(&headers, &mut response, Some("http://127.0.0.1:4040"));
        assert_eq!(
            response.fish[0].icon.as_deref(),
            Some("http://127.0.0.1:4040/images/FishIcons/00008477.png")
        );
        assert_eq!(
            response.fish[0].encyclopedia_icon.as_deref(),
            Some("http://127.0.0.1:4040/images/FishIcons/00821288.png")
        );
    }
}
