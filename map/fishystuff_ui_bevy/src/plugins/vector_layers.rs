use std::collections::{HashMap, HashSet};

use async_channel::TryRecvError;
use bevy::ecs::system::SystemParam;
use bevy::platform::time::Instant;
use bevy::prelude::*;
use bevy::window::RequestRedraw;

use crate::config::{
    VECTOR_FINISHED_CACHE_MAX, VECTOR_GLOBAL_BUILD_BUDGET_MS, VECTOR_MAX_BUILD_MS_PER_FRAME,
    VECTOR_MAX_CHUNK_TRIANGLES, VECTOR_MAX_CHUNK_VERTICES, VECTOR_MAX_FEATURES_PER_FRAME,
};
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::layers::{
    LayerId, LayerRegistry, LayerRuntime, LayerRuntimeState, LayerVectorStatus, VectorSourceSpec,
};
use crate::map::raster::cache::{clip_mask_allows_world_point, clip_mask_state_revision};
use crate::map::raster::RasterTileCache;
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::WorldPoint;
use crate::map::vector::build::{
    advance_job, begin_fetch, finalize_job, parse_into_job, revision_matches, state_revision,
    state_stats, state_status, AdvanceResult, VectorBuildLimits, VectorBuildState,
};
use crate::map::vector::cache::{
    BuiltVectorChunk, BuiltVectorGeometry, VectorFinishedCache, VectorLayerStats,
};
use crate::map::vector::render::{spawn_vector_meshes, VECTOR_3D_BASE_Y};
use crate::plugins::api::{LayerEffectiveFilterState, ZoneMembershipFilter};
use crate::plugins::points::EvidenceZoneFilter;

const VECTOR_MIN_PROGRESS_BUDGET_MS: f64 = 0.25;

#[derive(Resource, Debug, Clone, Copy)]
pub struct VectorBuildConfig {
    pub global_build_budget_ms: f64,
    pub max_features_per_frame: usize,
    pub max_build_ms_per_frame: f64,
    pub max_chunk_vertices: usize,
    pub max_chunk_triangles: usize,
}

impl Default for VectorBuildConfig {
    fn default() -> Self {
        Self {
            global_build_budget_ms: VECTOR_GLOBAL_BUILD_BUDGET_MS,
            max_features_per_frame: VECTOR_MAX_FEATURES_PER_FRAME,
            max_build_ms_per_frame: VECTOR_MAX_BUILD_MS_PER_FRAME,
            max_chunk_vertices: VECTOR_MAX_CHUNK_VERTICES,
            max_chunk_triangles: VECTOR_MAX_CHUNK_TRIANGLES,
        }
    }
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct VectorCacheConfig {
    pub max_entries: usize,
}

impl Default for VectorCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: VECTOR_FINISHED_CACHE_MAX,
        }
    }
}

#[derive(Resource, Default)]
pub struct VectorLayerRuntime {
    pub states: HashMap<LayerId, VectorBuildState>,
    pub finished: VectorFinishedCache,
}

impl VectorLayerRuntime {
    fn with_defaults() -> Self {
        Self {
            states: HashMap::new(),
            finished: VectorFinishedCache::with_capacity(VECTOR_FINISHED_CACHE_MAX),
        }
    }

    fn has_pending_work(&self) -> bool {
        self.states
            .values()
            .any(vector_build_state_needs_frame_updates)
    }
}

pub struct VectorLayersPlugin;

impl Plugin for VectorLayersPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(VectorLayerRuntime::with_defaults())
            .init_resource::<VectorBuildConfig>()
            .init_resource::<VectorCacheConfig>()
            .add_systems(
                Update,
                update_vector_layers.run_if(vector_layers_need_update),
            )
            .add_systems(
                Update,
                request_redraw_while_vector_pending.after(update_vector_layers),
            );
    }
}

fn vector_layers_need_update(
    registry: Res<'_, LayerRegistry>,
    layer_runtime: Res<'_, LayerRuntime>,
    vector_runtime: Res<'_, VectorLayerRuntime>,
    cache_config: Res<'_, VectorCacheConfig>,
    view_mode: Res<'_, ViewModeState>,
) -> bool {
    registry.is_changed()
        || layer_runtime.is_changed()
        || cache_config.is_changed()
        || view_mode.is_changed()
        || vector_runtime.has_pending_work()
}

fn update_vector_layers(mut commands: Commands, mut update: VectorLayerUpdate<'_, '_>) {
    let meshes = &mut update.meshes;
    let materials_2d = &mut update.materials_2d;
    let materials_3d = &mut update.materials_3d;
    let registry = &update.registry;
    let layer_runtime = &mut update.layer_runtime;
    let vector_runtime = &mut update.vector_runtime;
    let build_config = &update.build_config;
    let cache_config = &update.cache_config;
    let view_mode = &update.view_mode;
    layer_runtime.sync_to_registry(registry);

    if registry.is_changed() {
        prune_stale_runtime_data(registry, vector_runtime, &mut commands);
    }

    vector_runtime
        .finished
        .set_max_entries(cache_config.max_entries.max(1));

    let clip_mask_source_ids = layer_runtime.clip_mask_source_ids();
    let has_visible_vector_inputs = registry.ordered().iter().any(|layer| {
        layer.is_vector()
            && should_activate_vector_layer(
                layer,
                layer_runtime.visible(layer.id),
                clip_mask_source_ids.contains(&layer.id),
                view_mode.mode,
            )
    });
    if !has_visible_vector_inputs
        && vector_runtime.states.is_empty()
        && vector_runtime.finished.is_empty()
    {
        for layer in registry.ordered() {
            if !layer.is_vector() {
                continue;
            }
            if let Some(runtime_state) = layer_runtime.get_mut(layer.id) {
                clear_vector_build_metrics(runtime_state, LayerVectorStatus::NotRequested);
            }
        }
        return;
    }

    crate::perf_scope!("vector.layer_update");
    let map_to_world = MapToWorld::default();
    let map_version_id = registry.map_version_id();
    let frame_start = Instant::now();
    let active_by_layer: HashMap<LayerId, bool> = registry
        .ordered()
        .iter()
        .map(|layer| {
            let state = layer_runtime
                .get(layer.id)
                .expect("runtime synced to registry before vector update");
            (
                layer.id,
                should_activate_vector_layer(
                    layer,
                    state.visible,
                    clip_mask_source_ids.contains(&layer.id),
                    view_mode.mode,
                ),
            )
        })
        .collect();
    let visible_cache_keys = collect_visible_cache_keys(
        registry,
        layer_runtime,
        &active_by_layer,
        map_version_id,
        &update.layer_filters,
    );
    for evicted in vector_runtime
        .finished
        .evict_lru_non_visible(|key| visible_cache_keys.contains(key))
    {
        evicted.despawn(&mut commands);
    }

    for layer in registry.ordered() {
        let inactive_filter = EvidenceZoneFilter::default();
        let zone_filter = update
            .layer_filters
            .zone_membership_filter(layer.key.as_str())
            .unwrap_or(&inactive_filter);
        let clip_mask_layer_id = layer_runtime.clip_mask_layer(layer.id);
        let clip_mask_revision =
            clip_mask_state_revision(registry, layer_runtime, clip_mask_layer_id, zone_filter);
        let Some(mut runtime_state) = layer_runtime.get(layer.id) else {
            continue;
        };
        let active = active_by_layer.get(&layer.id).copied().unwrap_or(false);
        let render_visible =
            should_render_vector_layer(layer, runtime_state.visible, view_mode.mode);
        let previous_status = runtime_state.vector_status;

        if !layer.is_vector() {
            clear_vector_metrics(&mut runtime_state, LayerVectorStatus::Inactive);
            if let Some(slot) = layer_runtime.get_mut(layer.id) {
                *slot = runtime_state;
            }
            continue;
        }

        runtime_state.current_base_lod = None;
        runtime_state.current_detail_lod = None;
        runtime_state.visible_tile_count = 0;
        runtime_state.resident_tile_count = 0;
        runtime_state.pending_count = 0;
        runtime_state.inflight_count = 0;

        let Some(source_ref) = layer.vector_source.as_ref() else {
            clear_vector_metrics(&mut runtime_state, LayerVectorStatus::Failed);
            if let Some(slot) = layer_runtime.get_mut(layer.id) {
                *slot = runtime_state;
            }
            continue;
        };
        if !active
            && !vector_runtime.states.contains_key(&layer.id)
            && vector_runtime.finished.layer_len(layer.id) == 0
        {
            clear_vector_build_metrics(&mut runtime_state, LayerVectorStatus::NotRequested);
            if let Some(slot) = layer_runtime.get_mut(layer.id) {
                *slot = runtime_state;
            }
            continue;
        }
        let source = resolve_vector_source_for_map_version(source_ref, map_version_id);
        let selected_field_ids = update
            .layer_filters
            .semantic_field_ids_for_layer(layer.key.as_str())
            .to_vec();
        let revision = effective_revision(&source, &selected_field_ids, clip_mask_revision);
        invalidate_non_active_revisions(vector_runtime, layer.id, &revision, &mut commands);

        hide_non_active_finished(
            vector_runtime,
            layer.id,
            &revision,
            FinishedVisibilityContext {
                commands: &mut commands,
                materials_2d,
                materials_3d,
                z_base: runtime_state.z_base,
                opacity: runtime_state.opacity,
                visible: render_visible,
            },
        );

        let cache_key = (layer.id, revision.clone());
        if let Some(bundle) = vector_runtime.finished.get(&cache_key) {
            runtime_state.vector_cache_last_hit = true;
            runtime_state.vector_cache_hits = runtime_state.vector_cache_hits.saturating_add(1);
            crate::perf_counter_add!("vector.cache_hits", 1);
            bundle.set_depth(&mut commands, runtime_state.z_base, VECTOR_3D_BASE_Y);
            bundle.set_visibility(&mut commands, render_visible);
            bundle.set_opacity(materials_2d, materials_3d, runtime_state.opacity);
            apply_stats(&mut runtime_state, bundle.stats, LayerVectorStatus::Ready);
            runtime_state.vector_cache_entries = vector_runtime.finished.layer_len(layer.id) as u32;
            vector_runtime
                .states
                .insert(layer.id, VectorBuildState::Ready { revision });
            if let Some(slot) = layer_runtime.get_mut(layer.id) {
                *slot = runtime_state;
            }
            continue;
        }

        runtime_state.vector_cache_last_hit = false;

        let mut state = vector_runtime
            .states
            .remove(&layer.id)
            .unwrap_or(VectorBuildState::NotRequested);
        if !revision_matches(&state, &revision) {
            state = VectorBuildState::NotRequested;
        }

        if !active {
            if let Some(stats) = state_stats(&state) {
                apply_stats(&mut runtime_state, stats, state_status(&state));
            } else {
                runtime_state.vector_status = LayerVectorStatus::NotRequested;
                runtime_state.vector_progress = 0.0;
                runtime_state.vector_last_frame_build_ms = 0.0;
            }
            runtime_state.vector_cache_entries = vector_runtime.finished.layer_len(layer.id) as u32;
            vector_runtime.states.insert(layer.id, state);
            if let Some(slot) = layer_runtime.get_mut(layer.id) {
                *slot = runtime_state;
            }
            continue;
        }

        runtime_state.vector_cache_misses = runtime_state.vector_cache_misses.saturating_add(1);
        crate::perf_counter_add!("vector.cache_misses", 1);

        state = match state {
            VectorBuildState::NotRequested => begin_fetch(source.clone(), revision.clone()),
            VectorBuildState::Fetching {
                source,
                revision,
                url,
                receiver,
                started_at,
            } => match receiver.try_recv() {
                Ok(result) => match result {
                    Ok(bytes) => VectorBuildState::Parsing {
                        source,
                        revision,
                        bytes,
                        started_at,
                    },
                    Err(err) => VectorBuildState::Failed {
                        revision,
                        error: format!("fetch {} failed: {}", url, err),
                    },
                },
                Err(TryRecvError::Empty) => VectorBuildState::Fetching {
                    source,
                    revision,
                    url,
                    receiver,
                    started_at,
                },
                Err(TryRecvError::Closed) => VectorBuildState::Failed {
                    revision,
                    error: format!("fetch {} failed: request channel closed", url),
                },
            },
            VectorBuildState::Parsing {
                source,
                revision,
                bytes,
                ..
            } => match parse_into_job(source, revision.clone(), bytes, &selected_field_ids) {
                Ok(job) => VectorBuildState::Building { job },
                Err(err) => VectorBuildState::Failed {
                    revision,
                    error: err,
                },
            },
            VectorBuildState::Building { mut job } => {
                let spent_ms = frame_start.elapsed().as_secs_f64() * 1000.0;
                let global_remaining_ms = (build_config.global_build_budget_ms - spent_ms).max(0.0);
                let limits = VectorBuildLimits {
                    max_features_per_frame: build_config.max_features_per_frame.max(1),
                    max_build_ms_per_frame: effective_build_budget_ms(
                        build_config.max_build_ms_per_frame,
                        global_remaining_ms,
                    ),
                    max_chunk_vertices: build_config.max_chunk_vertices.max(1),
                    max_chunk_triangles: build_config.max_chunk_triangles.max(1),
                };

                if limits.max_build_ms_per_frame <= 0.0 {
                    job.stats.last_frame_build_ms = 0.0;
                    VectorBuildState::Building { job }
                } else {
                    match advance_job(&mut job, map_to_world, limits) {
                        Ok(AdvanceResult::InProgress) => VectorBuildState::Building { job },
                        Ok(AdvanceResult::Complete) => {
                            let revision = job.revision().to_string();
                            let geometry = maybe_clip_built_geometry(
                                finalize_job(job, limits),
                                clip_mask_layer_id,
                                registry,
                                layer_runtime,
                                &update.exact_lookups,
                                &update.raster_cache,
                                vector_runtime,
                                zone_filter,
                            );
                            let bundle = spawn_vector_meshes(
                                &mut commands,
                                meshes,
                                materials_2d,
                                materials_3d,
                                geometry,
                                runtime_state.z_base,
                                runtime_state.opacity,
                            );
                            bundle.set_depth(&mut commands, runtime_state.z_base, VECTOR_3D_BASE_Y);
                            bundle.set_visibility(&mut commands, render_visible);
                            bundle.set_opacity(materials_2d, materials_3d, runtime_state.opacity);
                            apply_stats(&mut runtime_state, bundle.stats, LayerVectorStatus::Ready);
                            if let Some(replaced) = vector_runtime
                                .finished
                                .insert((layer.id, revision.clone()), bundle)
                            {
                                replaced.despawn(&mut commands);
                            }
                            for evicted in vector_runtime
                                .finished
                                .evict_lru_non_visible(|key| visible_cache_keys.contains(key))
                            {
                                evicted.despawn(&mut commands);
                            }
                            VectorBuildState::Ready { revision }
                        }
                        Err(err) => {
                            let revision = job.revision().to_string();
                            VectorBuildState::Failed {
                                revision,
                                error: err,
                            }
                        }
                    }
                }
            }
            other => other,
        };

        if let Some(stats) = state_stats(&state) {
            apply_stats(&mut runtime_state, stats, state_status(&state));
        } else {
            runtime_state.vector_status = state_status(&state);
            if runtime_state.vector_status == LayerVectorStatus::Ready {
                runtime_state.vector_progress = 1.0;
            }
        }
        if let VectorBuildState::Failed { error, .. } = &state {
            if previous_status != LayerVectorStatus::Failed {
                bevy::log::warn!("vector layer {} failed: {}", layer.key, error);
            }
        }
        runtime_state.vector_cache_entries = vector_runtime.finished.layer_len(layer.id) as u32;

        vector_runtime.states.insert(layer.id, state);
        if let Some(slot) = layer_runtime.get_mut(layer.id) {
            *slot = runtime_state;
        }
    }
}

fn request_redraw_while_vector_pending(
    vector_runtime: Res<'_, VectorLayerRuntime>,
    mut request_redraw: MessageWriter<'_, RequestRedraw>,
) {
    if vector_runtime.has_pending_work() {
        request_redraw.write(RequestRedraw);
    }
}

fn effective_build_budget_ms(max_build_ms_per_frame: f64, global_remaining_ms: f64) -> f64 {
    let capped_max = max_build_ms_per_frame.max(0.0);
    if capped_max <= 0.0 {
        return 0.0;
    }
    let remaining = global_remaining_ms.max(0.0);
    if remaining > 0.0 {
        return capped_max.min(remaining);
    }
    capped_max.min(VECTOR_MIN_PROGRESS_BUDGET_MS)
}

fn should_activate_vector_layer(
    _layer: &crate::map::layers::LayerSpec,
    layer_visible: bool,
    required_for_clip_mask: bool,
    view_mode: ViewMode,
) -> bool {
    if !matches!(view_mode, ViewMode::Map2D | ViewMode::Terrain3D) {
        return false;
    }
    if required_for_clip_mask {
        return true;
    }
    if !layer_visible {
        return false;
    }
    true
}

fn should_render_vector_layer(
    layer: &crate::map::layers::LayerSpec,
    layer_visible: bool,
    view_mode: ViewMode,
) -> bool {
    let _ = layer;
    layer_visible && matches!(view_mode, ViewMode::Map2D | ViewMode::Terrain3D)
}

fn maybe_clip_built_geometry(
    geometry: BuiltVectorGeometry,
    clip_mask_layer_id: Option<LayerId>,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
    exact_lookups: &ExactLookupCache,
    raster_cache: &RasterTileCache,
    vector_runtime: &VectorLayerRuntime,
    zone_filter: &ZoneMembershipFilter,
) -> BuiltVectorGeometry {
    let Some(mask_layer_id) = clip_mask_layer_id else {
        return geometry;
    };

    let mut chunks = Vec::with_capacity(geometry.chunks.len());
    for chunk in geometry.chunks {
        let Some(clipped_chunk) = clip_vector_chunk_against_mask(
            chunk,
            mask_layer_id,
            layer_registry,
            layer_runtime,
            exact_lookups,
            raster_cache,
            vector_runtime,
            zone_filter,
        ) else {
            continue;
        };
        chunks.push(clipped_chunk);
    }

    let vertex_count = chunks
        .iter()
        .map(|chunk| chunk.positions.len() as u32)
        .sum::<u32>();
    let triangle_count = chunks
        .iter()
        .map(|chunk| (chunk.indices.len() / 3) as u32)
        .sum::<u32>();
    let mesh_count = chunks.len() as u32;
    let chunked_bucket_count = if chunks.len() > 1 {
        chunks.len() as u32
    } else {
        0
    };

    BuiltVectorGeometry {
        chunks,
        hover_features: geometry.hover_features,
        stats: VectorLayerStats {
            vertex_count,
            triangle_count,
            mesh_count,
            chunked_bucket_count,
            ..geometry.stats
        },
    }
}

fn clip_vector_chunk_against_mask(
    chunk: BuiltVectorChunk,
    mask_layer_id: LayerId,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
    exact_lookups: &ExactLookupCache,
    raster_cache: &RasterTileCache,
    vector_runtime: &VectorLayerRuntime,
    zone_filter: &ZoneMembershipFilter,
) -> Option<BuiltVectorChunk> {
    let mut positions = Vec::new();
    let mut vertex_colors = Vec::new();
    let mut indices = Vec::new();

    for triangle in chunk.indices.chunks_exact(3) {
        let Some(a) = chunk.positions.get(triangle[0] as usize).copied() else {
            continue;
        };
        let Some(b) = chunk.positions.get(triangle[1] as usize).copied() else {
            continue;
        };
        let Some(c) = chunk.positions.get(triangle[2] as usize).copied() else {
            continue;
        };
        if !triangle_overlaps_visible_clip_mask(
            mask_layer_id,
            a,
            b,
            c,
            layer_registry,
            layer_runtime,
            exact_lookups,
            raster_cache,
            vector_runtime,
            zone_filter,
        ) {
            continue;
        }

        let base_index = positions.len() as u32;
        positions.extend([a, b, c]);
        vertex_colors.extend([
            chunk
                .vertex_colors
                .get(triangle[0] as usize)
                .copied()
                .unwrap_or(chunk.color_rgba),
            chunk
                .vertex_colors
                .get(triangle[1] as usize)
                .copied()
                .unwrap_or(chunk.color_rgba),
            chunk
                .vertex_colors
                .get(triangle[2] as usize)
                .copied()
                .unwrap_or(chunk.color_rgba),
        ]);
        indices.extend([base_index, base_index + 1, base_index + 2]);
    }

    if positions.is_empty() {
        return None;
    }

    let mut min_world_x = f32::INFINITY;
    let mut max_world_x = f32::NEG_INFINITY;
    let mut min_world_z = f32::INFINITY;
    let mut max_world_z = f32::NEG_INFINITY;
    for position in &positions {
        min_world_x = min_world_x.min(position[0]);
        max_world_x = max_world_x.max(position[0]);
        min_world_z = min_world_z.min(position[1]);
        max_world_z = max_world_z.max(position[1]);
    }

    Some(BuiltVectorChunk {
        color_rgba: chunk.color_rgba,
        vertex_colors,
        positions,
        indices,
        min_world_x,
        max_world_x,
        min_world_z,
        max_world_z,
    })
}

fn triangle_overlaps_visible_clip_mask(
    mask_layer_id: LayerId,
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
    exact_lookups: &ExactLookupCache,
    raster_cache: &RasterTileCache,
    vector_runtime: &VectorLayerRuntime,
    zone_filter: &ZoneMembershipFilter,
) -> bool {
    let sample_points = [
        WorldPoint::new(a[0] as f64, a[1] as f64),
        WorldPoint::new(b[0] as f64, b[1] as f64),
        WorldPoint::new(c[0] as f64, c[1] as f64),
        WorldPoint::new(
            (a[0] as f64 + b[0] as f64 + c[0] as f64) / 3.0,
            (a[1] as f64 + b[1] as f64 + c[1] as f64) / 3.0,
        ),
        midpoint_world_point(a, b),
        midpoint_world_point(b, c),
        midpoint_world_point(c, a),
    ];
    sample_points.into_iter().any(|world_point| {
        !matches!(
            clip_mask_allows_world_point(
                mask_layer_id,
                world_point,
                layer_registry,
                layer_runtime,
                exact_lookups,
                raster_cache,
                vector_runtime,
                zone_filter,
                layer_registry.map_version_id(),
            ),
            Some(false)
        )
    })
}

fn midpoint_world_point(a: [f32; 3], b: [f32; 3]) -> WorldPoint {
    WorldPoint::new(
        (a[0] as f64 + b[0] as f64) * 0.5,
        (a[1] as f64 + b[1] as f64) * 0.5,
    )
}

fn prune_stale_runtime_data(
    registry: &LayerRegistry,
    runtime: &mut VectorLayerRuntime,
    commands: &mut Commands,
) {
    let active_vector_layers: HashSet<LayerId> = registry
        .ordered()
        .iter()
        .filter(|layer| layer.is_vector())
        .map(|layer| layer.id)
        .collect();

    runtime
        .states
        .retain(|layer_id, _| active_vector_layers.contains(layer_id));

    for key in runtime.finished.keys() {
        if !active_vector_layers.contains(&key.0) {
            if let Some(bundle) = runtime.finished.remove(&key) {
                bundle.despawn(commands);
            }
        }
    }
}

fn invalidate_non_active_revisions(
    runtime: &mut VectorLayerRuntime,
    layer_id: LayerId,
    active_revision: &str,
    commands: &mut Commands,
) {
    for removed in runtime
        .finished
        .remove_layer_except(layer_id, active_revision)
    {
        removed.despawn(commands);
    }

    let should_reset_state = runtime
        .states
        .get(&layer_id)
        .and_then(state_revision)
        .map(|revision| revision != active_revision)
        .unwrap_or(false);
    if should_reset_state {
        runtime.states.remove(&layer_id);
    }
}

fn collect_visible_cache_keys(
    registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
    visible_by_layer: &HashMap<LayerId, bool>,
    map_version_id: Option<&str>,
    layer_filters: &LayerEffectiveFilterState,
) -> HashSet<(LayerId, String)> {
    let mut out = HashSet::new();
    for layer in registry.ordered() {
        if !layer.is_vector() {
            continue;
        }
        if !visible_by_layer.get(&layer.id).copied().unwrap_or(false) {
            continue;
        }
        let Some(source_ref) = layer.vector_source.as_ref() else {
            continue;
        };
        let source = resolve_vector_source_for_map_version(source_ref, map_version_id);
        let selected_field_ids = layer_filters.semantic_field_ids_for_layer(layer.key.as_str());
        let inactive_filter = EvidenceZoneFilter::default();
        let zone_filter = layer_filters
            .zone_membership_filter(layer.key.as_str())
            .unwrap_or(&inactive_filter);
        let clip_mask_revision = clip_mask_state_revision(
            registry,
            layer_runtime,
            layer_runtime.clip_mask_layer(layer.id),
            zone_filter,
        );
        out.insert((
            layer.id,
            effective_revision(&source, selected_field_ids, clip_mask_revision),
        ));
    }
    out
}

fn resolve_vector_source_for_map_version(
    source: &VectorSourceSpec,
    map_version_id: Option<&str>,
) -> VectorSourceSpec {
    let mut resolved = source.clone();
    if resolved.url.contains("{map_version}") {
        // "0v0" is a dev placeholder that usually has no vector artifact.
        let version = map_version_id
            .filter(|value| !value.trim().is_empty() && *value != "0v0")
            .unwrap_or("v1");
        resolved.url = resolved.url.replace("{map_version}", version);
    }
    resolved
}

fn clear_vector_metrics(runtime_state: &mut LayerRuntimeState, status: LayerVectorStatus) {
    clear_vector_build_metrics(runtime_state, status);
    runtime_state.vector_cache_hits = 0;
    runtime_state.vector_cache_misses = 0;
}

fn clear_vector_build_metrics(runtime_state: &mut LayerRuntimeState, status: LayerVectorStatus) {
    runtime_state.vector_status = status;
    runtime_state.vector_progress = 0.0;
    runtime_state.vector_fetched_bytes = 0;
    runtime_state.vector_feature_count = 0;
    runtime_state.vector_features_processed = 0;
    runtime_state.vector_polygon_count = 0;
    runtime_state.vector_multipolygon_count = 0;
    runtime_state.vector_hole_ring_count = 0;
    runtime_state.vector_vertex_count = 0;
    runtime_state.vector_triangle_count = 0;
    runtime_state.vector_mesh_count = 0;
    runtime_state.vector_chunked_bucket_count = 0;
    runtime_state.vector_build_ms = 0.0;
    runtime_state.vector_last_frame_build_ms = 0.0;
    runtime_state.vector_cache_last_hit = false;
    runtime_state.vector_cache_entries = 0;
}

fn hide_non_active_finished(
    runtime: &mut VectorLayerRuntime,
    layer_id: LayerId,
    active_revision: &str,
    visibility: FinishedVisibilityContext<'_, '_, '_>,
) {
    let FinishedVisibilityContext {
        commands,
        materials_2d,
        materials_3d,
        z_base,
        opacity,
        visible,
    } = visibility;
    let keys = runtime.finished.keys_for_layer(layer_id);

    for key in keys {
        let Some(bundle) = runtime.finished.get_ref(&key) else {
            continue;
        };
        if key.1 == active_revision {
            bundle.set_depth(commands, z_base, VECTOR_3D_BASE_Y);
            bundle.set_visibility(commands, visible);
            bundle.set_opacity(materials_2d, materials_3d, opacity);
        } else {
            bundle.set_visibility(commands, false);
        }
    }
}

#[derive(SystemParam)]
struct VectorLayerUpdate<'w, 's> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials_2d: ResMut<'w, Assets<ColorMaterial>>,
    materials_3d: ResMut<'w, Assets<StandardMaterial>>,
    registry: Res<'w, LayerRegistry>,
    layer_runtime: ResMut<'w, LayerRuntime>,
    vector_runtime: ResMut<'w, VectorLayerRuntime>,
    layer_filters: Res<'w, LayerEffectiveFilterState>,
    exact_lookups: Res<'w, ExactLookupCache>,
    raster_cache: Res<'w, RasterTileCache>,
    build_config: Res<'w, VectorBuildConfig>,
    cache_config: Res<'w, VectorCacheConfig>,
    view_mode: Res<'w, ViewModeState>,
    _marker: std::marker::PhantomData<&'s ()>,
}

struct FinishedVisibilityContext<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    materials_2d: &'a mut Assets<ColorMaterial>,
    materials_3d: &'a mut Assets<StandardMaterial>,
    z_base: f32,
    opacity: f32,
    visible: bool,
}

fn apply_stats(
    runtime_state: &mut LayerRuntimeState,
    stats: VectorLayerStats,
    status: LayerVectorStatus,
) {
    runtime_state.vector_status = status;
    runtime_state.vector_progress = stats.progress;
    runtime_state.vector_fetched_bytes = stats.fetched_bytes;
    runtime_state.vector_feature_count = stats.feature_count;
    runtime_state.vector_features_processed = stats.features_processed;
    runtime_state.vector_polygon_count = stats.polygon_count;
    runtime_state.vector_multipolygon_count = stats.multipolygon_count;
    runtime_state.vector_hole_ring_count = stats.hole_ring_count;
    runtime_state.vector_vertex_count = stats.vertex_count;
    runtime_state.vector_triangle_count = stats.triangle_count;
    runtime_state.vector_mesh_count = stats.mesh_count;
    runtime_state.vector_chunked_bucket_count = stats.chunked_bucket_count;
    runtime_state.vector_build_ms = stats.build_ms;
    runtime_state.vector_last_frame_build_ms = stats.last_frame_build_ms;
}

fn effective_revision(
    source: &VectorSourceSpec,
    selected_feature_ids: &[u32],
    clip_mask_revision: u64,
) -> String {
    let revision = source.revision.trim();
    let base_revision = if revision.is_empty() {
        format!("url:{}", source.url)
    } else {
        revision.to_string()
    };
    if selected_feature_ids.is_empty() {
        return if clip_mask_revision == 0 {
            base_revision
        } else {
            format!("{base_revision}|clip:{clip_mask_revision}")
        };
    }
    let selected_suffix = selected_feature_ids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    if clip_mask_revision == 0 {
        format!("{base_revision}|field_ids:{selected_suffix}")
    } else {
        format!("{base_revision}|field_ids:{selected_suffix}|clip:{clip_mask_revision}")
    }
}

fn vector_build_state_needs_frame_updates(state: &VectorBuildState) -> bool {
    matches!(
        state,
        VectorBuildState::Fetching { .. }
            | VectorBuildState::Parsing { .. }
            | VectorBuildState::Building { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::{
        clip_vector_chunk_against_mask, effective_build_budget_ms, effective_revision,
        should_activate_vector_layer, should_render_vector_layer,
        vector_build_state_needs_frame_updates, VectorBuildState, VectorLayerRuntime,
    };
    use crate::map::camera::mode::ViewMode;
    use crate::map::exact_lookup::ExactLookupCache;
    use crate::map::layers::{
        build_local_layer_specs, AvailableLayerCatalog, AvailableLayerDefinition,
        AvailableLayerTemplate, GeometrySpace, LayerId, LayerRegistry, LayerRuntime, StyleMode,
        VectorSourceSpec,
    };
    use crate::map::raster::RasterTileCache;
    use crate::map::vector::cache::{BuiltVectorChunk, VectorMeshBundleSet};
    use crate::plugins::points::EvidenceZoneFilter;
    use bevy::platform::time::Instant;
    use std::collections::HashMap;

    #[test]
    fn ready_vector_state_does_not_force_frame_updates() {
        assert!(!vector_build_state_needs_frame_updates(
            &VectorBuildState::Ready {
                revision: "rg-v1".to_string(),
            }
        ));
        assert!(!vector_build_state_needs_frame_updates(
            &VectorBuildState::Failed {
                revision: "rg-v1".to_string(),
                error: "nope".to_string(),
            }
        ));
    }

    #[test]
    fn runtime_only_requires_frame_updates_for_pending_work() {
        let mut runtime = VectorLayerRuntime {
            states: HashMap::new(),
            finished: Default::default(),
        };
        assert!(!runtime.has_pending_work());

        runtime.states.insert(
            LayerId::from_raw(1),
            VectorBuildState::Ready {
                revision: "ready".to_string(),
            },
        );
        assert!(!runtime.has_pending_work());

        runtime.states.insert(
            LayerId::from_raw(2),
            VectorBuildState::Parsing {
                source: VectorSourceSpec {
                    url: "/region_groups/v1.geojson".to_string(),
                    revision: "rg-v1".to_string(),
                    geometry_space: GeometrySpace::MapPixels,
                    style_mode: StyleMode::FeaturePropertyPalette,
                    feature_id_property: Some("id".to_string()),
                    color_property: Some("c".to_string()),
                },
                revision: "parsing".to_string(),
                bytes: vec![1, 2, 3],
                started_at: Instant::now(),
            },
        );
        assert!(runtime.has_pending_work());
    }

    fn field_backed_vector_layer() -> crate::map::layers::LayerSpec {
        let catalog = AvailableLayerCatalog::default();
        let (_, layers) = build_local_layer_specs(catalog.entries(), None);
        layers
            .into_iter()
            .find(|layer| layer.key == "regions")
            .expect("regions layer")
    }

    #[test]
    fn field_backed_vector_layers_still_render_in_2d() {
        let layer = field_backed_vector_layer();

        assert!(should_activate_vector_layer(
            &layer,
            true,
            false,
            ViewMode::Map2D,
        ));
        assert!(should_render_vector_layer(&layer, true, ViewMode::Map2D));
    }

    #[test]
    fn pending_vector_builds_keep_a_small_progress_budget_when_globally_exhausted() {
        assert_eq!(effective_build_budget_ms(3.0, 2.0), 2.0);
        assert_eq!(effective_build_budget_ms(3.0, 0.0), 0.25);
        assert_eq!(effective_build_budget_ms(0.1, 0.0), 0.1);
        assert_eq!(effective_build_budget_ms(0.0, 0.0), 0.0);
    }

    #[test]
    fn effective_revision_changes_when_clip_mask_revision_changes() {
        let source = VectorSourceSpec {
            url: "/region_groups/v1.geojson".to_string(),
            revision: "rg-v1".to_string(),
            geometry_space: GeometrySpace::MapPixels,
            style_mode: StyleMode::FeaturePropertyPalette,
            feature_id_property: Some("rg".to_string()),
            color_property: Some("c".to_string()),
        };

        assert_eq!(
            effective_revision(&source, &[212], 0),
            "rg-v1|field_ids:212"
        );
        assert_eq!(
            effective_revision(&source, &[212], 77),
            "rg-v1|field_ids:212|clip:77"
        );
    }

    #[test]
    fn clip_vector_chunk_against_vector_mask_drops_outside_triangles() {
        let definition = AvailableLayerDefinition {
            layer_id: "region_groups".to_string(),
            name: "Region Groups".to_string(),
            template: AvailableLayerTemplate::RegionGroups,
            visible_default: true,
            opacity_default: 0.5,
            z_base: 30.0,
            display_order: 30,
        };
        let (revision, layers) = build_local_layer_specs(&[definition], Some("v1"));
        let mask_layer = layers[0].clone();
        let mut registry = LayerRegistry::default();
        registry.apply_layer_specs(revision, Some("v1".to_string()), layers);

        let mask_chunk = BuiltVectorChunk {
            color_rgba: [255, 0, 0, 255],
            vertex_colors: vec![[255, 0, 0, 255]; 4],
            positions: vec![
                [0.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [10.0, 10.0, 0.0],
                [0.0, 10.0, 0.0],
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            min_world_x: 0.0,
            max_world_x: 10.0,
            min_world_z: 0.0,
            max_world_z: 10.0,
        };
        let test_chunk = BuiltVectorChunk {
            color_rgba: [0, 255, 0, 255],
            vertex_colors: vec![[0, 255, 0, 255]; 6],
            positions: vec![
                [1.0, 1.0, 0.0],
                [3.0, 1.0, 0.0],
                [2.0, 3.0, 0.0],
                [20.0, 20.0, 0.0],
                [22.0, 20.0, 0.0],
                [21.0, 22.0, 0.0],
            ],
            indices: vec![0, 1, 2, 3, 4, 5],
            min_world_x: 1.0,
            max_world_x: 22.0,
            min_world_z: 1.0,
            max_world_z: 22.0,
        };

        let mut vector_runtime = VectorLayerRuntime::default();
        vector_runtime.finished.insert(
            (mask_layer.id, "rg-v1".to_string()),
            VectorMeshBundleSet {
                hover_chunks: vec![mask_chunk],
                ..VectorMeshBundleSet::default()
            },
        );

        let clipped = clip_vector_chunk_against_mask(
            test_chunk,
            mask_layer.id,
            &registry,
            &LayerRuntime::default(),
            &ExactLookupCache::default(),
            &RasterTileCache::default(),
            &vector_runtime,
            &EvidenceZoneFilter::default(),
        )
        .expect("inside triangle should remain");

        assert_eq!(clipped.indices, vec![0, 1, 2]);
        assert_eq!(clipped.positions.len(), 3);
        assert_eq!(clipped.vertex_colors.len(), 3);
        assert_eq!(clipped.min_world_x, 1.0);
        assert_eq!(clipped.max_world_x, 3.0);
        assert_eq!(clipped.min_world_z, 1.0);
        assert_eq!(clipped.max_world_z, 3.0);
    }

    #[test]
    fn clip_vector_chunk_against_vector_mask_keeps_overlap_when_centroid_is_outside() {
        let definition = AvailableLayerDefinition {
            layer_id: "region_groups".to_string(),
            name: "Region Groups".to_string(),
            template: AvailableLayerTemplate::RegionGroups,
            visible_default: true,
            opacity_default: 0.5,
            z_base: 30.0,
            display_order: 30,
        };
        let (revision, layers) = build_local_layer_specs(&[definition], Some("v1"));
        let mask_layer = layers[0].clone();
        let mut registry = LayerRegistry::default();
        registry.apply_layer_specs(revision, Some("v1".to_string()), layers);

        let mask_chunk = BuiltVectorChunk {
            color_rgba: [255, 0, 0, 255],
            vertex_colors: vec![[255, 0, 0, 255]; 4],
            positions: vec![
                [0.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [10.0, 10.0, 0.0],
                [0.0, 10.0, 0.0],
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            min_world_x: 0.0,
            max_world_x: 10.0,
            min_world_z: 0.0,
            max_world_z: 10.0,
        };
        let overlapping_chunk = BuiltVectorChunk {
            color_rgba: [0, 255, 0, 255],
            vertex_colors: vec![[0, 255, 0, 255]; 3],
            positions: vec![[9.0, 9.0, 0.0], [13.0, 9.0, 0.0], [9.0, 13.0, 0.0]],
            indices: vec![0, 1, 2],
            min_world_x: 9.0,
            max_world_x: 13.0,
            min_world_z: 9.0,
            max_world_z: 13.0,
        };

        let mut vector_runtime = VectorLayerRuntime::default();
        vector_runtime.finished.insert(
            (mask_layer.id, "rg-v1".to_string()),
            VectorMeshBundleSet {
                hover_chunks: vec![mask_chunk],
                ..VectorMeshBundleSet::default()
            },
        );

        let clipped = clip_vector_chunk_against_mask(
            overlapping_chunk,
            mask_layer.id,
            &registry,
            &LayerRuntime::default(),
            &ExactLookupCache::default(),
            &RasterTileCache::default(),
            &vector_runtime,
            &EvidenceZoneFilter::default(),
        )
        .expect("triangle overlap should remain");

        assert_eq!(clipped.indices, vec![0, 1, 2]);
        assert_eq!(clipped.positions.len(), 3);
    }
}
