use fishystuff_api::ids::{MapVersionId, RgbKey};
use fishystuff_api::models::layers::LayersResponse;
use fishystuff_api::models::meta::MetaResponse;
use fishystuff_api::models::zone_stats::ZoneStatsRequest;
use fishystuff_core::asset_urls::normalize_site_asset_path;

use super::super::state::{ApiBootstrapState, PatchFilterState};

const LOCAL_CDN_BASE_URL: &str = "http://127.0.0.1:4040";
const PROD_CDN_BASE_URL: &str = "https://cdn.fishystuff.fish";

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
    (js_sys::Date::now() / 1000.0).floor() as i64
}

pub(super) fn build_zone_stats_request(
    bootstrap: &ApiBootstrapState,
    patch_filter: &PatchFilterState,
    rgb: (u8, u8, u8),
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
        rgb: RgbKey(format!("{},{},{}", rgb.0, rgb.1, rgb.2)),
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

pub(crate) fn resolve_public_asset_url(
    value: Option<&str>,
    public_base_url: Option<&str>,
) -> Option<String> {
    let normalized = normalize_site_asset_path(value?);
    let raw = normalized.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return Some(raw.to_string());
    }
    if raw.starts_with('/') {
        if let Some(base) = public_base_url {
            let base = base.trim_end_matches('/');
            if !base.is_empty() {
                return Some(format!("{base}{raw}"));
            }
        }
        return Some(raw.to_string());
    }
    let base = public_base_url?.trim_end_matches('/');
    if base.is_empty() {
        return None;
    }
    let path = raw.trim_start_matches('/');
    Some(format!("{base}/{path}"))
}

pub(crate) fn normalize_public_base_url(value: Option<&str>) -> Option<String> {
    if let Some(raw) = value.map(str::trim) {
        if !raw.is_empty() {
            return Some(raw.trim_end_matches('/').to_string());
        }
    }
    fallback_public_base_url()
}

pub(super) fn absolutize_layers_response_assets(
    response: &mut LayersResponse,
    public_base_url: Option<&str>,
) {
    for layer in &mut response.layers {
        layer.tileset.manifest_url =
            resolve_public_asset_url(Some(&layer.tileset.manifest_url), public_base_url)
                .unwrap_or_default();
        layer.tileset.tile_url_template =
            resolve_public_asset_url(Some(&layer.tileset.tile_url_template), public_base_url)
                .unwrap_or_default();
        if let Some(vector_source) = layer.vector_source.as_mut() {
            vector_source.url = resolve_public_asset_url(Some(&vector_source.url), public_base_url)
                .unwrap_or_default();
        }
    }
}

fn fallback_public_base_url() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let hostname = web_sys::window()?.location().hostname().ok()?;
        return Some(fallback_public_base_url_for_hostname(&hostname).to_string());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

fn fallback_public_base_url_for_hostname(hostname: &str) -> &'static str {
    let hostname = hostname.trim().to_ascii_lowercase();
    if hostname == "localhost"
        || hostname == "127.0.0.1"
        || hostname == "::1"
        || hostname.ends_with(".localhost")
    {
        LOCAL_CDN_BASE_URL
    } else {
        PROD_CDN_BASE_URL
    }
}

#[cfg(test)]
mod tests {
    use fishystuff_api::models::layers::{
        LayerDescriptor, LayersResponse, TilesetRef, VectorSourceRef,
    };

    use super::{
        absolutize_layers_response_assets, default_from_patch_id, default_from_ts,
        fallback_public_base_url_for_hostname, resolve_public_asset_url,
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
    fn fallback_public_base_url_uses_local_or_production_hosts() {
        assert_eq!(
            fallback_public_base_url_for_hostname("localhost"),
            "http://127.0.0.1:4040"
        );
        assert_eq!(
            fallback_public_base_url_for_hostname("map.localhost"),
            "http://127.0.0.1:4040"
        );
        assert_eq!(
            fallback_public_base_url_for_hostname("fishystuff.fish"),
            "https://cdn.fishystuff.fish"
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
            Some("https://cdn.example.com/region_groups/v1.geojson")
        );
    }
}
