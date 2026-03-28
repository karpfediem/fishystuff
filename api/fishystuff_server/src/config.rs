use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};

use fishystuff_api::ids::MapVersionId;
use fishystuff_api::models::meta::MetaDefaults;
use fishystuff_config::{load_config, Config as FsConfig, DoltSqlConfig};
use fishystuff_core::asset_urls::normalize_public_asset_reference;

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
pub struct AppConfig {
    pub bind: String,
    pub database_url: String,
    pub cors_allowed_origins: Vec<String>,
    pub runtime_cdn_base_url: String,
    pub terrain_manifest_url: Option<String>,
    pub terrain_drape_manifest_url: Option<String>,
    pub terrain_height_tiles_url: Option<String>,
    pub defaults: MetaDefaults,
    pub status_cfg: ZoneStatusConfig,
    pub cache_zone_stats_max: usize,
    pub cache_effort_max: usize,
    pub cache_log: bool,
    pub request_timeout_secs: u64,
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
        let mut config_dir = None;
        if let Some(path) = &config_path {
            fs_config = load_config(path)?;
            config_dir = path.parent().map(PathBuf::from);
        }

        let resolve = |value: &Option<String>| -> Option<PathBuf> {
            value.as_ref().map(|raw| match &config_dir {
                Some(dir) => dir.join(raw),
                None => PathBuf::from(raw),
            })
        };

        let mut bind = "127.0.0.1:8080".to_string();
        let mut database_url = std::env::var("FISHYSTUFF_DATABASE_URL")
            .ok()
            .or_else(|| dolt_sql_to_database_url(&fs_config.dolt_sql));
        let mut cors_allowed_origins = parse_cors_allowed_origins(
            std::env::var("FISHYSTUFF_CORS_ALLOWED_ORIGINS")
                .ok()
                .as_deref()
                .or(fs_config.server.cors_allowed_origins.as_deref()),
        )?;
        let runtime_cdn_base_url = std::env::var("FISHYSTUFF_RUNTIME_CDN_BASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "https://cdn.fishystuff.fish".to_string())
            .trim()
            .trim_end_matches('/')
            .to_string();
        let mut images_dir = resolve(&fs_config.paths.images_dir).unwrap_or_else(|| {
            resolve_default_runtime_dir(
                config_dir.as_deref(),
                &[
                    "../data/cdn/public/images",
                    "data/cdn/public/images",
                    "images",
                ],
            )
        });
        let mut terrain_manifest_url =
            normalize_optional_url(fs_config.paths.terrain_manifest_url.as_deref());
        let mut terrain_drape_manifest_url =
            normalize_optional_url(fs_config.paths.terrain_drape_manifest_url.as_deref());
        let mut terrain_height_tiles_url =
            normalize_optional_url(fs_config.paths.terrain_height_tiles_url.as_deref());

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
                "--database-url" => {
                    database_url = Some(
                        args.get(i + 1)
                            .ok_or_else(|| anyhow!("--database-url requires value"))?
                            .clone(),
                    );
                    i += 2;
                }
                "--cors-allowed-origins" => {
                    let value = args
                        .get(i + 1)
                        .ok_or_else(|| anyhow!("--cors-allowed-origins requires value"))?;
                    cors_allowed_origins = parse_cors_allowed_origins(Some(value))?;
                    i += 2;
                }
                "--images-dir" => {
                    images_dir = PathBuf::from(
                        args.get(i + 1)
                            .ok_or_else(|| anyhow!("--images-dir requires value"))?,
                    );
                    i += 2;
                }
                "--terrain-manifest-url" => {
                    terrain_manifest_url = normalize_optional_url(Some(
                        args.get(i + 1)
                            .ok_or_else(|| anyhow!("--terrain-manifest-url requires value"))?
                            .as_str(),
                    ));
                    i += 2;
                }
                "--terrain-drape-manifest-url" => {
                    terrain_drape_manifest_url = normalize_optional_url(Some(
                        args.get(i + 1)
                            .ok_or_else(|| anyhow!("--terrain-drape-manifest-url requires value"))?
                            .as_str(),
                    ));
                    i += 2;
                }
                "--terrain-height-tiles-url" => {
                    terrain_height_tiles_url = normalize_optional_url(Some(
                        args.get(i + 1)
                            .ok_or_else(|| anyhow!("--terrain-height-tiles-url requires value"))?
                            .as_str(),
                    ));
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

        let database_url = database_url.ok_or_else(|| {
            anyhow!("database URL is required; pass --database-url or set FISHYSTUFF_DATABASE_URL")
        })?;
        if terrain_manifest_url.is_none() {
            terrain_manifest_url = detect_local_manifest_url(
                &images_dir,
                "terrain/v1/manifest.json",
                "/images/terrain/v1/manifest.json",
            );
        }
        if terrain_drape_manifest_url.is_none() {
            terrain_drape_manifest_url = detect_local_manifest_url(
                &images_dir,
                "terrain_drape/minimap/v1/manifest.json",
                "/images/terrain_drape/minimap/v1/manifest.json",
            );
        }
        if terrain_height_tiles_url.is_none() {
            terrain_height_tiles_url = detect_local_directory_url(
                &images_dir,
                "terrain_height/v1",
                "/images/terrain_height/v1",
            );
        }
        if terrain_height_tiles_url.is_none() {
            terrain_height_tiles_url = detect_local_directory_url(
                &images_dir,
                "terrain_fullres/v1",
                "/images/terrain_fullres/v1",
            );
        }

        Ok(Self {
            bind,
            database_url,
            cors_allowed_origins,
            runtime_cdn_base_url,
            terrain_manifest_url,
            terrain_drape_manifest_url,
            terrain_height_tiles_url,
            defaults,
            status_cfg,
            cache_zone_stats_max,
            cache_effort_max,
            cache_log,
            request_timeout_secs,
        })
    }
}

fn resolve_default_runtime_dir(config_dir: Option<&Path>, candidates: &[&str]) -> PathBuf {
    let bases = runtime_search_bases(config_dir);

    for base in &bases {
        for candidate in candidates {
            let path = base.join(candidate);
            if path.exists() {
                return path;
            }
        }
    }

    if let Some(dir) = config_dir {
        return dir.join(candidates[0]);
    }
    PathBuf::from(candidates[0])
}

fn runtime_search_bases(config_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut bases = Vec::new();
    if let Some(dir) = config_dir {
        bases.push(dir.to_path_buf());
    }
    if let Ok(cwd) = std::env::current_dir() {
        if !bases.iter().any(|existing| existing == &cwd) {
            bases.push(cwd);
        }
    }
    bases
}

fn normalize_optional_url(value: Option<&str>) -> Option<String> {
    let raw = value?.trim();
    if raw.is_empty() {
        return None;
    }
    Some(normalize_public_asset_reference(raw))
}

fn parse_cors_allowed_origins(value: Option<&str>) -> Result<Vec<String>> {
    let mut origins = Vec::new();
    for raw in value
        .unwrap_or("https://fishystuff.fish,https://www.fishystuff.fish")
        .split(',')
    {
        let Some(origin) = normalize_origin(raw) else {
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

fn normalize_origin(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let (scheme, rest) = trimmed.split_once("://")?;
    if scheme != "http" && scheme != "https" {
        return None;
    }
    if rest.is_empty() || rest.contains('/') || rest.contains('?') || rest.contains('#') {
        return None;
    }
    Some(format!("{scheme}://{rest}"))
}

fn detect_local_manifest_url(
    images_dir: &Path,
    relative_manifest_path: &str,
    public_manifest_url: &str,
) -> Option<String> {
    let path = images_dir.join(relative_manifest_path);
    if path.is_file() {
        Some(public_manifest_url.to_string())
    } else {
        None
    }
}

fn detect_local_directory_url(
    root_dir: &Path,
    relative_dir_path: &str,
    public_url: &str,
) -> Option<String> {
    let path = root_dir.join(relative_dir_path);
    if path.is_dir() {
        Some(public_url.to_string())
    } else {
        None
    }
}

fn dolt_sql_to_database_url(cfg: &DoltSqlConfig) -> Option<String> {
    if let Some(url) = &cfg.url {
        return Some(url.clone());
    }

    let host = cfg.host.clone().unwrap_or_else(|| "127.0.0.1".to_string());
    let port = cfg.port.unwrap_or(3306);
    let user = cfg.user.clone().unwrap_or_else(|| "root".to_string());
    let database = cfg.database.clone()?;

    let base = if let Some(password) = cfg.password.clone() {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            user, password, host, port, database
        )
    } else {
        format!("mysql://{}@{}:{}/{}", user, host, port, database)
    };

    Some(base)
}

#[cfg(test)]
mod tests {
    use super::{parse_cors_allowed_origins, resolve_default_runtime_dir};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn default_runtime_dir_prefers_existing_candidate_under_config_dir() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("fishystuff-config-path-resolution-{stamp}"));
        let config_dir = root.join("api");
        let nested = root.join("data/cdn/public/images");
        fs::create_dir_all(&nested).expect("create nested test dir");
        fs::create_dir_all(&config_dir).expect("create config dir");

        let resolved = resolve_default_runtime_dir(
            Some(&config_dir),
            &[
                "../data/cdn/public/images",
                "data/cdn/public/images",
                "images",
            ],
        );
        assert_eq!(
            resolved.canonicalize().expect("canonical resolved path"),
            nested.canonicalize().expect("canonical nested path")
        );

        fs::remove_dir_all(&root).expect("cleanup temp dir");
    }

    #[test]
    fn default_runtime_dir_falls_back_to_first_candidate_when_nothing_exists() {
        let resolved = resolve_default_runtime_dir(
            None,
            &[
                "../data/cdn/public/images",
                "data/cdn/public/images",
                "images",
            ],
        );
        assert_eq!(resolved, PathBuf::from("../data/cdn/public/images"));
    }

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
}
