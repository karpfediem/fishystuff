pub const DEFAULT_PUBLIC_SITE_BASE_URL: &str = "https://fishystuff.fish";
pub const DEFAULT_PUBLIC_API_BASE_URL: &str = "https://api.fishystuff.fish";
pub const DEFAULT_PUBLIC_CDN_BASE_URL: &str = "https://cdn.fishystuff.fish";
pub const DEFAULT_PUBLIC_OTEL_BASE_URL: &str = "https://otel.fishystuff.fish";

pub fn normalize_public_base_url(value: Option<&str>) -> Option<String> {
    let raw = value?.trim().trim_end_matches('/');
    if raw.is_empty() {
        return None;
    }
    let (scheme, rest) = raw.split_once("://")?;
    if scheme != "http" && scheme != "https" {
        return None;
    }
    if rest.is_empty() || rest.contains('/') || rest.contains('?') || rest.contains('#') {
        return None;
    }
    Some(format!("{scheme}://{rest}"))
}

pub fn derive_sibling_public_base_url(base_url: Option<&str>, subdomain: &str) -> Option<String> {
    let normalized_subdomain = subdomain.trim().trim_matches('.');
    if normalized_subdomain.is_empty() {
        return None;
    }
    let normalized_base_url = normalize_public_base_url(base_url)?;
    let (scheme, host) = normalized_base_url.split_once("://")?;
    let hostname = host.split(':').next().unwrap_or(host);
    if hostname == "localhost" || hostname == "127.0.0.1" {
        return None;
    }
    Some(format!("{scheme}://{normalized_subdomain}.{host}"))
}

#[cfg(test)]
mod tests {
    use super::{
        derive_sibling_public_base_url, normalize_public_base_url, DEFAULT_PUBLIC_API_BASE_URL,
        DEFAULT_PUBLIC_CDN_BASE_URL, DEFAULT_PUBLIC_OTEL_BASE_URL, DEFAULT_PUBLIC_SITE_BASE_URL,
    };

    #[test]
    fn default_constants_match_the_primary_public_hosts() {
        assert_eq!(DEFAULT_PUBLIC_SITE_BASE_URL, "https://fishystuff.fish");
        assert_eq!(DEFAULT_PUBLIC_API_BASE_URL, "https://api.fishystuff.fish");
        assert_eq!(DEFAULT_PUBLIC_CDN_BASE_URL, "https://cdn.fishystuff.fish");
        assert_eq!(DEFAULT_PUBLIC_OTEL_BASE_URL, "https://otel.fishystuff.fish");
    }

    #[test]
    fn normalize_public_base_url_rejects_non_origin_values() {
        assert_eq!(
            normalize_public_base_url(Some(" https://beta.fishystuff.fish/ ")).as_deref(),
            Some("https://beta.fishystuff.fish")
        );
        assert_eq!(
            normalize_public_base_url(Some("https://beta.fishystuff.fish:8443/")).as_deref(),
            Some("https://beta.fishystuff.fish:8443")
        );
        assert_eq!(
            normalize_public_base_url(Some("ftp://beta.fishystuff.fish")),
            None
        );
        assert_eq!(
            normalize_public_base_url(Some("https://beta.fishystuff.fish/path")),
            None
        );
    }

    #[test]
    fn derive_sibling_public_base_url_supports_beta_hosts() {
        assert_eq!(
            derive_sibling_public_base_url(Some("https://beta.fishystuff.fish"), "api").as_deref(),
            Some("https://api.beta.fishystuff.fish")
        );
        assert_eq!(
            derive_sibling_public_base_url(Some("https://beta.fishystuff.fish"), "cdn").as_deref(),
            Some("https://cdn.beta.fishystuff.fish")
        );
        assert_eq!(
            derive_sibling_public_base_url(Some("https://beta.fishystuff.fish"), "otel").as_deref(),
            Some("https://otel.beta.fishystuff.fish")
        );
        assert_eq!(
            derive_sibling_public_base_url(Some("http://localhost:1990"), "api"),
            None
        );
    }
}
