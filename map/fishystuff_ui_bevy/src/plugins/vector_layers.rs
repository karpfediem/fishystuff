use std::collections::{HashMap, HashSet};

use async_channel::TryRecvError;
use bevy::ecs::system::SystemParam;
use bevy::platform::time::Instant;
use bevy::prelude::*;

use crate::config::{
    VECTOR_FINISHED_CACHE_MAX, VECTOR_GLOBAL_BUILD_BUDGET_MS, VECTOR_MAX_BUILD_MS_PER_FRAME,
    VECTOR_MAX_CHUNK_TRIANGLES, VECTOR_MAX_CHUNK_VERTICES, VECTOR_MAX_FEATURES_PER_FRAME,
};
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::layers::{
    LayerId, LayerRegistry, LayerRuntime, LayerRuntimeState, LayerVectorStatus, VectorSourceSpec,
};
use crate::map::spaces::world::MapToWorld;
use crate::map::vector::build::{
    advance_job, begin_fetch, finalize_job, parse_into_job, revision_matches, state_revision,
    state_stats, state_status, AdvanceResult, VectorBuildLimits, VectorBuildState,
};
use crate::map::vector::cache::{VectorFinishedCache, VectorLayerStats};
use crate::map::vector::render::{spawn_vector_meshes, VECTOR_3D_BASE_Y};
use crate::plugins::bookmarks::BookmarkState;

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
            );
    }
}

fn vector_layers_need_update(
    registry: Res<'_, LayerRegistry>,
    layer_runtime: Res<'_, LayerRuntime>,
    vector_runtime: Res<'_, VectorLayerRuntime>,
    cache_config: Res<'_, VectorCacheConfig>,
    view_mode: Res<'_, ViewModeState>,
    bookmarks: Res<'_, BookmarkState>,
) -> bool {
    registry.is_changed()
        || layer_runtime.is_changed()
        || cache_config.is_changed()
        || view_mode.is_changed()
        || bookmarks.is_changed()
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
            && (layer_runtime.visible(layer.id) || clip_mask_source_ids.contains(&layer.id))
    });
    if !has_visible_vector_inputs
        && update.bookmarks.entries.is_empty()
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
    let mut active_by_layer: HashMap<LayerId, bool> = layer_runtime
        .iter()
        .map(|(layer_id, state)| {
            (
                layer_id,
                (state.visible || clip_mask_source_ids.contains(&layer_id))
                    && matches!(view_mode.mode, ViewMode::Map2D | ViewMode::Terrain3D),
            )
        })
        .collect();
    if let Some(regions_id) = registry.id_by_key("regions") {
        if !update.bookmarks.entries.is_empty() {
            active_by_layer.insert(regions_id, true);
        }
    }
    let visible_cache_keys = collect_visible_cache_keys(registry, &active_by_layer, map_version_id);
    for evicted in vector_runtime
        .finished
        .evict_lru_non_visible(|key| visible_cache_keys.contains(key))
    {
        evicted.despawn(&mut commands);
    }

    for layer in registry.ordered() {
        let Some(runtime_state) = layer_runtime.get_mut(layer.id) else {
            continue;
        };
        let active = active_by_layer.get(&layer.id).copied().unwrap_or(false);
        let render_visible = runtime_state.visible
            && matches!(view_mode.mode, ViewMode::Map2D | ViewMode::Terrain3D);
        let previous_status = runtime_state.vector_status;

        if !layer.is_vector() {
            clear_vector_metrics(runtime_state, LayerVectorStatus::Inactive);
            continue;
        }

        runtime_state.current_base_lod = None;
        runtime_state.current_detail_lod = None;
        runtime_state.visible_tile_count = 0;
        runtime_state.resident_tile_count = 0;
        runtime_state.pending_count = 0;
        runtime_state.inflight_count = 0;

        let Some(source_ref) = layer.vector_source.as_ref() else {
            clear_vector_metrics(runtime_state, LayerVectorStatus::Failed);
            continue;
        };
        if !active
            && !vector_runtime.states.contains_key(&layer.id)
            && vector_runtime.finished.layer_len(layer.id) == 0
        {
            clear_vector_build_metrics(runtime_state, LayerVectorStatus::NotRequested);
            continue;
        }
        let source = resolve_vector_source_for_map_version(source_ref, map_version_id);
        let revision = effective_revision(&source);
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
            apply_stats(runtime_state, bundle.stats, LayerVectorStatus::Ready);
            runtime_state.vector_cache_entries = vector_runtime.finished.layer_len(layer.id) as u32;
            vector_runtime
                .states
                .insert(layer.id, VectorBuildState::Ready { revision });
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
                apply_stats(runtime_state, stats, state_status(&state));
            } else {
                runtime_state.vector_status = LayerVectorStatus::NotRequested;
                runtime_state.vector_progress = 0.0;
                runtime_state.vector_last_frame_build_ms = 0.0;
            }
            runtime_state.vector_cache_entries = vector_runtime.finished.layer_len(layer.id) as u32;
            vector_runtime.states.insert(layer.id, state);
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
            } => match parse_into_job(source, revision.clone(), bytes) {
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
                    max_build_ms_per_frame: build_config
                        .max_build_ms_per_frame
                        .min(global_remaining_ms)
                        .max(0.0),
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
                            let geometry = finalize_job(job, limits);
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
                            apply_stats(runtime_state, bundle.stats, LayerVectorStatus::Ready);
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
            apply_stats(runtime_state, stats, state_status(&state));
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
    }
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
    visible_by_layer: &HashMap<LayerId, bool>,
    map_version_id: Option<&str>,
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
        out.insert((layer.id, effective_revision(&source)));
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
    build_config: Res<'w, VectorBuildConfig>,
    cache_config: Res<'w, VectorCacheConfig>,
    view_mode: Res<'w, ViewModeState>,
    bookmarks: Res<'w, BookmarkState>,
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

fn effective_revision(source: &VectorSourceSpec) -> String {
    let revision = source.revision.trim();
    if revision.is_empty() {
        format!("url:{}", source.url)
    } else {
        revision.to_string()
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
    use super::{vector_build_state_needs_frame_updates, VectorBuildState, VectorLayerRuntime};
    use crate::map::layers::{GeometrySpace, LayerId, StyleMode, VectorSourceSpec};
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
}
