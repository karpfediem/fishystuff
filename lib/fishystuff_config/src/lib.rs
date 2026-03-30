use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub paths: Paths,
    pub watermap: WatermapConfig,
    pub zone_masks: HashMap<String, String>,
    pub dolt_sql: DoltSqlConfig,
    pub defaults: Defaults,
    pub thresholds: Thresholds,
    pub server: ServerConfig,
    pub server_cache: ServerCache,
}

#[derive(Debug, Clone, Default)]
pub struct Paths {
    pub db: Option<String>,
    pub watermap: Option<String>,
    pub fish_names: Option<String>,
    pub data_dir: Option<String>,
    pub dolt_repo: Option<String>,
    pub patches_csv: Option<String>,
    pub images_dir: Option<String>,
    pub terrain_manifest_url: Option<String>,
    pub terrain_drape_manifest_url: Option<String>,
    pub terrain_height_tiles_url: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WatermapConfig {
    pub path: Option<String>,
    pub transform: WatermapTransform,
}

#[derive(Debug, Clone, Default)]
pub struct WatermapTransform {
    pub kind: Option<String>,
    pub sx: Option<f64>,
    pub sy: Option<f64>,
    pub ox: Option<f64>,
    pub oy: Option<f64>,
    pub world_left: Option<f64>,
    pub world_right: Option<f64>,
    pub world_bottom: Option<f64>,
    pub world_top: Option<f64>,
    pub map_pixel_center_offset: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct Defaults {
    pub tile_px: Option<u32>,
    pub sigma_tiles: Option<f64>,
    pub half_life_days: Option<f64>,
    pub alpha0: Option<f64>,
    pub top_k: Option<usize>,
    pub map_version: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Thresholds {
    pub stale_days: Option<f64>,
    pub ess: Option<f64>,
    pub drift_jsd: Option<f64>,
    pub drift_prob: Option<f64>,
    pub drift_samples: Option<usize>,
    pub drift_min_ess: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct ServerCache {
    pub zone_stats_max_entries: Option<usize>,
    pub effort_grid_max_entries: Option<usize>,
    pub log: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct ServerConfig {
    pub cors_allowed_origins: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DoltSqlConfig {
    pub url: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
}

pub fn load_config(path: impl AsRef<Path>) -> Result<Config> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("read config: {}", path.display()))?;
    parse_config(&content)
}

fn parse_config(content: &str) -> Result<Config> {
    let mut config = Config::default();
    let mut section = String::new();
    for (idx, line) in content.lines().enumerate() {
        let mut line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(pos) = line.find('#') {
            line = line[..pos].trim();
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_lowercase();
            continue;
        }
        let (key, value) =
            split_kv(line).with_context(|| format!("parse config line {}: {}", idx + 1, line))?;
        let value = strip_quotes(value);
        match section.as_str() {
            "paths" => assign_path(&mut config.paths, key, value),
            "watermap" => assign_watermap(&mut config.watermap, key, value),
            "watermap.transform" => {
                assign_watermap_transform(&mut config.watermap.transform, key, value)?
            }
            "dolt_sql" => assign_dolt_sql(&mut config.dolt_sql, key, value)?,
            "zone_masks" => {
                config.zone_masks.insert(key.to_string(), value.to_string());
            }
            "defaults" => assign_default(&mut config.defaults, key, value)?,
            "thresholds" => assign_threshold(&mut config.thresholds, key, value)?,
            "server" => assign_server(&mut config.server, key, value),
            "server.cache" => assign_cache(&mut config.server_cache, key, value)?,
            _ => {}
        }
    }
    Ok(config)
}

fn split_kv(line: &str) -> Result<(&str, &str)> {
    let mut parts = line.splitn(2, '=');
    let key = parts
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing key"))?;
    let value = parts
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing value"))?;
    Ok((key, value))
}

fn strip_quotes(value: &str) -> &str {
    let v = value.trim();
    if (v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')) {
        &v[1..v.len() - 1]
    } else {
        v
    }
}

fn assign_path(paths: &mut Paths, key: &str, value: &str) {
    match key {
        "db" => paths.db = Some(value.to_string()),
        "watermap" => paths.watermap = Some(value.to_string()),
        "fish_names" => paths.fish_names = Some(value.to_string()),
        "data_dir" => paths.data_dir = Some(value.to_string()),
        "dolt_repo" => paths.dolt_repo = Some(value.to_string()),
        "patches_csv" => paths.patches_csv = Some(value.to_string()),
        "images_dir" => paths.images_dir = Some(value.to_string()),
        "terrain_manifest_url" => paths.terrain_manifest_url = Some(value.to_string()),
        "terrain_drape_manifest_url" => paths.terrain_drape_manifest_url = Some(value.to_string()),
        "terrain_height_tiles_url" => paths.terrain_height_tiles_url = Some(value.to_string()),
        _ => {}
    }
}

fn assign_watermap(watermap: &mut WatermapConfig, key: &str, value: &str) {
    if key == "path" {
        watermap.path = Some(value.to_string());
    }
}

fn assign_watermap_transform(
    transform: &mut WatermapTransform,
    key: &str,
    value: &str,
) -> Result<()> {
    match key {
        "kind" => transform.kind = Some(value.to_string()),
        "sx" => transform.sx = Some(parse_f64(value, key)?),
        "sy" => transform.sy = Some(parse_f64(value, key)?),
        "ox" => transform.ox = Some(parse_f64(value, key)?),
        "oy" => transform.oy = Some(parse_f64(value, key)?),
        "world_left" => transform.world_left = Some(parse_f64(value, key)?),
        "world_right" => transform.world_right = Some(parse_f64(value, key)?),
        "world_bottom" => transform.world_bottom = Some(parse_f64(value, key)?),
        "world_top" => transform.world_top = Some(parse_f64(value, key)?),
        "map_pixel_center_offset" => {
            transform.map_pixel_center_offset = Some(parse_f64(value, key)?)
        }
        _ => {}
    }
    Ok(())
}

fn assign_default(defaults: &mut Defaults, key: &str, value: &str) -> Result<()> {
    match key {
        "tile_px" => defaults.tile_px = Some(parse_u32(value, key)?),
        "sigma_tiles" => defaults.sigma_tiles = Some(parse_f64(value, key)?),
        "half_life_days" => defaults.half_life_days = Some(parse_f64(value, key)?),
        "alpha0" => defaults.alpha0 = Some(parse_f64(value, key)?),
        "top_k" => defaults.top_k = Some(parse_usize(value, key)?),
        "map_version" => defaults.map_version = Some(value.to_string()),
        _ => {}
    }
    Ok(())
}

fn assign_threshold(thresholds: &mut Thresholds, key: &str, value: &str) -> Result<()> {
    match key {
        "stale_days" => thresholds.stale_days = Some(parse_f64(value, key)?),
        "ess" => thresholds.ess = Some(parse_f64(value, key)?),
        "drift_jsd" => thresholds.drift_jsd = Some(parse_f64(value, key)?),
        "drift_prob" => thresholds.drift_prob = Some(parse_f64(value, key)?),
        "drift_samples" => thresholds.drift_samples = Some(parse_usize(value, key)?),
        "drift_min_ess" => thresholds.drift_min_ess = Some(parse_f64(value, key)?),
        _ => {}
    }
    Ok(())
}

fn assign_cache(cache: &mut ServerCache, key: &str, value: &str) -> Result<()> {
    match key {
        "zone_stats_max_entries" => cache.zone_stats_max_entries = Some(parse_usize(value, key)?),
        "effort_grid_max_entries" => cache.effort_grid_max_entries = Some(parse_usize(value, key)?),
        "log" => cache.log = Some(parse_bool(value, key)?),
        _ => {}
    }
    Ok(())
}

fn assign_server(server: &mut ServerConfig, key: &str, value: &str) {
    if key == "cors_allowed_origins" {
        server.cors_allowed_origins = Some(value.to_string());
    }
}

fn assign_dolt_sql(dolt_sql: &mut DoltSqlConfig, key: &str, value: &str) -> Result<()> {
    match key {
        "url" => dolt_sql.url = Some(value.to_string()),
        "host" => dolt_sql.host = Some(value.to_string()),
        "port" => dolt_sql.port = Some(parse_u16(value, key)?),
        "user" => dolt_sql.user = Some(value.to_string()),
        "password" => dolt_sql.password = Some(value.to_string()),
        "database" => dolt_sql.database = Some(value.to_string()),
        _ => {}
    }
    Ok(())
}

fn parse_bool(value: &str, key: &str) -> Result<bool> {
    match value.trim().to_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => Err(anyhow::anyhow!("parse {} as bool", key)),
    }
}

fn parse_f64(value: &str, key: &str) -> Result<f64> {
    value
        .parse::<f64>()
        .with_context(|| format!("parse {} as f64", key))
}

fn parse_u32(value: &str, key: &str) -> Result<u32> {
    value
        .parse::<u32>()
        .with_context(|| format!("parse {} as u32", key))
}

fn parse_u16(value: &str, key: &str) -> Result<u16> {
    value
        .parse::<u16>()
        .with_context(|| format!("parse {} as u16", key))
}

fn parse_usize(value: &str, key: &str) -> Result<usize> {
    value
        .parse::<usize>()
        .with_context(|| format!("parse {} as usize", key))
}
