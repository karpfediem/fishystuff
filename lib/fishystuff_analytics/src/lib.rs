use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use serde::Serialize;

use fishystuff_core::masks::format_rgb_u32;
use fishystuff_core::prob::js_divergence;
use fishystuff_store::sqlite::SqliteStore;
use fishystuff_zones_meta::ZoneMeta;

const EPS: f64 = 1e-9;
const EPS_EFF: f64 = 1e-9;
const EPS_FISH: f64 = 1e-9;

#[derive(Clone, Debug)]
pub struct QueryParams {
    pub map_version: String,
    pub from_ts_utc: i64,
    pub to_ts_utc: i64,
    pub half_life_days: Option<f64>,
    pub tile_px: u32,
    pub sigma_tiles: f64,
    pub fish_norm: bool,
    pub alpha0: f64,
    pub top_k: usize,
    pub drift_boundary_ts: Option<i64>,
}

impl QueryParams {
    pub fn validate(&self) -> Result<()> {
        if self.from_ts_utc >= self.to_ts_utc {
            bail!("from_ts_utc must be < to_ts_utc");
        }
        if self.tile_px == 0 {
            bail!("tile_px must be > 0");
        }
        if self.sigma_tiles <= 0.0 {
            bail!("sigma_tiles must be > 0");
        }
        if let Some(half) = self.half_life_days {
            if half <= 0.0 {
                bail!("half_life_days must be > 0");
            }
        }
        if self.alpha0 <= 0.0 {
            bail!("alpha0 must be > 0");
        }
        if self.top_k == 0 {
            bail!("top_k must be > 0");
        }
        if let Some(boundary) = self.drift_boundary_ts {
            if boundary <= self.from_ts_utc || boundary >= self.to_ts_utc {
                bail!("drift_boundary_ts must be within (from_ts_utc, to_ts_utc)");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ZoneStats {
    pub zone_rgb_u32: u32,
    pub zone_rgb: String,
    pub zone_name: Option<String>,
    pub window: WindowInfo,
    pub confidence: ZoneConfidence,
    pub distribution: Vec<FishEvidence>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowInfo {
    pub from_ts_utc: i64,
    pub to_ts_utc: i64,
    pub half_life_days: Option<f64>,
    pub fish_norm: bool,
    pub tile_px: u32,
    pub sigma_tiles: f64,
    pub alpha0: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ZoneConfidence {
    pub ess: f64,
    pub total_weight: f64,
    pub last_seen_ts_utc: Option<i64>,
    pub age_days_last: Option<f64>,
    pub status: ZoneStatus,
    pub notes: Vec<String>,
    pub drift: Option<DriftInfo>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum ZoneStatus {
    Unknown,
    Stale,
    Fresh,
    Drifting,
}

#[derive(Debug, Clone, Serialize)]
pub struct FishEvidence {
    pub fish_id: i32,
    pub fish_name: Option<String>,
    pub evidence_weight: f64,
    pub p_mean: f64,
    pub ci_low: Option<f64>,
    pub ci_high: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DriftInfo {
    pub boundary_ts_utc: i64,
    pub jsd_mean: f64,
    pub p_drift: f64,
    pub ess_old: f64,
    pub ess_new: f64,
    pub samples: usize,
    pub jsd_threshold: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EffortGrid {
    pub tile_px: u32,
    pub grid_w: i32,
    pub grid_h: i32,
    pub sigma_tiles: f64,
    pub values: Vec<f64>,
}

pub fn zone_stats_to_json(stats: &ZoneStats) -> String {
    let mut out = String::new();
    out.push('{');
    out.push_str(&format!("\"zone_rgb_u32\":{},", stats.zone_rgb_u32));
    out.push_str(&format!("\"zone_rgb\":\"{}\",", stats.zone_rgb));
    match &stats.zone_name {
        Some(name) => out.push_str(&format!("\"zone_name\":\"{}\",", json_escape(name))),
        None => out.push_str("\"zone_name\":null,"),
    }
    out.push_str("\"window\":{");
    out.push_str(&format!("\"from_ts_utc\":{},", stats.window.from_ts_utc));
    out.push_str(&format!("\"to_ts_utc\":{},", stats.window.to_ts_utc));
    match stats.window.half_life_days {
        Some(v) => out.push_str(&format!("\"half_life_days\":{},", v)),
        None => out.push_str("\"half_life_days\":null,"),
    }
    out.push_str(&format!("\"fish_norm\":{},", stats.window.fish_norm));
    out.push_str(&format!("\"tile_px\":{},", stats.window.tile_px));
    out.push_str(&format!("\"sigma_tiles\":{},", stats.window.sigma_tiles));
    out.push_str(&format!("\"alpha0\":{}", stats.window.alpha0));
    out.push_str("},");
    out.push_str("\"confidence\":{");
    out.push_str(&format!("\"ess\":{},", stats.confidence.ess));
    out.push_str(&format!(
        "\"total_weight\":{},",
        stats.confidence.total_weight
    ));
    match stats.confidence.last_seen_ts_utc {
        Some(v) => out.push_str(&format!("\"last_seen_ts_utc\":{},", v)),
        None => out.push_str("\"last_seen_ts_utc\":null,"),
    }
    match stats.confidence.age_days_last {
        Some(v) => out.push_str(&format!("\"age_days_last\":{},", v)),
        None => out.push_str("\"age_days_last\":null,"),
    }
    out.push_str(&format!("\"status\":\"{:?}\",", stats.confidence.status));
    out.push_str("\"notes\":[");
    for (i, note) in stats.confidence.notes.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!("\"{}\"", json_escape(note)));
    }
    out.push(']');
    out.push_str(",\"drift\":");
    if let Some(drift) = &stats.confidence.drift {
        out.push('{');
        out.push_str(&format!("\"boundary_ts_utc\":{},", drift.boundary_ts_utc));
        out.push_str(&format!("\"jsd_mean\":{},", drift.jsd_mean));
        out.push_str(&format!("\"p_drift\":{},", drift.p_drift));
        out.push_str(&format!("\"ess_old\":{},", drift.ess_old));
        out.push_str(&format!("\"ess_new\":{},", drift.ess_new));
        out.push_str(&format!("\"samples\":{},", drift.samples));
        out.push_str(&format!("\"jsd_threshold\":{}", drift.jsd_threshold));
        out.push('}');
    } else {
        out.push_str("null");
    }
    out.push_str("},");
    out.push_str("\"distribution\":[");
    for (i, fish) in stats.distribution.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('{');
        out.push_str(&format!("\"fish_id\":{},", fish.fish_id));
        match &fish.fish_name {
            Some(name) => out.push_str(&format!("\"fish_name\":\"{}\",", json_escape(name))),
            None => out.push_str("\"fish_name\":null,"),
        }
        out.push_str(&format!("\"evidence_weight\":{},", fish.evidence_weight));
        out.push_str(&format!("\"p_mean\":{},", fish.p_mean));
        match fish.ci_low {
            Some(v) => out.push_str(&format!("\"ci_low\":{},", v)),
            None => out.push_str("\"ci_low\":null,"),
        }
        match fish.ci_high {
            Some(v) => out.push_str(&format!("\"ci_high\":{}", v)),
            None => out.push_str("\"ci_high\":null"),
        }
        out.push('}');
    }
    out.push(']');
    out.push('}');
    out
}

pub fn effort_grid_to_json(grid: &EffortGrid) -> String {
    let mut out = String::new();
    out.push('{');
    out.push_str(&format!("\"tile_px\":{},", grid.tile_px));
    out.push_str(&format!("\"grid_w\":{},", grid.grid_w));
    out.push_str(&format!("\"grid_h\":{},", grid.grid_h));
    out.push_str(&format!("\"sigma_tiles\":{},", grid.sigma_tiles));
    out.push_str("\"values\":[");
    for (i, v) in grid.values.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!("{v}"));
    }
    out.push(']');
    out.push('}');
    out
}

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

pub fn compute_effort_grid(store: &SqliteStore, params: &QueryParams) -> Result<EffortGrid> {
    params.validate()?;
    if !store
        .has_event_zone(&params.map_version)
        .context("check event_zone")?
    {
        bail!(
            "event_zone missing for map_version={}; run fishystuff_ingest index-zone-mask --map-version ...",
            params.map_version
        );
    }

    let tile_px = params.tile_px as i32;
    let (grid_w, grid_h, water_counts) = store.load_water_tiles(tile_px)?;
    let events = store.load_events_with_zone_in_window(
        &params.map_version,
        params.from_ts_utc,
        params.to_ts_utc,
    )?;

    let len = (grid_w * grid_h) as usize;
    let mut e_raw = vec![0.0f64; len];
    for ev in &events {
        let idx = tile_index(grid_w, grid_h, ev.tile_x, ev.tile_y)?;
        let w_time = time_weight(params, ev.ts_utc)?;
        e_raw[idx] += w_time;
    }
    let m: Vec<f64> = water_counts.into_iter().map(|v| v as f64).collect();
    let e_blur = gaussian_blur_grid(&e_raw, grid_w as usize, grid_h as usize, params.sigma_tiles);
    let m_blur = gaussian_blur_grid(&m, grid_w as usize, grid_h as usize, params.sigma_tiles);
    let mut effort = Vec::with_capacity(len);
    for i in 0..len {
        effort.push(e_blur[i] / m_blur[i].max(EPS));
    }

    Ok(EffortGrid {
        tile_px: params.tile_px,
        grid_w,
        grid_h,
        sigma_tiles: params.sigma_tiles,
        values: effort,
    })
}

pub fn compute_zone_stats(
    store: &SqliteStore,
    zones_meta: &HashMap<u32, ZoneMeta>,
    fish_names: &HashMap<i32, String>,
    params: &QueryParams,
    zone_rgb_u32: u32,
) -> Result<ZoneStats> {
    compute_zone_stats_with_config(
        store,
        zones_meta,
        fish_names,
        params,
        zone_rgb_u32,
        &ZoneStatusConfig::default(),
    )
}

pub fn compute_zone_stats_with_config(
    store: &SqliteStore,
    zones_meta: &HashMap<u32, ZoneMeta>,
    fish_names: &HashMap<i32, String>,
    params: &QueryParams,
    zone_rgb_u32: u32,
    status_cfg: &ZoneStatusConfig,
) -> Result<ZoneStats> {
    params.validate()?;
    if !store
        .has_event_zone(&params.map_version)
        .context("check event_zone")?
    {
        bail!(
            "event_zone missing for map_version={}; run fishystuff_ingest index-zone-mask --map-version ...",
            params.map_version
        );
    }

    let window = WindowInfo {
        from_ts_utc: params.from_ts_utc,
        to_ts_utc: params.to_ts_utc,
        half_life_days: params.half_life_days,
        fish_norm: params.fish_norm,
        tile_px: params.tile_px,
        sigma_tiles: params.sigma_tiles,
        alpha0: params.alpha0,
    };
    let summary = compute_window_summary(store, params, zone_rgb_u32)?;
    if summary.alpha_by_fish.is_empty() || summary.alpha_total <= 0.0 {
        return Ok(ZoneStats {
            zone_rgb_u32,
            zone_rgb: format_rgb_u32(zone_rgb_u32),
            zone_name: zones_meta.get(&zone_rgb_u32).and_then(|m| m.name.clone()),
            window,
            confidence: ZoneConfidence {
                ess: 0.0,
                total_weight: summary.total_weight,
                last_seen_ts_utc: summary.last_seen,
                age_days_last: None,
                status: ZoneStatus::Unknown,
                notes: vec!["no evidence in window".to_string()],
                drift: None,
            },
            distribution: Vec::new(),
        });
    }

    let mut dist = Vec::new();
    for fish_id in zone_distribution_fish_ids(&summary) {
        let p_mean = summary.p_mean_by_fish.get(&fish_id).copied().unwrap_or(0.0);
        let evidence = summary.c_zone.get(&fish_id).copied().unwrap_or(0.0);
        dist.push(FishEvidence {
            fish_id,
            fish_name: fish_names.get(&fish_id).cloned(),
            evidence_weight: evidence,
            p_mean,
            ci_low: None,
            ci_high: None,
        });
    }

    dist.sort_by(|a, b| {
        b.p_mean
            .partial_cmp(&a.p_mean)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.fish_id.cmp(&b.fish_id))
    });
    if dist.len() > params.top_k {
        dist.truncate(params.top_k);
    }

    let mut with_ci = Vec::with_capacity(dist.len());
    for mut fish in dist {
        let alpha = summary
            .alpha_by_fish
            .get(&fish.fish_id)
            .copied()
            .unwrap_or(0.0);
        let beta = (summary.alpha_total - alpha).max(0.0);
        if alpha > 0.0 && beta > 0.0 {
            let seed = seed_from_params(
                &params.map_version,
                zone_rgb_u32,
                fish.fish_id,
                params.from_ts_utc,
                params.to_ts_utc,
            );
            let (low, high) = beta_ci(alpha, beta, seed, 2000);
            fish.ci_low = Some(low);
            fish.ci_high = Some(high);
        }
        with_ci.push(fish);
    }

    let ess = summary.ess;
    let (mut status, age_days_last, mut notes) = compute_status(
        summary.total_weight,
        summary.last_seen,
        params.to_ts_utc,
        ess,
        status_cfg,
    );

    let mut drift_info = None;
    if let Some(boundary) = params.drift_boundary_ts {
        let (info, drifting, drift_note) =
            compute_drift_info(store, params, zone_rgb_u32, boundary, status_cfg)?;
        drift_info = info;
        if let Some(note) = drift_note {
            notes.push(note);
        }
        if status != ZoneStatus::Unknown && drifting {
            status = ZoneStatus::Drifting;
        }
    }

    Ok(ZoneStats {
        zone_rgb_u32,
        zone_rgb: format_rgb_u32(zone_rgb_u32),
        zone_name: zones_meta.get(&zone_rgb_u32).and_then(|m| m.name.clone()),
        window,
        confidence: ZoneConfidence {
            ess,
            total_weight: summary.total_weight,
            last_seen_ts_utc: summary.last_seen,
            age_days_last,
            status,
            notes,
            drift: drift_info,
        },
        distribution: with_ci,
    })
}

fn compute_status(
    total_weight: f64,
    last_seen: Option<i64>,
    to_ts_utc: i64,
    ess: f64,
    cfg: &ZoneStatusConfig,
) -> (ZoneStatus, Option<f64>, Vec<String>) {
    if total_weight <= 0.0 || last_seen.is_none() {
        return (
            ZoneStatus::Unknown,
            None,
            vec!["no events in window".to_string()],
        );
    }
    let last_seen = last_seen.unwrap();
    let age_days = (to_ts_utc - last_seen) as f64 / 86_400.0;
    let mut notes = Vec::new();
    let mut stale = false;
    if age_days > cfg.stale_days_threshold {
        stale = true;
        notes.push(format!(
            "last_seen age_days {:.2} > stale_days_threshold {:.2}",
            age_days, cfg.stale_days_threshold
        ));
    }
    if ess < cfg.ess_threshold {
        stale = true;
        notes.push(format!(
            "ess {:.2} < ess_threshold {:.2}",
            ess, cfg.ess_threshold
        ));
    }
    (
        if stale {
            ZoneStatus::Stale
        } else {
            ZoneStatus::Fresh
        },
        Some(age_days),
        notes,
    )
}

#[derive(Debug, Clone)]
struct DerivedEvent {
    ts_utc: i64,
    fish_id: i32,
    tile_idx: usize,
    zone_rgb_u32: u32,
    w_time: f64,
}

#[derive(Debug, Clone)]
struct WindowSummary {
    alpha_total: f64,
    alpha_by_fish: HashMap<i32, f64>,
    p_mean_by_fish: HashMap<i32, f64>,
    c_zone: HashMap<i32, f64>,
    ess: f64,
    total_weight: f64,
    last_seen: Option<i64>,
}

fn tile_index(grid_w: i32, grid_h: i32, tile_x: i32, tile_y: i32) -> Result<usize> {
    if tile_x < 0 || tile_y < 0 || tile_x >= grid_w || tile_y >= grid_h {
        bail!(
            "tile coordinate out of bounds: tile_x={}, tile_y={}, grid={}x{}",
            tile_x,
            tile_y,
            grid_w,
            grid_h
        );
    }
    Ok((tile_y * grid_w + tile_x) as usize)
}

fn time_weight(params: &QueryParams, ts_utc: i64) -> Result<f64> {
    if let Some(half) = params.half_life_days {
        let age_days = (params.to_ts_utc - ts_utc) as f64 / 86_400.0;
        Ok(2.0f64.powf(-age_days / half))
    } else {
        Ok(1.0)
    }
}

fn clamp(v: f64, min: f64, max: f64) -> f64 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

fn compute_window_summary(
    store: &SqliteStore,
    params: &QueryParams,
    zone_rgb_u32: u32,
) -> Result<WindowSummary> {
    let tile_px = params.tile_px as i32;
    let (grid_w, grid_h, water_counts) = store.load_water_tiles(tile_px)?;

    let events = store.load_events_with_zone_in_window(
        &params.map_version,
        params.from_ts_utc,
        params.to_ts_utc,
    )?;

    if events.is_empty() {
        return Ok(WindowSummary {
            alpha_total: 0.0,
            alpha_by_fish: HashMap::new(),
            p_mean_by_fish: HashMap::new(),
            c_zone: HashMap::new(),
            ess: 0.0,
            total_weight: 0.0,
            last_seen: None,
        });
    }

    let len = (grid_w * grid_h) as usize;
    let mut e_raw = vec![0.0f64; len];
    let mut derived: Vec<DerivedEvent> = Vec::with_capacity(events.len());
    let mut fish_time: HashMap<i32, f64> = HashMap::new();
    for ev in events {
        let idx = tile_index(grid_w, grid_h, ev.tile_x, ev.tile_y)?;
        let w_time = time_weight(params, ev.ts_utc)?;
        e_raw[idx] += w_time;
        *fish_time.entry(ev.fish_id).or_insert(0.0) += w_time;
        derived.push(DerivedEvent {
            ts_utc: ev.ts_utc,
            fish_id: ev.fish_id,
            tile_idx: idx,
            zone_rgb_u32: ev.zone_rgb_u32,
            w_time,
        });
    }

    let m: Vec<f64> = water_counts.into_iter().map(|v| v as f64).collect();
    let e_blur = gaussian_blur_grid(&e_raw, grid_w as usize, grid_h as usize, params.sigma_tiles);
    let m_blur = gaussian_blur_grid(&m, grid_w as usize, grid_h as usize, params.sigma_tiles);
    let mut effort = Vec::with_capacity(len);
    for i in 0..len {
        effort.push(e_blur[i] / m_blur[i].max(EPS));
    }
    let eff_med = median_effort(&e_raw, &effort).unwrap_or(1.0);

    let mut fish_norm = HashMap::new();
    if params.fish_norm {
        for (fish_id, sum) in fish_time {
            let w = 1.0 / sum.max(EPS_FISH);
            fish_norm.insert(fish_id, w);
        }
    }

    let mut c_global: HashMap<i32, f64> = HashMap::new();
    let mut c_zone: HashMap<i32, f64> = HashMap::new();
    let mut w_sum = 0.0f64;
    let mut w2_sum = 0.0f64;
    let mut last_seen: Option<i64> = None;

    for ev in derived {
        let eff = effort[ev.tile_idx].max(EPS_EFF);
        let w_eff = clamp(eff_med / eff, 0.1, 10.0);
        let u = ev.w_time * w_eff;
        let w = if params.fish_norm {
            u * fish_norm.get(&ev.fish_id).copied().unwrap_or(0.0)
        } else {
            u
        };
        *c_global.entry(ev.fish_id).or_insert(0.0) += w;
        if ev.zone_rgb_u32 == zone_rgb_u32 {
            *c_zone.entry(ev.fish_id).or_insert(0.0) += w;
            w_sum += u;
            w2_sum += u * u;
            last_seen = Some(last_seen.map_or(ev.ts_utc, |v| v.max(ev.ts_utc)));
        }
    }

    let total_global: f64 = c_global.values().sum();
    if total_global <= 0.0 {
        return Ok(WindowSummary {
            alpha_total: 0.0,
            alpha_by_fish: HashMap::new(),
            p_mean_by_fish: HashMap::new(),
            c_zone,
            ess: if w2_sum > 0.0 {
                (w_sum * w_sum) / w2_sum.max(EPS)
            } else {
                0.0
            },
            total_weight: w_sum,
            last_seen,
        });
    }

    let mut fish_ids: Vec<i32> = c_global.keys().copied().collect();
    fish_ids.sort_unstable();
    let mut alpha_total = params.alpha0;
    let mut alpha_by_fish = HashMap::new();
    let mut p_mean_by_fish = HashMap::new();
    for fish_id in &fish_ids {
        let p0 = c_global.get(fish_id).copied().unwrap_or(0.0) / total_global;
        let c = c_zone.get(fish_id).copied().unwrap_or(0.0);
        let alpha = params.alpha0 * p0 + c;
        alpha_total += c;
        alpha_by_fish.insert(*fish_id, alpha);
    }
    for (fish_id, alpha) in &alpha_by_fish {
        p_mean_by_fish.insert(*fish_id, *alpha / alpha_total);
    }

    let ess = if w2_sum > 0.0 {
        (w_sum * w_sum) / w2_sum.max(EPS)
    } else {
        0.0
    };

    Ok(WindowSummary {
        alpha_total,
        alpha_by_fish,
        p_mean_by_fish,
        c_zone,
        ess,
        total_weight: w_sum,
        last_seen,
    })
}

fn zone_distribution_fish_ids(summary: &WindowSummary) -> Vec<i32> {
    let mut fish_ids: Vec<i32> = summary.c_zone.keys().copied().collect();
    fish_ids.sort_unstable();
    fish_ids
}

fn median_effort(e_raw: &[f64], effort: &[f64]) -> Option<f64> {
    let mut values: Vec<f64> = e_raw
        .iter()
        .zip(effort.iter())
        .filter_map(|(&e, &eff)| if e > 0.0 { Some(eff) } else { None })
        .collect();
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        Some(values[mid])
    } else {
        Some(0.5 * (values[mid - 1] + values[mid]))
    }
}

fn gaussian_kernel_1d(sigma: f64) -> Vec<f64> {
    if sigma <= 0.0 {
        return vec![1.0];
    }
    let radius = (sigma * 3.0).ceil() as i32;
    let mut kernel = Vec::with_capacity((2 * radius + 1) as usize);
    let mut sum = 0.0f64;
    for i in -radius..=radius {
        let x = i as f64;
        let v = (-0.5 * (x / sigma).powi(2)).exp();
        kernel.push(v);
        sum += v;
    }
    for v in &mut kernel {
        *v /= sum;
    }
    kernel
}

fn gaussian_blur_grid(input: &[f64], width: usize, height: usize, sigma: f64) -> Vec<f64> {
    if width == 0 || height == 0 {
        return Vec::new();
    }
    if sigma <= 0.0 {
        return input.to_vec();
    }
    let kernel = gaussian_kernel_1d(sigma);
    let radius = (kernel.len() as i32 - 1) / 2;
    let mut tmp = vec![0.0f64; input.len()];
    let mut out = vec![0.0f64; input.len()];

    for y in 0..height {
        let row = y * width;
        for x in 0..width {
            let mut acc = 0.0f64;
            for (k, weight) in kernel.iter().enumerate() {
                let dx = k as i32 - radius;
                let sx = clamp_i32(x as i32 + dx, 0, width as i32 - 1) as usize;
                acc += input[row + sx] * weight;
            }
            tmp[row + x] = acc;
        }
    }

    for y in 0..height {
        for x in 0..width {
            let mut acc = 0.0f64;
            for (k, weight) in kernel.iter().enumerate() {
                let dy = k as i32 - radius;
                let sy = clamp_i32(y as i32 + dy, 0, height as i32 - 1) as usize;
                acc += tmp[sy * width + x] * weight;
            }
            out[y * width + x] = acc;
        }
    }
    out
}

fn clamp_i32(v: i32, min: i32, max: i32) -> i32 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 8);
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

fn seed_from_params(
    map_version: &str,
    zone_rgb_u32: u32,
    fish_id: i32,
    from_ts: i64,
    to_ts: i64,
) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for b in map_version.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    for b in zone_rgb_u32.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    for b in fish_id.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    for b in from_ts.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    for b in to_ts.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

fn seed_from_drift(
    map_version: &str,
    zone_rgb_u32: u32,
    boundary: i64,
    from_ts: i64,
    to_ts: i64,
) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for b in map_version.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    for b in zone_rgb_u32.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    for b in boundary.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    for b in from_ts.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    for b in to_ts.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

fn beta_ci(alpha: f64, beta: f64, seed: u64, samples: usize) -> (f64, f64) {
    let mut rng = XorShift64::new(seed);
    let mut values = Vec::with_capacity(samples);
    for _ in 0..samples {
        values.push(sample_beta(alpha, beta, &mut rng));
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let low = quantile_sorted(&values, 0.05);
    let high = quantile_sorted(&values, 0.95);
    (low, high)
}

fn quantile_sorted(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let idx = (p.clamp(0.0, 1.0) * (values.len() as f64 - 1.0)).round() as usize;
    values[idx]
}

fn compute_drift_info(
    store: &SqliteStore,
    params: &QueryParams,
    zone_rgb_u32: u32,
    boundary: i64,
    cfg: &ZoneStatusConfig,
) -> Result<(Option<DriftInfo>, bool, Option<String>)> {
    let mut old_params = params.clone();
    old_params.to_ts_utc = boundary;
    old_params.drift_boundary_ts = None;
    let mut new_params = params.clone();
    new_params.from_ts_utc = boundary;
    new_params.drift_boundary_ts = None;

    let old = compute_window_summary(store, &old_params, zone_rgb_u32)?;
    let new = compute_window_summary(store, &new_params, zone_rgb_u32)?;

    let mut union: Vec<i32> = old
        .alpha_by_fish
        .keys()
        .chain(new.alpha_by_fish.keys())
        .copied()
        .collect();
    union.sort_unstable();
    union.dedup();

    if union.is_empty() {
        return Ok((None, false, Some("drift skipped: no evidence".to_string())));
    }

    let p_old = align_probs(&old.p_mean_by_fish, &union);
    let p_new = align_probs(&new.p_mean_by_fish, &union);
    let jsd_mean = js_divergence(&p_old, &p_new);

    let mut p_drift = 0.0;
    let mut drifting = false;
    let mut note = None;
    if old.ess >= cfg.drift_min_ess && new.ess >= cfg.drift_min_ess {
        let alpha_old = align_alpha(&old.alpha_by_fish, &union);
        let alpha_new = align_alpha(&new.alpha_by_fish, &union);
        let seed = seed_from_drift(
            &params.map_version,
            zone_rgb_u32,
            boundary,
            params.from_ts_utc,
            params.to_ts_utc,
        );
        let mut rng = XorShift64::new(seed);
        let mut count = 0usize;
        for _ in 0..cfg.drift_samples {
            let s_old = sample_dirichlet(&alpha_old, &mut rng);
            let s_new = sample_dirichlet(&alpha_new, &mut rng);
            let jsd = js_divergence(&s_old, &s_new);
            if jsd > cfg.drift_jsd_threshold {
                count += 1;
            }
        }
        p_drift = count as f64 / cfg.drift_samples as f64;
        drifting = p_drift >= cfg.drift_prob_threshold;
    } else {
        note = Some("drift skipped: insufficient ESS".to_string());
    }

    let info = DriftInfo {
        boundary_ts_utc: boundary,
        jsd_mean,
        p_drift,
        ess_old: old.ess,
        ess_new: new.ess,
        samples: cfg.drift_samples,
        jsd_threshold: cfg.drift_jsd_threshold,
    };
    Ok((Some(info), drifting, note))
}

fn align_probs(map: &HashMap<i32, f64>, fish_ids: &[i32]) -> Vec<f64> {
    fish_ids
        .iter()
        .map(|id| map.get(id).copied().unwrap_or(0.0))
        .collect()
}

fn align_alpha(map: &HashMap<i32, f64>, fish_ids: &[i32]) -> Vec<f64> {
    fish_ids
        .iter()
        .map(|id| map.get(id).copied().unwrap_or(0.0))
        .collect()
}

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        let seed = if seed == 0 { 0x9e3779b97f4a7c15 } else { seed };
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(2685821657736338717)
    }

    fn next_f64(&mut self) -> f64 {
        let v = self.next_u64() >> 11;
        v as f64 * (1.0 / 9007199254740992.0)
    }
}

fn uniform_open01(rng: &mut XorShift64) -> f64 {
    loop {
        let v = rng.next_f64();
        if v > 0.0 && v < 1.0 {
            return v;
        }
    }
}

fn sample_normal(rng: &mut XorShift64) -> f64 {
    let u1 = uniform_open01(rng);
    let u2 = rng.next_f64();
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

fn sample_gamma(shape: f64, rng: &mut XorShift64) -> f64 {
    if shape <= 0.0 {
        return 0.0;
    }
    if shape < 1.0 {
        let u = uniform_open01(rng);
        return sample_gamma(shape + 1.0, rng) * u.powf(1.0 / shape);
    }
    let d = shape - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();
    loop {
        let x = sample_normal(rng);
        let v = (1.0 + c * x).powi(3);
        if v <= 0.0 {
            continue;
        }
        let u = uniform_open01(rng);
        if u < 1.0 - 0.0331 * x.powi(4) {
            return d * v;
        }
        if u.ln() < 0.5 * x * x + d * (1.0 - v + v.ln()) {
            return d * v;
        }
    }
}

fn sample_beta(alpha: f64, beta: f64, rng: &mut XorShift64) -> f64 {
    let x = sample_gamma(alpha, rng);
    let y = sample_gamma(beta, rng);
    if x + y == 0.0 {
        0.0
    } else {
        x / (x + y)
    }
}

fn sample_dirichlet(alphas: &[f64], rng: &mut XorShift64) -> Vec<f64> {
    let mut out = Vec::with_capacity(alphas.len());
    let mut sum = 0.0;
    for &a in alphas {
        let v = sample_gamma(a, rng);
        out.push(v);
        sum += v;
    }
    if sum <= 0.0 {
        if alphas.is_empty() {
            return Vec::new();
        }
        let v = 1.0 / alphas.len() as f64;
        return vec![v; alphas.len()];
    }
    for v in &mut out {
        *v /= sum;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use fishystuff_core::masks::pack_rgb_u32;
    use fishystuff_store::{Event, WaterTile};

    #[test]
    fn gaussian_kernel_sums_to_one() {
        let k = gaussian_kernel_1d(1.5);
        let sum: f64 = k.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn gaussian_blur_constant_grid() {
        let input = vec![2.0f64; 9];
        let output = gaussian_blur_grid(&input, 3, 3, 1.0);
        for v in output {
            assert!((v - 2.0).abs() < 1e-9);
        }
    }

    #[test]
    fn beta_sampler_mean_reasonable() {
        let mut rng = XorShift64::new(42);
        let mut sum = 0.0;
        let n = 5000;
        for _ in 0..n {
            sum += sample_beta(2.0, 5.0, &mut rng);
        }
        let mean = sum / n as f64;
        let expected = 2.0 / 7.0;
        assert!((mean - expected).abs() < 0.02);
    }

    #[test]
    fn beta_ci_monotonic() {
        let (low, high) = beta_ci(2.0, 3.0, 123, 1000);
        assert!(low <= high);
    }

    #[test]
    fn compute_zone_stats_basic() {
        let mut store = SqliteStore::open_in_memory().expect("db");
        let mut events = Vec::new();
        let zone_a = pack_rgb_u32(10, 20, 30);
        let zone_b = pack_rgb_u32(40, 50, 60);
        for i in 0..12 {
            events.push(Event {
                ts_utc: 1000 + i as i64,
                fish_id: 1,
                world_x: 0.0,
                world_z: 0.0,
                px: Some(0),
                py: Some(0),
                water_px: Some(0),
                water_py: Some(0),
                tile_x: Some(0),
                tile_y: Some(0),
                water_ok: true,
            });
        }
        for i in 0..4 {
            events.push(Event {
                ts_utc: 2000 + i as i64,
                fish_id: 2,
                world_x: 0.0,
                world_z: 0.0,
                px: Some(0),
                py: Some(0),
                water_px: Some(0),
                water_py: Some(0),
                tile_x: Some(0),
                tile_y: Some(0),
                water_ok: true,
            });
        }
        store.insert_events(&events).expect("insert events");

        let mut rows = Vec::new();
        for id in 1..=12 {
            rows.push((id as i64, zone_a));
        }
        for id in 13..=16 {
            rows.push((id as i64, zone_b));
        }
        store
            .insert_event_zones("v1", &rows, true)
            .expect("event zones");

        let tiles = vec![WaterTile {
            tile_px: 11_560,
            tile_x: 0,
            tile_y: 0,
            water_count: 100,
        }];
        store.upsert_water_tiles(&tiles).expect("water tiles");

        let mut zones_meta = HashMap::new();
        zones_meta.insert(
            zone_a,
            ZoneMeta {
                rgb_u32: zone_a,
                r: 10,
                g: 20,
                b: 30,
                name: Some("Zone A".to_string()),
                active: None,
                confirmed: None,
                index: None,
                bite_time_min: None,
                bite_time_max: None,
            },
        );

        let mut fish_names = HashMap::new();
        fish_names.insert(1, "Fish One".to_string());
        fish_names.insert(2, "Fish Two".to_string());

        let params = QueryParams {
            map_version: "v1".to_string(),
            from_ts_utc: 0,
            to_ts_utc: 10_000,
            half_life_days: None,
            tile_px: 11_560,
            sigma_tiles: 1.0,
            fish_norm: false,
            alpha0: 1.0,
            top_k: 5,
            drift_boundary_ts: None,
        };

        let stats =
            compute_zone_stats(&store, &zones_meta, &fish_names, &params, zone_a).expect("stats");
        assert_eq!(stats.zone_name.as_deref(), Some("Zone A"));
        assert_eq!(stats.confidence.status, ZoneStatus::Fresh);
        assert!(stats.confidence.ess >= 10.0);
        assert!(!stats.distribution.is_empty());
        assert!(stats.distribution.iter().any(|f| f.fish_id == 1));
    }

    #[test]
    fn compute_zone_stats_stale_by_age() {
        let mut store = SqliteStore::open_in_memory().expect("db");
        let zone_a = pack_rgb_u32(1, 2, 3);
        let events = vec![Event {
            ts_utc: 0,
            fish_id: 1,
            world_x: 0.0,
            world_z: 0.0,
            px: Some(0),
            py: Some(0),
            water_px: Some(0),
            water_py: Some(0),
            tile_x: Some(0),
            tile_y: Some(0),
            water_ok: true,
        }];
        store.insert_events(&events).expect("insert events");
        store
            .insert_event_zones("v1", &[(1, zone_a)], true)
            .expect("event zones");
        store
            .upsert_water_tiles(&[WaterTile {
                tile_px: 11_560,
                tile_x: 0,
                tile_y: 0,
                water_count: 100,
            }])
            .expect("water tiles");

        let params = QueryParams {
            map_version: "v1".to_string(),
            from_ts_utc: 0,
            to_ts_utc: 100 * 86_400,
            half_life_days: None,
            tile_px: 11_560,
            sigma_tiles: 1.0,
            fish_norm: false,
            alpha0: 1.0,
            top_k: 5,
            drift_boundary_ts: None,
        };
        let stats = compute_zone_stats(&store, &HashMap::new(), &HashMap::new(), &params, zone_a)
            .expect("stats");
        assert_eq!(stats.confidence.status, ZoneStatus::Stale);
        assert!(stats
            .confidence
            .notes
            .iter()
            .any(|n| n.contains("last_seen age_days")));
    }

    #[test]
    fn compute_zone_stats_hides_prior_only_fish_from_distribution() {
        let mut store = SqliteStore::open_in_memory().expect("db");
        let zone_a = pack_rgb_u32(10, 20, 30);
        let zone_b = pack_rgb_u32(40, 50, 60);
        let events = vec![
            Event {
                ts_utc: 100,
                fish_id: 1,
                world_x: 0.0,
                world_z: 0.0,
                px: Some(0),
                py: Some(0),
                water_px: Some(0),
                water_py: Some(0),
                tile_x: Some(0),
                tile_y: Some(0),
                water_ok: true,
            },
            Event {
                ts_utc: 200,
                fish_id: 2,
                world_x: 0.0,
                world_z: 0.0,
                px: Some(0),
                py: Some(0),
                water_px: Some(0),
                water_py: Some(0),
                tile_x: Some(0),
                tile_y: Some(0),
                water_ok: true,
            },
        ];
        store.insert_events(&events).expect("insert events");
        store
            .insert_event_zones("v1", &[(1, zone_a), (2, zone_b)], true)
            .expect("event zones");
        store
            .upsert_water_tiles(&[WaterTile {
                tile_px: 11_560,
                tile_x: 0,
                tile_y: 0,
                water_count: 100,
            }])
            .expect("water tiles");

        let mut fish_names = HashMap::new();
        fish_names.insert(1, "Zone Fish".to_string());
        fish_names.insert(2, "Prior Fish".to_string());

        let params = QueryParams {
            map_version: "v1".to_string(),
            from_ts_utc: 0,
            to_ts_utc: 1_000,
            half_life_days: None,
            tile_px: 11_560,
            sigma_tiles: 1.0,
            fish_norm: false,
            alpha0: 10.0,
            top_k: 10,
            drift_boundary_ts: None,
        };

        let stats = compute_zone_stats(&store, &HashMap::new(), &fish_names, &params, zone_a)
            .expect("stats");

        assert_eq!(stats.distribution.len(), 1);
        assert_eq!(stats.distribution[0].fish_id, 1);
        assert!(stats
            .distribution
            .iter()
            .all(|fish| fish.evidence_weight > 0.0));
    }

    #[test]
    fn compute_zone_stats_drifting() {
        let mut store = SqliteStore::open_in_memory().expect("db");
        let zone_a = pack_rgb_u32(5, 5, 5);
        let mut events = Vec::new();
        // Old window: fish 1
        for i in 0..30 {
            events.push(Event {
                ts_utc: 1000 + i,
                fish_id: 1,
                world_x: 0.0,
                world_z: 0.0,
                px: Some(0),
                py: Some(0),
                water_px: Some(0),
                water_py: Some(0),
                tile_x: Some(0),
                tile_y: Some(0),
                water_ok: true,
            });
        }
        // New window: fish 2
        for i in 0..30 {
            events.push(Event {
                ts_utc: 2000 + i,
                fish_id: 2,
                world_x: 0.0,
                world_z: 0.0,
                px: Some(0),
                py: Some(0),
                water_px: Some(0),
                water_py: Some(0),
                tile_x: Some(0),
                tile_y: Some(0),
                water_ok: true,
            });
        }
        store.insert_events(&events).expect("insert events");
        let mut rows = Vec::new();
        for id in 1..=60 {
            rows.push((id as i64, zone_a));
        }
        store
            .insert_event_zones("v1", &rows, true)
            .expect("event zones");
        store
            .upsert_water_tiles(&[WaterTile {
                tile_px: 11_560,
                tile_x: 0,
                tile_y: 0,
                water_count: 100,
            }])
            .expect("water tiles");

        let params = QueryParams {
            map_version: "v1".to_string(),
            from_ts_utc: 900,
            to_ts_utc: 3000,
            half_life_days: None,
            tile_px: 11_560,
            sigma_tiles: 1.0,
            fish_norm: false,
            alpha0: 1.0,
            top_k: 5,
            drift_boundary_ts: Some(1800),
        };
        let stats = compute_zone_stats(&store, &HashMap::new(), &HashMap::new(), &params, zone_a)
            .expect("stats");
        assert_eq!(stats.confidence.status, ZoneStatus::Drifting);
        assert!(stats.confidence.drift.is_some());
    }

    #[test]
    fn zone_stats_json_deterministic_and_sane() {
        let mut store = SqliteStore::open_in_memory().expect("db");
        let zone_a = pack_rgb_u32(9, 9, 9);
        let events = vec![
            Event {
                ts_utc: 100,
                fish_id: 1,
                world_x: 0.0,
                world_z: 0.0,
                px: Some(0),
                py: Some(0),
                water_px: Some(0),
                water_py: Some(0),
                tile_x: Some(0),
                tile_y: Some(0),
                water_ok: true,
            },
            Event {
                ts_utc: 200,
                fish_id: 2,
                world_x: 0.0,
                world_z: 0.0,
                px: Some(0),
                py: Some(0),
                water_px: Some(0),
                water_py: Some(0),
                tile_x: Some(0),
                tile_y: Some(0),
                water_ok: true,
            },
        ];
        store.insert_events(&events).expect("insert events");
        store
            .insert_event_zones("v1", &[(1, zone_a), (2, zone_a)], true)
            .expect("event zones");
        store
            .upsert_water_tiles(&[WaterTile {
                tile_px: 11_560,
                tile_x: 0,
                tile_y: 0,
                water_count: 100,
            }])
            .expect("water tiles");

        let params = QueryParams {
            map_version: "v1".to_string(),
            from_ts_utc: 0,
            to_ts_utc: 1000,
            half_life_days: None,
            tile_px: 11_560,
            sigma_tiles: 1.0,
            fish_norm: false,
            alpha0: 1.0,
            top_k: 10,
            drift_boundary_ts: None,
        };
        let stats1 = compute_zone_stats(&store, &HashMap::new(), &HashMap::new(), &params, zone_a)
            .expect("stats1");
        let stats2 = compute_zone_stats(&store, &HashMap::new(), &HashMap::new(), &params, zone_a)
            .expect("stats2");
        let json1 = zone_stats_to_json(&stats1);
        let json2 = zone_stats_to_json(&stats2);
        assert_eq!(json1, json2);
        assert!(stats1.confidence.ess > 0.0);
        for fish in stats1.distribution.iter() {
            assert!(fish.p_mean >= 0.0 && fish.p_mean <= 1.0);
            if let (Some(low), Some(high)) = (fish.ci_low, fish.ci_high) {
                assert!(low >= 0.0 && high <= 1.0);
                assert!(low <= fish.p_mean && fish.p_mean <= high);
            }
        }
    }
}
