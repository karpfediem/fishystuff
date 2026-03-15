use std::collections::{BTreeMap, HashMap};

use crate::map::layers::{LayerId, LayerSpec};

use super::super::cache::{RasterTileCache, RasterTileEntry, TileState};
use super::super::manifest::LoadedTileset;
use super::super::TileKey;
use super::{
    merge_level_count_maps, DesiredLayerTiles, LayerResidencyPlan, Level0Rect, ResidencyClass,
    TileBounds, TileResidencyState,
};

const EVICT_DETAIL_WEIGHT: f64 = 4.0;
const EVICT_AGE_WEIGHT: f64 = 1.0;
const EVICT_DISTANCE_WEIGHT: f64 = 2.0;
const EVICT_LAYER_WEIGHT: f64 = 8.0;

pub(crate) fn build_layer_residency_plan(
    layer: &LayerSpec,
    tileset: &LoadedTileset,
    desired: DesiredLayerTiles,
    map_version_id: u64,
    cache: &RasterTileCache,
    camera_unstable: bool,
) -> LayerResidencyPlan {
    crate::perf_scope!("raster.desired_tile_set_build");
    let mut plan = LayerResidencyPlan {
        max_detail_requests_while_moving: layer.lod_policy.max_detail_requests_while_camera_moving,
        motion_suppresses_refine: layer.lod_policy.motion_suppresses_refine,
        ..Default::default()
    };
    let protected_margin = layer.lod_policy.protected_margin_tiles.max(0);
    let mut warm_margin = layer.lod_policy.warm_margin_tiles.max(0);
    if camera_unstable {
        warm_margin = warm_margin.saturating_add(1);
    }

    let mut protected_bounds = Vec::new();
    let mut warm_bounds = Vec::new();

    if let Some(base) = desired.base {
        if let Some(rect) = bounds_level0_rect(base) {
            plan.desired_spans.push(rect);
        }
        protected_bounds.push(expand_bounds(base, tileset, protected_margin));
        warm_bounds.push(expand_bounds(base, tileset, warm_margin));
        plan = ingest_desired_coverage(layer, tileset, base, map_version_id, cache, true, plan);
    }
    if let Some(detail) = desired.detail {
        if let Some(rect) = bounds_level0_rect(detail) {
            plan.desired_spans.push(rect);
        }
        protected_bounds.push(expand_bounds(detail, tileset, protected_margin));
        warm_bounds.push(expand_bounds(detail, tileset, warm_margin));
        plan = ingest_desired_coverage(layer, tileset, detail, map_version_id, cache, false, plan);
    }

    for (key, entry) in &cache.entries {
        if key.layer != layer.id
            || key.map_version != map_version_id
            || entry.state != TileState::Ready
        {
            continue;
        }
        if is_pinned_coarse_level(
            key.z,
            tileset.max_level,
            layer.lod_policy.pinned_coarse_levels,
            layer.lod_policy.coarse_pin_min_level,
        ) {
            plan.protected.insert(*key);
            plan.warm.insert(*key);
        }
    }

    for (key, entry) in &cache.entries {
        if key.layer != layer.id
            || key.map_version != map_version_id
            || entry.state != TileState::Ready
        {
            continue;
        }
        if protected_bounds
            .iter()
            .any(|bounds| tile_overlaps_bounds(key, *bounds))
        {
            plan.protected.insert(*key);
            continue;
        }
        if warm_bounds
            .iter()
            .any(|bounds| tile_overlaps_bounds(key, *bounds))
        {
            plan.warm.insert(*key);
            continue;
        }
        let recently_used = cache.use_counter.saturating_sub(entry.last_used) <= 180;
        if recently_used {
            plan.warm.insert(*key);
        }
    }

    for key in plan.render_visible.clone() {
        if !plan.protected.contains(&key) {
            plan.warm.insert(key);
        }
    }

    plan
}

fn ingest_desired_coverage(
    layer: &LayerSpec,
    tileset: &LoadedTileset,
    bounds: TileBounds,
    map_version_id: u64,
    cache: &RasterTileCache,
    protect_direct: bool,
    mut plan: LayerResidencyPlan,
) -> LayerResidencyPlan {
    let Some(level) = tileset.level(bounds.z) else {
        return plan;
    };
    for ty in bounds.min_ty..=bounds.max_ty {
        for tx in bounds.min_tx..=bounds.max_tx {
            if !level.contains(tx, ty) {
                continue;
            }
            let key = TileKey {
                layer: layer.id,
                map_version: map_version_id,
                z: bounds.z,
                tx,
                ty,
            };
            if cache.contains_ready(&key) {
                plan.render_visible.insert(key);
                if protect_direct {
                    plan.protected.insert(key);
                }
                continue;
            }
            if let Some(ancestor) = cache.nearest_loaded_ancestor(key, tileset) {
                plan.render_visible.insert(ancestor);
                plan.fallback_visible.insert(ancestor);
                plan.protected.insert(ancestor);
            } else {
                if let Some(ancestor_key) = nearest_available_ancestor_key(key, tileset) {
                    if !cache.contains(&ancestor_key) {
                        plan.ancestor_requests.insert(ancestor_key);
                    }
                }
                plan.blank_visible_count = plan.blank_visible_count.saturating_add(1);
            }
        }
    }
    plan
}

fn nearest_available_ancestor_key(key: TileKey, tileset: &LoadedTileset) -> Option<TileKey> {
    let mut tx = key.tx;
    let mut ty = key.ty;
    let mut current_z = key.z;
    for level in tileset.levels.iter().filter(|level| level.z > key.z) {
        while current_z < level.z {
            tx = tx.div_euclid(2);
            ty = ty.div_euclid(2);
            current_z += 1;
        }
        if !level.contains(tx, ty) {
            continue;
        }
        return Some(TileKey {
            layer: key.layer,
            map_version: key.map_version,
            z: level.z,
            tx,
            ty,
        });
    }
    None
}

pub(crate) fn apply_layer_residency_plan(
    layer_id: LayerId,
    plan: LayerResidencyPlan,
    residency: &mut TileResidencyState,
) {
    crate::perf_scope!("raster.residency_apply");
    residency
        .render_visible
        .extend(plan.render_visible.iter().copied());
    residency.protected.extend(plan.protected.iter().copied());
    residency.warm.extend(plan.warm.iter().copied());
    residency
        .fallback_visible
        .extend(plan.fallback_visible.iter().copied());
    residency
        .ancestor_requests
        .extend(plan.ancestor_requests.iter().copied());
    residency
        .desired_spans_by_layer
        .insert(layer_id, plan.desired_spans);
    residency
        .blank_visible_by_layer
        .insert(layer_id, plan.blank_visible_count);
    residency
        .protected_by_layer_level
        .insert(layer_id, count_tile_levels(plan.protected.iter().copied()));
    residency
        .warm_by_layer_level
        .insert(layer_id, count_tile_levels(plan.warm.iter().copied()));
    residency.fallback_by_layer_level.insert(
        layer_id,
        count_tile_levels(plan.fallback_visible.iter().copied()),
    );
    residency
        .max_detail_requests_while_moving_by_layer
        .insert(layer_id, plan.max_detail_requests_while_moving.max(1));
    residency
        .motion_suppresses_refine_by_layer
        .insert(layer_id, plan.motion_suppresses_refine);
}

fn count_tile_levels(keys: impl Iterator<Item = TileKey>) -> BTreeMap<i32, u32> {
    let mut counts = BTreeMap::new();
    for key in keys {
        *counts.entry(key.z).or_default() += 1;
    }
    counts
}

fn expand_bounds(bounds: TileBounds, tileset: &LoadedTileset, margin_tiles: i32) -> TileBounds {
    if margin_tiles <= 0 {
        return bounds;
    }
    let Some(level) = tileset.level(bounds.z) else {
        return bounds;
    };
    TileBounds {
        min_tx: (bounds.min_tx - margin_tiles).max(level.min_x),
        max_tx: (bounds.max_tx + margin_tiles).min(level.max_x),
        min_ty: (bounds.min_ty - margin_tiles).max(level.min_y),
        max_ty: (bounds.max_ty + margin_tiles).min(level.max_y),
        z: bounds.z,
        map_version: bounds.map_version,
    }
}

fn is_pinned_coarse_level(
    z: i32,
    max_level: u8,
    pinned_coarse_levels: u8,
    coarse_pin_min_level: Option<i32>,
) -> bool {
    if let Some(min_level) = coarse_pin_min_level {
        if z >= min_level {
            return true;
        }
    }
    if pinned_coarse_levels == 0 {
        return false;
    }
    let max_level = i32::from(max_level);
    let first_pinned = max_level - i32::from(pinned_coarse_levels) + 1;
    z >= first_pinned
}

fn bounds_level0_rect(bounds: TileBounds) -> Option<Level0Rect> {
    let (min_x, _) = level0_span(bounds.min_tx, bounds.z)?;
    let (_, max_x) = level0_span(bounds.max_tx, bounds.z)?;
    let (min_y, _) = level0_span(bounds.min_ty, bounds.z)?;
    let (_, max_y) = level0_span(bounds.max_ty, bounds.z)?;
    Some(Level0Rect {
        min_x,
        max_x,
        min_y,
        max_y,
    })
}

pub(crate) fn tile_residency_class(key: TileKey, residency: &TileResidencyState) -> ResidencyClass {
    if residency.protected.contains(&key) {
        ResidencyClass::Protected
    } else if residency.warm.contains(&key) {
        ResidencyClass::Warm
    } else {
        ResidencyClass::Evictable
    }
}

pub(crate) fn eviction_priority_score(
    current_use_counter: u64,
    key: TileKey,
    entry: &RasterTileEntry,
    residency: &TileResidencyState,
    layer: Option<&LayerSpec>,
) -> f64 {
    let detail_rank = layer
        .map(|layer| (i32::from(layer.max_level) - key.z).max(0) as f64)
        .unwrap_or(0.0);
    let detail_weight = layer
        .map(|layer| layer.lod_policy.detail_eviction_weight as f64)
        .unwrap_or(1.0);
    let age_rank = current_use_counter.saturating_sub(entry.last_used) as f64;
    let distance_rank =
        desired_distance_rank(key, residency.desired_spans_by_layer.get(&key.layer)) as f64;
    let layer_weight = layer
        .map(|layer| layer.request_weight as f64)
        .unwrap_or(1.0);

    detail_rank * detail_weight * EVICT_DETAIL_WEIGHT
        + age_rank * EVICT_AGE_WEIGHT
        + distance_rank * EVICT_DISTANCE_WEIGHT
        - layer_weight * EVICT_LAYER_WEIGHT
}

fn desired_distance_rank(key: TileKey, desired: Option<&Vec<Level0Rect>>) -> i64 {
    let Some(key_rect) = key_level0_rect(key) else {
        return 0;
    };
    let Some(desired) = desired else {
        return 0;
    };
    desired
        .iter()
        .map(|rect| rect_distance2(key_rect, *rect))
        .min()
        .unwrap_or(0)
}

fn key_level0_rect(key: TileKey) -> Option<Level0Rect> {
    let (min_x, max_x) = level0_span(key.tx, key.z)?;
    let (min_y, max_y) = level0_span(key.ty, key.z)?;
    Some(Level0Rect {
        min_x,
        max_x,
        min_y,
        max_y,
    })
}

fn rect_distance2(lhs: Level0Rect, rhs: Level0Rect) -> i64 {
    let dx = interval_gap(lhs.min_x, lhs.max_x, rhs.min_x, rhs.max_x);
    let dy = interval_gap(lhs.min_y, lhs.max_y, rhs.min_y, rhs.max_y);
    dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
}

fn interval_gap(a0: i64, a1: i64, b0: i64, b1: i64) -> i64 {
    if a1 < b0 {
        b0 - a1
    } else if b1 < a0 {
        a0 - b1
    } else {
        0
    }
}

pub(crate) fn merge_level_counts(
    dst: &mut HashMap<LayerId, BTreeMap<i32, u32>>,
    layer_id: LayerId,
    src: &BTreeMap<i32, u32>,
) {
    if src.is_empty() {
        return;
    }
    let entry = dst.entry(layer_id).or_default();
    merge_level_count_maps(entry, src);
}

pub(crate) fn incr_level_count(
    dst: &mut HashMap<LayerId, BTreeMap<i32, u32>>,
    layer_id: LayerId,
    level: i32,
    value: u32,
) {
    let entry = dst.entry(layer_id).or_default();
    let counter = entry.entry(level).or_default();
    *counter = counter.saturating_add(value);
}

pub(crate) fn sum_level_counts(counts: &BTreeMap<i32, u32>) -> u32 {
    counts.values().copied().sum()
}

fn tile_overlaps_bounds(key: &TileKey, bounds: TileBounds) -> bool {
    if key.map_version != bounds.map_version {
        return false;
    }

    let Some((vx0, vx1)) = level0_span(bounds.min_tx, bounds.z)
        .and_then(|(min, _)| level0_span(bounds.max_tx, bounds.z).map(|(_, max)| (min, max)))
    else {
        return false;
    };
    let Some((vy0, vy1)) = level0_span(bounds.min_ty, bounds.z)
        .and_then(|(min, _)| level0_span(bounds.max_ty, bounds.z).map(|(_, max)| (min, max)))
    else {
        return false;
    };
    let Some((kx0, kx1)) = level0_span(key.tx, key.z) else {
        return false;
    };
    let Some((ky0, ky1)) = level0_span(key.ty, key.z) else {
        return false;
    };

    !(kx1 < vx0 || kx0 > vx1 || ky1 < vy0 || ky0 > vy1)
}

fn level0_span(tile: i32, z: i32) -> Option<(i64, i64)> {
    if z < 0 || z > 30 {
        return None;
    }
    let scale = 1_i64.checked_shl(z as u32)?;
    let min = (tile as i64).checked_mul(scale)?;
    let max = min.checked_add(scale - 1)?;
    Some((min, max))
}
