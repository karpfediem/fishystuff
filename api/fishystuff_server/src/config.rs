use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};

use fishystuff_api::ids::MapVersionId;
use fishystuff_api::models::meta::MetaDefaults;
use fishystuff_config::{load_api_database_url_from_secretspec, load_config, Config as FsConfig};
use fishystuff_core::public_endpoints::{
    derive_sibling_public_base_url, normalize_public_base_url, DEFAULT_PUBLIC_CDN_BASE_URL,
    DEFAULT_PUBLIC_SITE_BASE_URL,
};

#[derive(Debug, Clone)]
pub struct ZoneStatusConfig {
    pub stale_days_threshold: f64,
    pub ess_threshold: f64,
    pub drift_jsd_threshold: f64,
    pub drift_prob_threshold: f64,
    pub drift_samples: usize,
    pub drift_min_ess: f64,
}

impl Default for ZoneStatusConfig {
    fn default() -> Self {
        Self {
            stale_days_threshold: 30.0,
            ess_threshold: 10.0,
            drift_jsd_threshold: 0.1,
            drift_prob_threshold: 0.95,
            drift_samples: 300,
            drift_min_ess: 10.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub service_name: String,
    pub otlp_traces_endpoint: String,
    pub sample_ratio: f64,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            service_name: "fishystuff-api".to_string(),
            otlp_traces_endpoint: String::new(),
            sample_ratio: 0.25,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind: String,
    pub database_url: String,
    pub cors_allowed_origins: Vec<String>,
    pub runtime_cdn_base_url: String,
    pub defaults: MetaDefaults,
    pub status_cfg: ZoneStatusConfig,
    pub cache_zone_stats_max: usize,
    pub cache_effort_max: usize,
    pub cache_log: bool,
    pub request_timeout_secs: u64,
    pub telemetry: TelemetryConfig,
}

impl AppConfig {
    pub fn parse() -> Result<Self> {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut config_path: Option<PathBuf> = None;
        let mut idx = 0usize;
        while idx < args.len() {
            if args[idx] == "--config" {
                if idx + 1 >= args.len() {
                    bail!("--config requires value");
                }
                config_path = Some(PathBuf::from(&args[idx + 1]));
                idx += 2;
            } else {
                idx += 1;
            }
        }

        let mut fs_config = FsConfig::default();
        if let Some(path) = &config_path {
            fs_config = load_config(path)?;
        }

        let mut bind = "127.0.0.1:8080".to_string();
        let mut cors_allowed_origins = parse_cors_allowed_origins(
            std::env::var("FISHYSTUFF_CORS_ALLOWED_ORIGINS")
                .ok()
                .as_deref()
                .or(fs_config.server.cors_allowed_origins.as_deref()),
        )?;
        let runtime_cdn_base_url = std::env::var("FISHYSTUFF_RUNTIME_CDN_BASE_URL")
            .ok()
            .as_deref()
            .and_then(|value| normalize_public_base_url(Some(value)))
            .or_else(default_public_cdn_base_url)
            .unwrap_or_else(|| DEFAULT_PUBLIC_CDN_BASE_URL.to_string());

        let mut defaults = MetaDefaults {
            tile_px: fs_config.defaults.tile_px.unwrap_or(32),
            sigma_tiles: fs_config.defaults.sigma_tiles.unwrap_or(3.0),
            half_life_days: fs_config.defaults.half_life_days,
            alpha0: fs_config.defaults.alpha0.unwrap_or(1.0),
            top_k: fs_config.defaults.top_k.unwrap_or(30),
            map_version_id: fs_config.defaults.map_version.clone().map(MapVersionId),
        };

        let mut status_cfg = ZoneStatusConfig::default();
        if let Some(v) = fs_config.thresholds.stale_days {
            status_cfg.stale_days_threshold = v;
        }
        if let Some(v) = fs_config.thresholds.ess {
            status_cfg.ess_threshold = v;
        }
        if let Some(v) = fs_config.thresholds.drift_jsd {
            status_cfg.drift_jsd_threshold = v;
        }
        if let Some(v) = fs_config.thresholds.drift_prob {
            status_cfg.drift_prob_threshold = v;
        }
        if let Some(v) = fs_config.thresholds.drift_samples {
            status_cfg.drift_samples = v;
        }
        if let Some(v) = fs_config.thresholds.drift_min_ess {
            status_cfg.drift_min_ess = v;
        }

        let mut cache_zone_stats_max = fs_config.server_cache.zone_stats_max_entries.unwrap_or(256);
        let mut cache_effort_max = fs_config.server_cache.effort_grid_max_entries.unwrap_or(16);
        let mut cache_log = fs_config.server_cache.log.unwrap_or(false);
        let mut request_timeout_secs = 15u64;
        let telemetry = TelemetryConfig {
            enabled: parse_env_flag("FISHYSTUFF_OTEL_ENABLED", false),
            service_name: std::env::var("FISHYSTUFF_OTEL_SERVICE_NAME")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "fishystuff-api".to_string()),
            otlp_traces_endpoint: std::env::var("FISHYSTUFF_OTEL_TRACES_ENDPOINT")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_default(),
            sample_ratio: parse_env_f64("FISHYSTUFF_OTEL_SAMPLE_RATIO", 0.25).clamp(0.0, 1.0),
        };

        let mut i = 0usize;
        while i < args.len() {
            match args[i].as_str() {
                "--config" => {
                    i += 2;
                }
                "--bind" => {
                    bind = args
                        .get(i + 1)
                        .ok_or_else(|| anyhow!("--bind requires value"))?
                        .clone();
                    i += 2;
                }
                "--cors-allowed-origins" => {
                    let value = args
                        .get(i + 1)
                        .ok_or_else(|| anyhow!("--cors-allowed-origins requires value"))?;
                    cors_allowed_origins = parse_cors_allowed_origins(Some(value))?;
                    i += 2;
                }
                "--default-map-version" => {
                    let value = args
                        .get(i + 1)
                        .ok_or_else(|| anyhow!("--default-map-version requires value"))?
                        .clone();
                    defaults.map_version_id = Some(MapVersionId(value));
                    i += 2;
                }
                "--default-tile-px" => {
                    defaults.tile_px = args
                        .get(i + 1)
                        .ok_or_else(|| anyhow!("--default-tile-px requires value"))?
                        .parse()
                        .context("parse --default-tile-px")?;
                    i += 2;
                }
                "--default-sigma-tiles" => {
                    defaults.sigma_tiles = args
                        .get(i + 1)
                        .ok_or_else(|| anyhow!("--default-sigma-tiles requires value"))?
                        .parse()
                        .context("parse --default-sigma-tiles")?;
                    i += 2;
                }
                "--default-half-life-days" => {
                    defaults.half_life_days = Some(
                        args.get(i + 1)
                            .ok_or_else(|| anyhow!("--default-half-life-days requires value"))?
                            .parse()
                            .context("parse --default-half-life-days")?,
                    );
                    i += 2;
                }
                "--default-alpha0" => {
                    defaults.alpha0 = args
                        .get(i + 1)
                        .ok_or_else(|| anyhow!("--default-alpha0 requires value"))?
                        .parse()
                        .context("parse --default-alpha0")?;
                    i += 2;
                }
                "--default-top-k" => {
                    defaults.top_k = args
                        .get(i + 1)
                        .ok_or_else(|| anyhow!("--default-top-k requires value"))?
                        .parse()
                        .context("parse --default-top-k")?;
                    i += 2;
                }
                "--cache-zone-stats-max" => {
                    cache_zone_stats_max = args
                        .get(i + 1)
                        .ok_or_else(|| anyhow!("--cache-zone-stats-max requires value"))?
                        .parse()
                        .context("parse --cache-zone-stats-max")?;
                    i += 2;
                }
                "--cache-effort-max" => {
                    cache_effort_max = args
                        .get(i + 1)
                        .ok_or_else(|| anyhow!("--cache-effort-max requires value"))?
                        .parse()
                        .context("parse --cache-effort-max")?;
                    i += 2;
                }
                "--cache-log" => {
                    cache_log = true;
                    i += 1;
                }
                "--request-timeout-secs" => {
                    request_timeout_secs = args
                        .get(i + 1)
                        .ok_or_else(|| anyhow!("--request-timeout-secs requires value"))?
                        .parse()
                        .context("parse --request-timeout-secs")?;
                    i += 2;
                }
                _ => bail!("unknown arg: {}", args[i]),
            }
        }

        let database_url = load_api_database_url_from_secretspec()
            .context("resolve database URL from SecretSpec `api` profile")?;

        Ok(Self {
            bind,
            database_url,
            cors_allowed_origins,
            runtime_cdn_base_url,
            defaults,
            status_cfg,
            cache_zone_stats_max,
            cache_effort_max,
            cache_log,
            request_timeout_secs,
            telemetry,
        })
    }
}

fn parse_cors_allowed_origins(value: Option<&str>) -> Result<Vec<String>> {
    let mut origins = Vec::new();
    let default_origin =
        default_public_site_origin().unwrap_or_else(|| DEFAULT_PUBLIC_SITE_BASE_URL.to_string());
    for raw in value.unwrap_or(default_origin.as_str()).split(',') {
        let Some(origin) = normalize_public_base_url(Some(raw)) else {
            continue;
        };
        if !origins.iter().any(|existing| existing == &origin) {
            origins.push(origin);
        }
    }
    if origins.is_empty() {
        bail!("at least one CORS allowed origin is required");
    }
    Ok(origins)
}

fn default_public_site_origin() -> Option<String> {
    std::env::var("FISHYSTUFF_PUBLIC_SITE_BASE_URL")
        .ok()
        .as_deref()
        .and_then(|value| normalize_public_base_url(Some(value)))
}

fn default_public_cdn_base_url() -> Option<String> {
    std::env::var("FISHYSTUFF_PUBLIC_CDN_BASE_URL")
        .ok()
        .as_deref()
        .and_then(|value| normalize_public_base_url(Some(value)))
        .or_else(|| {
            std::env::var("FISHYSTUFF_PUBLIC_SITE_BASE_URL")
                .ok()
                .as_deref()
                .and_then(|value| derive_sibling_public_base_url(Some(value), "cdn"))
        })
}

fn parse_env_flag(name: &str, fallback: bool) -> bool {
    let Some(value) = std::env::var(name).ok() else {
        return fallback;
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn parse_env_f64(name: &str, fallback: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<f64>().ok())
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::parse_cors_allowed_origins;
    use fishystuff_core::public_endpoints::derive_sibling_public_base_url;

    #[test]
    fn parse_cors_allowed_origins_normalizes_and_deduplicates() {
        let origins = parse_cors_allowed_origins(Some(
            " https://fishystuff.fish/ , http://127.0.0.1:1990 , https://fishystuff.fish ",
        ))
        .expect("parse origins");
        assert_eq!(
            origins,
            vec![
                "https://fishystuff.fish".to_string(),
                "http://127.0.0.1:1990".to_string()
            ]
        );
    }

    #[test]
    fn parse_cors_allowed_origins_rejects_paths() {
        assert!(parse_cors_allowed_origins(Some("https://fishystuff.fish/map")).is_err());
    }

    #[test]
    fn derive_sibling_origin_supports_beta_sibling_hosts() {
        assert_eq!(
            derive_sibling_public_base_url(Some("https://beta.fishystuff.fish"), "cdn").as_deref(),
            Some("https://cdn.beta.fishystuff.fish")
        );
        assert_eq!(
            derive_sibling_public_base_url(Some("https://beta.fishystuff.fish"), "api").as_deref(),
            Some("https://api.beta.fishystuff.fish")
        );
    }
}
