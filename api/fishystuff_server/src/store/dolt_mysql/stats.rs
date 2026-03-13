use std::collections::HashMap;

use fishystuff_api::models::zone_stats::ZoneStatus;
use fishystuff_core::tile::pixel_to_tile;

use crate::config::ZoneStatusConfig;
use crate::error::AppResult;

use super::QueryParams;

pub(super) fn compute_status(
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

    let last_seen = last_seen.unwrap_or_default();
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

pub(super) fn pixel_to_tile_index(
    grid_w: i32,
    grid_h: i32,
    tile_px: i32,
    px_x: i32,
    px_y: i32,
) -> Option<usize> {
    let (tile_x, tile_y) = pixel_to_tile(px_x, px_y, tile_px);
    if tile_x < 0 || tile_y < 0 || tile_x >= grid_w || tile_y >= grid_h {
        return None;
    }
    Some((tile_y * grid_w + tile_x) as usize)
}

pub(super) fn time_weight(params: &QueryParams, ts_utc: i64) -> AppResult<f64> {
    if let Some(half) = params.half_life_days {
        let age_days = (params.to_ts_utc - ts_utc) as f64 / 86_400.0;
        Ok(2.0f64.powf(-age_days / half))
    } else {
        Ok(1.0)
    }
}

fn gaussian_kernel_1d(sigma: f64) -> Vec<f64> {
    if sigma <= 0.0 {
        return vec![1.0];
    }
    let radius = (sigma * 3.0).ceil() as i32;
    let mut kernel = Vec::with_capacity((2 * radius + 1) as usize);
    let mut sum = 0.0;
    for idx in -radius..=radius {
        let x = idx as f64;
        let value = (-0.5 * (x / sigma).powi(2)).exp();
        kernel.push(value);
        sum += value;
    }
    for value in &mut kernel {
        *value /= sum;
    }
    kernel
}

pub(super) fn gaussian_blur_grid(
    input: &[f64],
    width: usize,
    height: usize,
    sigma: f64,
) -> Vec<f64> {
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
            let mut acc = 0.0;
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
            let mut acc = 0.0;
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

fn clamp_i32(value: i32, min: i32, max: i32) -> i32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

pub(super) fn seed_from_params(
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

pub(super) fn seed_from_drift(
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

pub(super) fn beta_ci(alpha: f64, beta: f64, seed: u64, samples: usize) -> (f64, f64) {
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

pub(super) fn align_probs(map: &HashMap<i32, f64>, fish_ids: &[i32]) -> Vec<f64> {
    fish_ids
        .iter()
        .map(|id| map.get(id).copied().unwrap_or(0.0))
        .collect()
}

pub(super) fn align_alpha(map: &HashMap<i32, f64>, fish_ids: &[i32]) -> Vec<f64> {
    fish_ids
        .iter()
        .map(|id| map.get(id).copied().unwrap_or(0.0))
        .collect()
}

pub(super) struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    pub(super) fn new(seed: u64) -> Self {
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
        let value = self.next_u64() >> 11;
        value as f64 * (1.0 / 9007199254740992.0)
    }
}

fn uniform_open01(rng: &mut XorShift64) -> f64 {
    loop {
        let value = rng.next_f64();
        if value > 0.0 && value < 1.0 {
            return value;
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

pub(super) fn sample_dirichlet(alphas: &[f64], rng: &mut XorShift64) -> Vec<f64> {
    let mut out = Vec::with_capacity(alphas.len());
    let mut sum = 0.0;
    for &alpha in alphas {
        let value = sample_gamma(alpha, rng);
        out.push(value);
        sum += value;
    }
    if sum <= 0.0 {
        if alphas.is_empty() {
            return Vec::new();
        }
        return vec![1.0 / alphas.len() as f64; alphas.len()];
    }
    for value in &mut out {
        *value /= sum;
    }
    out
}
