use std::collections::{BTreeMap, HashMap, HashSet};

use bevy::asset::AssetServer;
use bevy::prelude::Time;

use crate::map::layers::{LayerId, LayerManifestStatus, LayerRegistry, LayerRuntime, LayerSpec};
use crate::map::streaming::{RequestKind, TileRequest, TileStreamer};

use super::super::cache::RasterTileCache;
use super::super::manifest::{layer_tile_url, LoadedTileset};
use super::super::TileKey;
use super::{merge_level_count_maps, DesiredLayerTiles, TileBounds, TileResidencyState};

pub(crate) struct BuildResult {
    pub(crate) requests: Vec<TileRequest>,
    pub(crate) cache_hits: u32,
    pub(crate) cache_hits_by_level: BTreeMap<i32, u32>,
    pub(crate) cache_misses_by_level: BTreeMap<i32, u32>,
    pub(crate) detail_queued: u32,
    pub(crate) coverage_queued: u32,
}

pub(crate) struct LayerRequestBuild<'a> {
    pub(crate) layer: &'a LayerSpec,
    pub(crate) tileset: &'a LoadedTileset,
    pub(crate) desired: DesiredLayerTiles,
    pub(crate) map_version: Option<&'a str>,
    pub(crate) cache: &'a RasterTileCache,
    pub(crate) map_version_id: u64,
    pub(crate) camera_unstable: bool,
    pub(crate) residency: &'a TileResidencyState,
}

struct BoundsRequestBuild<'a> {
    layer: &'a LayerSpec,
    tileset: &'a LoadedTileset,
    bounds: TileBounds,
    map_version: Option<&'a str>,
    cache: &'a RasterTileCache,
    kind: RequestKind,
    request_weight: f32,
    map_version_id: u64,
}

pub(crate) struct StartTileRequests<'a> {
    pub(crate) streamer: &'a mut TileStreamer,
    pub(crate) cache: &'a mut RasterTileCache,
    pub(crate) asset_server: &'a AssetServer,
    pub(crate) layer_registry: &'a LayerRegistry,
    pub(crate) layer_runtime: &'a LayerRuntime,
    pub(crate) residency: &'a TileResidencyState,
    pub(crate) camera_unstable: bool,
    pub(crate) stats: &'a mut crate::map::raster::TileStats,
}

pub(crate) fn build_layer_requests(input: LayerRequestBuild<'_>) -> BuildResult {
    crate::perf_scope!("raster.request_scheduling");
    let LayerRequestBuild {
        layer,
        tileset,
        desired,
        map_version,
        cache,
        map_version_id,
        camera_unstable,
        residency,
    } = input;
    let mut requests = Vec::new();
    let mut cache_hits: u32 = 0;
    let mut cache_hits_by_level = BTreeMap::new();
    let mut cache_misses_by_level = BTreeMap::new();
    let mut detail_queued = 0_u32;
    let mut coverage_queued = 0_u32;

    if let Some(base) = desired.base {
        let BuildResult {
            requests: mut base_requests,
            cache_hits: base_hits,
            cache_hits_by_level: base_hits_by_level,
            cache_misses_by_level: base_misses_by_level,
            detail_queued: _,
            coverage_queued: _,
        } = build_requests_for_bounds(BoundsRequestBuild {
            layer,
            tileset,
            bounds: base,
            map_version,
            cache,
            kind: RequestKind::BaseCoverage,
            request_weight: layer.request_weight,
            map_version_id,
        });
        coverage_queued = coverage_queued.saturating_add(base_requests.len() as u32);
        requests.append(&mut base_requests);
        cache_hits = cache_hits.saturating_add(base_hits);
        merge_level_count_maps(&mut cache_hits_by_level, &base_hits_by_level);
        merge_level_count_maps(&mut cache_misses_by_level, &base_misses_by_level);
    }

    let mut ancestor_keys: Vec<TileKey> = residency
        .ancestor_requests
        .iter()
        .filter(|key| key.layer == layer.id && key.map_version == map_version_id)
        .copied()
        .collect();
    ancestor_keys.sort_by_key(|key| (key.z, key.ty, key.tx));
    for key in ancestor_keys {
        if cache.contains(&key) {
            cache_hits = cache_hits.saturating_add(1);
            *cache_hits_by_level.entry(key.z).or_default() += 1;
            continue;
        }
        coverage_queued = coverage_queued.saturating_add(1);
        *cache_misses_by_level.entry(key.z).or_default() += 1;
        requests.push(TileRequest {
            key,
            url: layer_tile_url(layer, map_version, key.z, key.tx, key.ty),
            priority: request_priority(RequestKind::BaseCoverage, 0, layer.request_weight * 2.0),
            kind: RequestKind::BaseCoverage,
        });
    }

    let suppress_detail = camera_unstable
        && layer.lod_policy.motion_suppresses_refine
        && residency
            .blank_visible_by_layer
            .get(&layer.id)
            .copied()
            .unwrap_or(0)
            > 0;
    if !suppress_detail {
        if let Some(detail) = desired.detail {
            let BuildResult {
                requests: mut detail_requests,
                cache_hits: detail_hits,
                cache_hits_by_level: detail_hits_by_level,
                cache_misses_by_level: detail_misses_by_level,
                detail_queued: _,
                coverage_queued: _,
            } = build_requests_for_bounds(BoundsRequestBuild {
                layer,
                tileset,
                bounds: detail,
                map_version,
                cache,
                kind: RequestKind::DetailRefine,
                request_weight: layer.request_weight * 0.75,
                map_version_id,
            });
            if camera_unstable {
                let max_detail = residency
                    .max_detail_requests_while_moving_by_layer
                    .get(&layer.id)
                    .copied()
                    .unwrap_or(layer.lod_policy.max_detail_requests_while_camera_moving);
                if detail_requests.len() > max_detail {
                    detail_requests.truncate(max_detail);
                }
            }
            detail_queued = detail_queued.saturating_add(detail_requests.len() as u32);
            requests.append(&mut detail_requests);
            cache_hits = cache_hits.saturating_add(detail_hits);
            merge_level_count_maps(&mut cache_hits_by_level, &detail_hits_by_level);
            merge_level_count_maps(&mut cache_misses_by_level, &detail_misses_by_level);
        }
    } else if let Some(detail) = desired.detail {
        let BuildResult {
            requests: _,
            cache_hits: detail_hits,
            cache_hits_by_level: detail_hits_by_level,
            cache_misses_by_level: detail_misses_by_level,
            detail_queued: _,
            coverage_queued: _,
        } = build_requests_for_bounds(BoundsRequestBuild {
            layer,
            tileset,
            bounds: detail,
            map_version,
            cache,
            kind: RequestKind::DetailRefine,
            request_weight: layer.request_weight * 0.75,
            map_version_id,
        });
        cache_hits = cache_hits.saturating_add(detail_hits);
        merge_level_count_maps(&mut cache_hits_by_level, &detail_hits_by_level);
        merge_level_count_maps(&mut cache_misses_by_level, &detail_misses_by_level);
    }

    dedupe_requests(&mut requests);
    crate::perf_counter_add!("raster.cache_hits", cache_hits);
    crate::perf_counter_add!(
        "raster.cache_misses",
        cache_misses_by_level.values().copied().sum::<u32>()
    );
    BuildResult {
        requests,
        cache_hits,
        cache_hits_by_level,
        cache_misses_by_level,
        detail_queued,
        coverage_queued,
    }
}

fn build_requests_for_bounds(input: BoundsRequestBuild<'_>) -> BuildResult {
    crate::perf_scope!("raster.cache_lookup_update");
    let BoundsRequestBuild {
        layer,
        tileset,
        bounds,
        map_version,
        cache,
        kind,
        request_weight,
        map_version_id,
    } = input;
    let Some(level) = tileset.level(bounds.z) else {
        return BuildResult {
            requests: Vec::new(),
            cache_hits: 0,
            cache_hits_by_level: BTreeMap::new(),
            cache_misses_by_level: BTreeMap::new(),
            detail_queued: 0,
            coverage_queued: 0,
        };
    };

    let center_tx = (bounds.min_tx + bounds.max_tx) / 2;
    let center_ty = (bounds.min_ty + bounds.max_ty) / 2;

    let mut requests = Vec::new();
    let mut cache_hits = 0;
    let mut misses = 0_u32;
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
            if cache.contains(&key) {
                cache_hits += 1;
                continue;
            }
            misses = misses.saturating_add(1);
            let distance2 = tile_distance2(tx, ty, center_tx, center_ty);
            let priority = request_priority(kind, distance2, request_weight);
            requests.push(TileRequest {
                key,
                url: layer_tile_url(layer, map_version, bounds.z, tx, ty),
                priority,
                kind,
            });
        }
    }

    let mut cache_hits_by_level = BTreeMap::new();
    if cache_hits > 0 {
        cache_hits_by_level.insert(bounds.z, cache_hits);
    }
    let mut cache_misses_by_level = BTreeMap::new();
    if misses > 0 {
        cache_misses_by_level.insert(bounds.z, misses);
    }

    BuildResult {
        requests,
        cache_hits,
        cache_hits_by_level,
        cache_misses_by_level,
        detail_queued: 0,
        coverage_queued: 0,
    }
}

fn request_priority(kind: RequestKind, distance2: i64, request_weight: f32) -> f32 {
    let kind_bias = match kind {
        RequestKind::BaseCoverage => 1_000_000.0,
        RequestKind::DetailRefine => 2_000_000.0,
    };
    kind_bias + (distance2 as f32) / request_weight.max(0.05)
}

fn dedupe_requests(requests: &mut Vec<TileRequest>) {
    let mut seen = HashSet::with_capacity(requests.len());
    requests.retain(|req| seen.insert(req.key));
}

pub(crate) fn start_tile_requests(input: StartTileRequests<'_>) {
    crate::perf_scope!("raster.request_start");
    let StartTileRequests {
        streamer,
        cache,
        asset_server,
        layer_registry,
        layer_runtime,
        residency,
        camera_unstable,
        stats,
    } = input;
    let mut started = 0;
    let mut detail_started_by_layer: HashMap<LayerId, usize> = HashMap::new();
    while started < streamer.max_new_requests_per_frame && streamer.inflight < streamer.max_inflight
    {
        let Some(req) = streamer.next_request() else {
            break;
        };
        if cache.contains(&req.key) {
            continue;
        }
        let Some(layer) = layer_registry.get(req.key.layer) else {
            continue;
        };
        let motion_suppresses = residency
            .motion_suppresses_refine_by_layer
            .get(&layer.id)
            .copied()
            .unwrap_or(layer.lod_policy.motion_suppresses_refine);
        if camera_unstable && req.kind == RequestKind::DetailRefine && motion_suppresses {
            let max_detail = residency
                .max_detail_requests_while_moving_by_layer
                .get(&layer.id)
                .copied()
                .unwrap_or(layer.lod_policy.max_detail_requests_while_camera_moving);
            let started_for_layer = detail_started_by_layer.entry(layer.id).or_default();
            if *started_for_layer >= max_detail {
                stats.requests_suppressed_motion =
                    stats.requests_suppressed_motion.saturating_add(1);
                continue;
            }
            *started_for_layer += 1;
        }
        let alpha = layer_runtime
            .get(layer.id)
            .map(|state| state.opacity)
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);
        let visible = tile_should_render(&req.key, layer_runtime, residency);
        let handle = asset_server.load(req.url);
        cache.insert_loading(req.key, handle, visible, alpha);
        stats.requested_tiles = stats.requested_tiles.saturating_add(1);
        match req.kind {
            RequestKind::BaseCoverage => {
                stats.coverage_requests_started = stats.coverage_requests_started.saturating_add(1);
            }
            RequestKind::DetailRefine => {
                stats.detail_requests_started = stats.detail_requests_started.saturating_add(1);
            }
        }
        streamer.inflight += 1;
        stats.inflight = streamer.inflight;
        started += 1;
    }
    crate::perf_counter_add!("raster.requests_started", started);
}

pub(crate) fn tile_should_render(
    key: &TileKey,
    layer_runtime: &LayerRuntime,
    residency: &TileResidencyState,
) -> bool {
    layer_runtime.visible(key.layer)
        && layer_runtime
            .get(key.layer)
            .map(|state| state.manifest_status == LayerManifestStatus::Ready)
            .unwrap_or(false)
        && residency.render_visible.contains(key)
}

fn tile_distance2(tx: i32, ty: i32, center_tx: i32, center_ty: i32) -> i64 {
    let dx = tx - center_tx;
    let dy = ty - center_ty;
    (dx as i64 * dx as i64) + (dy as i64 * dy as i64)
}

pub(crate) fn log_tile_stats(stats: &mut crate::map::raster::TileStats, time: &Time) {
    let now = time.elapsed_secs_f64();
    if now - stats.last_log >= 1.0 {
        stats.last_log = now;
        stats.requested_tiles = 0;
        stats.cache_hits = 0;
        stats.cache_evictions = 0;
        stats.cache_hits_by_level.clear();
        stats.cache_misses_by_level.clear();
        stats.cache_evictions_by_level.clear();
        stats.requests_suppressed_motion = 0;
        stats.detail_requests_started = 0;
        stats.coverage_requests_started = 0;
    }
}
