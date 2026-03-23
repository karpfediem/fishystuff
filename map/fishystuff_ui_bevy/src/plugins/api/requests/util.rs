use fishystuff_api::ids::MapVersionId;
use fishystuff_api::models::layers::LayersResponse;
use fishystuff_api::models::meta::MetaResponse;
use fishystuff_api::models::zone_stats::ZoneStatsRequest;
use fishystuff_api::Rgb;

use super::super::state::{ApiBootstrapState, PatchFilterState};
pub(crate) use crate::public_assets::{normalize_public_base_url, resolve_public_asset_url};

#[cfg(target_arch = "wasm32")]
const PROD_API_BASE_URL: &str = "https://api.fishystuff.fish";

pub(super) fn pick_map_version(meta: &MetaResponse) -> Option<String> {
    if let Some(default) = meta.defaults.map_version_id.as_ref() {
        if meta
            .map_versions
            .iter()
            .any(|version| version.map_version_id == *default)
        {
            return Some(default.0.clone());
        }
    }
    if let Some(found) = meta.map_versions.iter().find(|version| version.is_default) {
        return Some(found.map_version_id.0.clone());
    }
    meta.map_versions
        .first()
        .map(|version| version.map_version_id.0.clone())
}

pub(super) fn default_from_ts(meta: &MetaResponse) -> i64 {
    meta.patches
        .first()
        .map(|patch| patch.start_ts_utc)
        .unwrap_or_else(|| now_utc_seconds() - 180 * 86400)
}

pub(super) fn default_from_patch_id(meta: &MetaResponse) -> Option<String> {
    meta.patches.first().map(|patch| patch.patch_id.0.clone())
}

pub(super) fn now_utc_seconds() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        return (js_sys::Date::now() / 1000.0).floor() as i64;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};

        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_secs() as i64)
            .unwrap_or_default()
    }
}

pub(super) fn build_zone_stats_request(
    bootstrap: &ApiBootstrapState,
    patch_filter: &PatchFilterState,
    rgb: Rgb,
) -> Option<ZoneStatsRequest> {
    let defaults = bootstrap.defaults.as_ref()?;
    let map_version = bootstrap.map_version.as_ref()?;
    let from_ts = patch_filter.from_ts?;
    let to_ts = patch_filter.to_ts?;

    Some(ZoneStatsRequest {
        layer_revision_id: Some(map_version.clone()),
        layer_id: None,
        patch_id: None,
        at_ts_utc: None,
        map_version_id: Some(MapVersionId(map_version.clone())),
        rgb: rgb.key(),
        from_ts_utc: from_ts,
        to_ts_utc: to_ts,
        tile_px: defaults.tile_px,
        sigma_tiles: defaults.sigma_tiles,
        fish_norm: false,
        alpha0: defaults.alpha0,
        top_k: defaults.top_k,
        half_life_days: defaults.half_life_days,
        drift_boundary_ts_utc: None,
        ref_id: None,
        lang: None,
    })
}

#[cfg(target_arch = "wasm32")]
pub(super) fn resolve_api_request_url(path: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let base = browser_global_base_url("__fishystuffApiBaseUrl")
            .unwrap_or_else(|| PROD_API_BASE_URL.to_string());
        if path.starts_with("http://") || path.starts_with("https://") {
            return path.to_string();
        }
        return format!(
            "{}{}{}",
            base.trim_end_matches('/'),
            if path.starts_with('/') { "" } else { "/" },
            path
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        path.to_string()
    }
}

pub(super) fn absolutize_layers_response_assets(
    response: &mut LayersResponse,
    public_base_url: Option<&str>,
) {
    let map_asset_cache_key = map_asset_cache_key();
    for layer in &mut response.layers {
        layer.tileset.manifest_url =
            resolve_public_asset_url(Some(&layer.tileset.manifest_url), public_base_url)
                .unwrap_or_default();
        layer.tileset.tile_url_template =
            resolve_public_asset_url(Some(&layer.tileset.tile_url_template), public_base_url)
                .unwrap_or_default();
        if let Some(field_source) = layer.field_source.as_mut() {
            field_source.url = resolve_public_asset_url(Some(&field_source.url), public_base_url)
                .unwrap_or_default();
            let cache_key = normalize_vector_cache_key(
                Some(field_source.revision.as_str()),
                map_asset_cache_key.as_deref(),
            );
            field_source.url =
                append_cache_key_to_public_asset_url(&field_source.url, Some(cache_key.as_str()));
        }
        if let Some(vector_source) = layer.vector_source.as_mut() {
            vector_source.url = resolve_public_asset_url(Some(&vector_source.url), public_base_url)
                .unwrap_or_default();
            let cache_key = normalize_vector_cache_key(
                Some(vector_source.revision.as_str()),
                map_asset_cache_key.as_deref(),
            );
            vector_source.url =
                append_cache_key_to_public_asset_url(&vector_source.url, Some(cache_key.as_str()));
        }
    }
}

fn append_cache_key_to_public_asset_url(url: &str, cache_key: Option<&str>) -> String {
    let cache_key = cache_key.map(str::trim).unwrap_or("");
    if cache_key.is_empty() {
        return url.to_string();
    }
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}v={cache_key}")
}

fn normalize_vector_cache_key(revision: Option<&str>, map_asset_cache_key: Option<&str>) -> String {
    let map_asset_cache_key = map_asset_cache_key.map(str::trim).unwrap_or("");
    if !map_asset_cache_key.is_empty() {
        return map_asset_cache_key.to_string();
    }
    let revision = revision.map(str::trim).unwrap_or("");
    if !revision.is_empty() {
        return revision.to_string();
    }
    String::new()
}

#[cfg(target_arch = "wasm32")]
fn browser_global_base_url(name: &str) -> Option<String> {
    use wasm_bindgen::JsValue;

    let window = web_sys::window()?;
    let value = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str(name)).ok()?;
    let value = value.as_string()?;
    normalize_public_base_url(Some(value.as_str()))
}

#[cfg(target_arch = "wasm32")]
fn map_asset_cache_key() -> Option<String> {
    use wasm_bindgen::JsValue;

    let window = web_sys::window()?;
    let runtime_config = js_sys::Reflect::get(
        window.as_ref(),
        &JsValue::from_str("__fishystuffRuntimeConfig"),
    )
    .ok()?;
    if runtime_config.is_null() || runtime_config.is_undefined() {
        return None;
    }
    let value = js_sys::Reflect::get(&runtime_config, &JsValue::from_str("mapAssetCacheKey"))
        .ok()?
        .as_string()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn map_asset_cache_key() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use fishystuff_api::models::layers::{
        LayerDescriptor, LayersResponse, TilesetRef, VectorSourceRef,
    };

    use super::{
        absolutize_layers_response_assets, append_cache_key_to_public_asset_url,
        default_from_patch_id, default_from_ts, normalize_vector_cache_key,
        resolve_public_asset_url,
    };
    use fishystuff_api::ids::PatchId;
    use fishystuff_api::models::layers::{GeometrySpace, StyleMode};
    use fishystuff_api::models::meta::{MetaResponse, PatchInfo};

    fn patch(id: &str, start_ts_utc: i64) -> PatchInfo {
        PatchInfo {
            patch_id: PatchId(id.to_string()),
            start_ts_utc,
            patch_name: None,
        }
    }

    #[test]
    fn default_range_starts_at_oldest_patch_even_when_default_patch_is_newer() {
        let meta = MetaResponse {
            patches: vec![
                patch("2025-01-01", 10),
                patch("2025-02-01", 20),
                patch("2025-03-01", 30),
            ],
            default_patch: Some(patch("2025-03-01", 30)),
            ..MetaResponse::default()
        };

        assert_eq!(default_from_ts(&meta), 10);
        assert_eq!(default_from_patch_id(&meta).as_deref(), Some("2025-01-01"));
    }

    #[test]
    fn resolve_public_asset_url_normalizes_legacy_static_paths() {
        assert_eq!(
            resolve_public_asset_url(Some("/terrain/v1/manifest.json"), None).as_deref(),
            Some("/images/terrain/v1/manifest.json")
        );
        assert_eq!(
            resolve_public_asset_url(
                Some("/terrain_drape/minimap/v1/manifest.json"),
                Some("https://cdn.example.com"),
            )
            .as_deref(),
            Some("https://cdn.example.com/images/terrain_drape/minimap/v1/manifest.json")
        );
    }

    #[test]
    fn absolutize_layers_response_assets_uses_single_public_base_url() {
        let mut response = LayersResponse {
            revision: "test".to_string(),
            map_version_id: None,
            layers: vec![LayerDescriptor {
                tileset: TilesetRef {
                    manifest_url: "/images/tiles/minimap/v1/tileset.json".to_string(),
                    tile_url_template: "/tiles/mask/v1/{level}/{x}_{y}.png".to_string(),
                    version: "v1".to_string(),
                },
                vector_source: Some(VectorSourceRef {
                    url: "/region_groups/v1.geojson".to_string(),
                    revision: "rg-v1".to_string(),
                    geometry_space: GeometrySpace::MapPixels,
                    style_mode: StyleMode::FeaturePropertyPalette,
                    feature_id_property: Some("id".to_string()),
                    color_property: Some("c".to_string()),
                }),
                ..LayerDescriptor::default()
            }],
        };

        absolutize_layers_response_assets(&mut response, Some("https://cdn.example.com"));

        let layer = &response.layers[0];
        assert_eq!(
            layer.tileset.manifest_url,
            "https://cdn.example.com/images/tiles/minimap/v1/tileset.json"
        );
        assert_eq!(
            layer.tileset.tile_url_template,
            "https://cdn.example.com/images/tiles/mask/v1/{level}/{x}_{y}.png"
        );
        assert_eq!(
            layer
                .vector_source
                .as_ref()
                .map(|source| source.url.as_str()),
            Some("https://cdn.example.com/region_groups/v1.geojson?v=rg-v1")
        );
    }

    #[test]
    fn append_cache_key_adds_query_parameter() {
        assert_eq!(
            append_cache_key_to_public_asset_url("/region_groups/v1.geojson", Some("deploy-123")),
            "/region_groups/v1.geojson?v=deploy-123"
        );
        assert_eq!(
            append_cache_key_to_public_asset_url(
                "https://cdn.example.com/region_groups/v1.geojson?lang=en",
                Some("deploy-123"),
            ),
            "https://cdn.example.com/region_groups/v1.geojson?lang=en&v=deploy-123"
        );
    }

    #[test]
    fn normalize_vector_cache_key_prefers_vector_revision() {
        assert_eq!(
            normalize_vector_cache_key(Some("rg-v1"), Some("deploy-123")),
            "deploy-123"
        );
        assert_eq!(
            normalize_vector_cache_key(Some(""), Some("deploy-123")),
            "deploy-123"
        );
        assert_eq!(normalize_vector_cache_key(Some("rg-v1"), None), "rg-v1");
    }
}
