use crate::config::TILE_CACHE_MAX;
use crate::map::layers::{LayerRegistry, LayerRuntime, LayerRuntimeState, LayerSpec, LodPolicy};
use crate::map::spaces::layer_transform::{TileSpace, WorldTransform};
use crate::map::spaces::{WorldPoint, WorldRect};
use crate::plugins::input::PanState;

use super::super::manifest::{LevelInfo, LoadedTileset};
use super::{CameraMotionState, DesiredLayerTiles, TileBounds};

const MOTION_PAN_FRACTION_THRESHOLD: f64 = 0.08;
const MOTION_ZOOM_OUT_THRESHOLD: f64 = 0.06;
const MOTION_COOLDOWN_FRAMES: u32 = 16;

pub(crate) fn compute_desired_layer_tiles(
    layer: &LayerSpec,
    tileset: &LoadedTileset,
    world_transform: WorldTransform,
    view_world: WorldRect,
    map_version: u64,
    frame: u64,
    runtime: &mut LayerRuntimeState,
    previous: Option<DesiredLayerTiles>,
) -> DesiredLayerTiles {
    crate::perf_scope!("raster.visible_tile_computation");
    let layer_aabb = world_transform.world_rect_to_layer_aabb(view_world);

    let mut candidates: Vec<(TileBounds, usize)> = Vec::new();
    for level in &tileset.levels {
        if level.z < 0 || level.z as u8 > layer.max_level {
            continue;
        }
        let Some(bounds) = bounds_for_level(
            layer,
            layer_aabb,
            level,
            map_version,
            layer.lod_policy.margin_tiles,
        ) else {
            continue;
        };
        let count = level.count_in_rect(bounds.min_tx, bounds.max_tx, bounds.min_ty, bounds.max_ty);
        candidates.push((bounds, count));
    }
    candidates.sort_by_key(|(bounds, _)| bounds.z);

    let previous_base = previous.and_then(|state| state.base);
    let base = choose_bounds_with_hysteresis(&candidates, previous_base, &layer.lod_policy);
    runtime.current_base_lod = base.and_then(|value| u8::try_from(value.z).ok());

    if base != previous_base {
        runtime.last_view_update_frame = frame;
    }

    let mut detail = None;
    if layer.lod_policy.enable_refine {
        if let Some(base_bounds) = base {
            if base_bounds.z > 0 {
                let debounce_frames =
                    (layer.lod_policy.refine_debounce_ms.max(1) as u64).div_ceil(16);
                if frame.saturating_sub(runtime.last_view_update_frame) >= debounce_frames {
                    let detail_z = base_bounds.z - 1;
                    if let Some(level) = tileset.level(detail_z) {
                        if let Some(detail_bounds) =
                            bounds_for_level(layer, layer_aabb, level, map_version, 0)
                        {
                            let count = level.count_in_rect(
                                detail_bounds.min_tx,
                                detail_bounds.max_tx,
                                detail_bounds.min_ty,
                                detail_bounds.max_ty,
                            );
                            if count <= layer.lod_policy.max_detail_tiles {
                                detail = Some(detail_bounds);
                            }
                        }
                    }
                }
            }
        }
    }
    runtime.current_detail_lod = detail.and_then(|value| u8::try_from(value.z).ok());

    DesiredLayerTiles { base, detail }
}

fn bounds_for_level(
    layer: &LayerSpec,
    layer_aabb: crate::map::spaces::LayerRect,
    level: &LevelInfo,
    map_version: u64,
    margin_tiles: i32,
) -> Option<TileBounds> {
    let tile_span = TileSpace::new(layer.tile_px, layer.y_flip).tile_span_px(level.z)?;
    if tile_span <= 0.0 {
        return None;
    }

    let mut min_tx = (layer_aabb.min.x / tile_span).floor() as i32 - margin_tiles;
    let mut max_tx = (layer_aabb.max.x / tile_span).floor() as i32 + margin_tiles;
    let mut min_ty = (layer_aabb.min.y / tile_span).floor() as i32 - margin_tiles;
    let mut max_ty = (layer_aabb.max.y / tile_span).floor() as i32 + margin_tiles;

    min_tx = min_tx.max(level.min_x);
    max_tx = max_tx.min(level.max_x);
    min_ty = min_ty.max(level.min_y);
    max_ty = max_ty.min(level.max_y);

    if min_tx > max_tx || min_ty > max_ty {
        return None;
    }

    Some(TileBounds {
        min_tx,
        max_tx,
        min_ty,
        max_ty,
        z: level.z,
        map_version,
    })
}

fn choose_bounds_with_hysteresis(
    candidates: &[(TileBounds, usize)],
    current: Option<TileBounds>,
    policy: &LodPolicy,
) -> Option<TileBounds> {
    if candidates.is_empty() {
        return None;
    }

    let target = policy.target_tiles.max(1);
    let hi = policy.hysteresis_hi.max(target as f32) as usize;
    let lo = policy.hysteresis_lo.min(target as f32) as usize;

    let ideal = candidates
        .iter()
        .find(|(_, count)| *count <= target)
        .copied()
        .unwrap_or_else(|| *candidates.last().expect("non-empty"));

    let Some(current) = current else {
        return Some(ideal.0);
    };

    let Some((current_bounds, current_count)) = candidates
        .iter()
        .find(|(bounds, _)| bounds.z == current.z)
        .copied()
    else {
        return Some(ideal.0);
    };

    if current_count > hi {
        let choice = candidates
            .iter()
            .filter(|(bounds, _)| bounds.z >= current.z)
            .find(|(_, count)| *count <= target)
            .copied()
            .or_else(|| {
                candidates
                    .iter()
                    .rfind(|(bounds, _)| bounds.z >= current.z)
                    .copied()
            })
            .unwrap_or(ideal);
        return Some(choice.0);
    }

    if current_count < lo {
        let finer = candidates
            .iter()
            .filter(|(bounds, _)| bounds.z <= current.z)
            .find(|(_, count)| *count <= target)
            .copied();
        if let Some((bounds, _)) = finer {
            return Some(bounds);
        }
    }

    Some(current_bounds)
}

pub(crate) fn desired_change_is_minor(
    previous: Option<DesiredLayerTiles>,
    desired: DesiredLayerTiles,
    policy: &LodPolicy,
) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    let base_tolerance = policy.protected_margin_tiles.max(1);
    let detail_tolerance = policy.warm_margin_tiles.max(1);
    bounds_shift_is_minor(previous.base, desired.base, base_tolerance)
        && bounds_shift_is_minor(previous.detail, desired.detail, detail_tolerance)
}

pub(crate) fn lod_signature(
    desired: DesiredLayerTiles,
) -> (Option<(i32, u64)>, Option<(i32, u64)>) {
    (
        desired.base.map(|bounds| (bounds.z, bounds.map_version)),
        desired.detail.map(|bounds| (bounds.z, bounds.map_version)),
    )
}

fn bounds_shift_is_minor(
    previous: Option<TileBounds>,
    next: Option<TileBounds>,
    tolerance: i32,
) -> bool {
    match (previous, next) {
        (None, None) => true,
        (Some(a), Some(b)) if a.z == b.z && a.map_version == b.map_version => {
            let dx = (a.min_tx - b.min_tx).abs().max((a.max_tx - b.max_tx).abs());
            let dy = (a.min_ty - b.min_ty).abs().max((a.max_ty - b.max_ty).abs());
            dx <= tolerance && dy <= tolerance
        }
        _ => false,
    }
}

pub(crate) fn update_camera_motion_state(
    motion: &mut CameraMotionState,
    view_world: WorldRect,
    pan_state: &PanState,
) {
    let center = WorldPoint::new(
        (view_world.min.x + view_world.max.x) * 0.5,
        (view_world.min.z + view_world.max.z) * 0.5,
    );
    let span_x = (view_world.max.x - view_world.min.x).abs();
    let span_y = (view_world.max.z - view_world.min.z).abs();
    let diag = (span_x * span_x + span_y * span_y).sqrt().max(1.0);

    let mut pan_fraction = 0.0;
    if let Some(prev_center) = motion.prev_center {
        let dx = center.x - prev_center.x;
        let dy = center.z - prev_center.z;
        let pan_dist = (dx * dx + dy * dy).sqrt();
        pan_fraction = pan_dist / diag;
    }

    let mut zoom_out_ratio = 0.0;
    if let Some(prev_diag) = motion.prev_diag {
        if prev_diag > 0.0 {
            zoom_out_ratio = (diag / prev_diag - 1.0).max(0.0);
        }
    }

    let moving_now = pan_state.dragging
        || pan_fraction >= MOTION_PAN_FRACTION_THRESHOLD
        || zoom_out_ratio >= MOTION_ZOOM_OUT_THRESHOLD;
    if moving_now {
        motion.unstable_frames_left = MOTION_COOLDOWN_FRAMES;
    } else if motion.unstable_frames_left > 0 {
        motion.unstable_frames_left -= 1;
    }
    motion.unstable = motion.unstable_frames_left > 0;
    motion.pan_fraction = pan_fraction;
    motion.zoom_out_ratio = zoom_out_ratio;
    motion.prev_center = Some(center);
    motion.prev_diag = Some(diag);
}

pub(crate) fn compute_cache_budget(
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> usize {
    let sum = layer_registry
        .ordered()
        .iter()
        .filter(|layer| layer.is_raster() && layer_runtime.visible(layer.id))
        .map(|layer| layer.lod_policy.max_resident_tiles)
        .sum::<usize>();
    if sum == 0 {
        TILE_CACHE_MAX
    } else {
        sum.clamp(256, 16_384)
    }
}
