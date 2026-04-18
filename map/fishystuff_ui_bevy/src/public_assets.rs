use fishystuff_core::asset_urls::normalize_site_asset_path;
use fishystuff_core::public_endpoints::normalize_public_base_url as normalize_core_public_base_url;
#[cfg(target_arch = "wasm32")]
use fishystuff_core::public_endpoints::{
    derive_sibling_public_base_url, DEFAULT_PUBLIC_CDN_BASE_URL,
};

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
    if let Some(normalized) = normalize_core_public_base_url(value) {
        return Some(normalized);
    }
    fallback_public_base_url()
}

#[cfg(target_arch = "wasm32")]
fn fallback_public_base_url() -> Option<String> {
    if let Some(configured) = browser_global_base_url("__fishystuffCdnBaseUrl") {
        return Some(configured);
    }
    if let Some(origin) = browser_location_origin() {
        if let Some(derived) = derive_sibling_public_base_url(Some(origin.as_str()), "cdn") {
            return Some(derived);
        }
    }
    Some(DEFAULT_PUBLIC_CDN_BASE_URL.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn fallback_public_base_url() -> Option<String> {
    None
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
fn browser_location_origin() -> Option<String> {
    let window = web_sys::window()?;
    let origin = window.location().origin().ok()?;
    normalize_core_public_base_url(Some(origin.as_str()))
}

#[cfg(test)]
mod tests {
    use super::resolve_public_asset_url;

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
    fn resolve_public_asset_url_uses_public_base_for_zone_assets() {
        assert_eq!(
            resolve_public_asset_url(
                Some("/images/exact_lookup/zone_mask.v1.bin"),
                Some("https://cdn.example.com"),
            )
            .as_deref(),
            Some("https://cdn.example.com/images/exact_lookup/zone_mask.v1.bin")
        );
        assert_eq!(
            resolve_public_asset_url(
                Some("/images/zones_mask_v1.png"),
                Some("https://cdn.example.com"),
            )
            .as_deref(),
            Some("https://cdn.example.com/images/zones_mask_v1.png")
        );
    }
}
