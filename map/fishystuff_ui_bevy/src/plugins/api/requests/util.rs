use fishystuff_api::ids::MapVersionId;
use fishystuff_api::models::meta::MetaResponse;
use fishystuff_api::models::zone_stats::ZoneStatsRequest;
use fishystuff_api::Rgb;
#[cfg(target_arch = "wasm32")]
use fishystuff_core::public_endpoints::{
    derive_sibling_public_base_url, DEFAULT_PUBLIC_API_BASE_URL,
};

use super::super::state::{ApiBootstrapState, PatchFilterState};
#[cfg(target_arch = "wasm32")]
use crate::public_assets::normalize_public_base_url;

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
    let meta = bootstrap.meta.as_ref()?;
    let defaults = bootstrap.defaults.as_ref()?;
    let map_version = bootstrap.map_version.as_ref()?;
    let from_ts = patch_filter
        .from_ts
        .unwrap_or_else(|| default_from_ts(meta));
    let to_ts = patch_filter.to_ts.unwrap_or_else(now_utc_seconds);
    if from_ts >= to_ts {
        return None;
    }

    Some(ZoneStatsRequest {
        layer_revision_id: None,
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
        lang: selected_data_lang(),
    })
}

#[cfg(target_arch = "wasm32")]
pub(super) fn resolve_api_request_url(path: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let base = browser_global_base_url("__fishystuffApiBaseUrl")
            .or_else(browser_location_api_base_url)
            .unwrap_or_else(|| DEFAULT_PUBLIC_API_BASE_URL.to_string());
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

#[cfg(target_arch = "wasm32")]
pub(super) fn resolve_api_request_url_with_data_lang(path: &str) -> String {
    let path = if let Some(lang) = selected_data_lang() {
        append_query_param(path, "lang", &lang)
    } else {
        path.to_string()
    };
    resolve_api_request_url(&path)
}

fn selected_data_lang() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        return browser_selected_data_lang();
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

#[cfg(target_arch = "wasm32")]
fn append_query_param(path: &str, key: &str, value: &str) -> String {
    let separator = if path.contains('?') { '&' } else { '?' };
    format!("{path}{separator}{key}={value}")
}

#[cfg(target_arch = "wasm32")]
fn browser_selected_data_lang() -> Option<String> {
    use wasm_bindgen::{JsCast, JsValue};

    let window = web_sys::window()?;
    let helper =
        js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("__fishystuffLanguage")).ok()?;
    let current = js_sys::Reflect::get(&helper, &JsValue::from_str("current")).ok()?;
    let current = current.dyn_ref::<js_sys::Function>()?;
    let snapshot = current.call0(&helper).ok()?;
    let value = js_sys::Reflect::get(&snapshot, &JsValue::from_str("apiLang"))
        .ok()?
        .as_string()?;
    normalize_data_lang(value.as_str())
}

#[cfg(target_arch = "wasm32")]
fn normalize_data_lang(value: &str) -> Option<String> {
    let code = value.trim();
    if code.is_empty()
        || !code
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
    {
        return None;
    }
    Some(code.to_string())
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
fn browser_location_api_base_url() -> Option<String> {
    let window = web_sys::window()?;
    let origin = window.location().origin().ok()?;
    derive_sibling_public_base_url(Some(origin.as_str()), "api")
}

#[cfg(test)]
mod tests {
    use super::{build_zone_stats_request, default_from_patch_id, default_from_ts};
    use crate::plugins::api::{ApiBootstrapState, PatchFilterState};
    use crate::public_assets::resolve_public_asset_url;
    use fishystuff_api::ids::{MapVersionId, PatchId};
    use fishystuff_api::models::meta::{MetaDefaults, MetaResponse, PatchInfo};
    use fishystuff_api::Rgb;

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
    fn build_zone_stats_request_uses_map_version_id_without_forcing_layer_revision_id() {
        let bootstrap = ApiBootstrapState {
            defaults: Some(MetaDefaults {
                map_version_id: Some(MapVersionId("bdo-v2".to_string())),
                tile_px: 32,
                sigma_tiles: 3.5,
                alpha0: 1.25,
                top_k: 15,
                half_life_days: Some(21.0),
                ..MetaDefaults::default()
            }),
            map_version: Some("bdo-v2".to_string()),
            meta: Some(MetaResponse::default()),
            ..ApiBootstrapState::default()
        };
        let patch_filter = PatchFilterState {
            from_ts: Some(100),
            to_ts: Some(200),
            ..PatchFilterState::default()
        };

        let request = build_zone_stats_request(&bootstrap, &patch_filter, Rgb::new(1, 2, 3))
            .expect("zone stats request");

        assert_eq!(request.layer_revision_id, None);
        assert_eq!(
            request
                .map_version_id
                .as_ref()
                .map(|value| value.0.as_str()),
            Some("bdo-v2")
        );
        assert_eq!(request.from_ts_utc, 100);
        assert_eq!(request.to_ts_utc, 200);
        assert_eq!(request.tile_px, 32);
        assert_eq!(request.sigma_tiles, 3.5);
        assert_eq!(request.alpha0, 1.25);
        assert_eq!(request.top_k, 15);
        assert_eq!(request.half_life_days, Some(21.0));
    }
}
