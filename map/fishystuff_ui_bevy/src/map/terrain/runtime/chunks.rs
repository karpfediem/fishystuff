use super::manifest::spawn_bytes_request;
use super::*;

pub(super) fn queue_visible_chunks(
    mode: Res<ViewModeState>,
    config: Res<Terrain3dConfig>,
    view: Res<Terrain3dViewState>,
    mut runtime: ResMut<TerrainRuntime>,
) {
    crate::perf_scope!("terrain.visible_chunk_computation");
    let frame = runtime.frame;
    if mode.mode != ViewMode::Terrain3D {
        return;
    }
    let Some(manifest) = runtime.manifest.clone() else {
        return;
    };

    let map_to_world = MapToWorld::default();
    let map_center = map_to_world.world_to_map(WorldPoint::new(
        view.pivot_world.x as f64,
        view.pivot_world.z as f64,
    ));
    let px_per_map = map_to_world.distance_per_pixel as f32;
    let radius_map_px =
        (view.distance / px_per_map).max(manifest.manifest.chunk_map_px as f32 * 2.0) * 1.25;
    let base_level = lod_for_view_distance(
        view.distance,
        px_per_map,
        manifest.manifest.chunk_map_px,
        config.terrain_target_chunks_radius,
        manifest.manifest.max_level,
    );
    let detail_level = base_level.saturating_sub(1);

    let mut desired = Vec::new();
    let (base_xs, base_ys) = visible_chunk_range(
        map_center.x as f32,
        map_center.y as f32,
        radius_map_px,
        manifest.manifest.map_width,
        manifest.manifest.map_height,
        manifest.manifest.chunk_map_px,
        base_level,
    );
    for cy in base_ys {
        for cx in base_xs.clone() {
            let key = TerrainChunkKey::new(base_level, cx, cy);
            if manifest.contains(key) {
                desired.push(key);
            }
        }
    }
    if config.use_chunk_aligned_drape && detail_level < base_level {
        let detail_radius = radius_map_px * 0.65;
        let (detail_xs, detail_ys) = visible_chunk_range(
            map_center.x as f32,
            map_center.y as f32,
            detail_radius,
            manifest.manifest.map_width,
            manifest.manifest.map_height,
            manifest.manifest.chunk_map_px,
            detail_level,
        );
        for cy in detail_ys {
            for cx in detail_xs.clone() {
                let key = TerrainChunkKey::new(detail_level, cx, cy);
                if manifest.contains(key) {
                    desired.push(key);
                }
            }
        }
    }
    let desired_set: HashSet<TerrainChunkKey> = desired.iter().copied().collect();

    let mut pinned = HashSet::new();
    let pinned_levels = config
        .terrain_pinned_coarse_levels
        .max(1)
        .min(manifest.manifest.max_level + 1);
    for offset in 0..pinned_levels {
        let level = manifest.manifest.max_level.saturating_sub(offset);
        let (tiles_x, tiles_y) = chunk_grid_dims_for_level(
            manifest.manifest.map_width,
            manifest.manifest.map_height,
            manifest.manifest.chunk_map_px,
            level,
        );
        for cy in 0..tiles_y {
            for cx in 0..tiles_x {
                let key = TerrainChunkKey::new(level, cx, cy);
                if manifest.contains(key) {
                    pinned.insert(key);
                }
            }
        }
    }
    runtime.pinned_chunks = pinned.clone();
    for entry in runtime.chunks.values_mut() {
        entry.residency = TerrainChunkResidency::Evictable;
    }

    let mut render_set = HashSet::new();
    let mut fallback_count = 0_u32;
    {
        crate::perf_scope!("terrain.chunk_cache_resolution");
        for key in &desired {
            let resolved =
                nearest_available_ancestor(key.raw(), manifest.manifest.max_level, |candidate| {
                    let entry = runtime.chunks.get(&TerrainChunkKey(candidate));
                    entry
                        .map(|item| item.state == TerrainChunkState::Ready && item.chunk.is_some())
                        .unwrap_or(false)
                });
            if let Some(found) = resolved {
                let found_key = TerrainChunkKey(found);
                if found_key != *key {
                    fallback_count = fallback_count.saturating_add(1);
                }
                render_set.insert(found_key);
            }
        }
    }
    runtime.render_chunks = render_set;
    runtime.fallback_chunks = fallback_count;

    runtime.visible_chunks_by_level.clear();
    let render_keys: Vec<TerrainChunkKey> = runtime.render_chunks.iter().copied().collect();
    for key in render_keys {
        *runtime
            .visible_chunks_by_level
            .entry(key.level())
            .or_default() += 1;
    }

    let mut request_targets = Vec::new();
    request_targets.extend(desired.iter().copied());
    request_targets.extend(pinned.iter().copied());
    for key in request_targets {
        if !manifest.contains(key) {
            continue;
        }
        let protected = desired_set.contains(&key) || pinned.contains(&key);
        let already_queued =
            runtime.queued_chunks.contains(&key) || runtime.pending_chunks.contains_key(&key);
        let entry = runtime.chunks.entry(key).or_default();
        entry.last_touched = frame;
        if protected {
            entry.residency = TerrainChunkResidency::Protected;
        } else {
            entry.residency = TerrainChunkResidency::Warm;
        }

        let needs_request = matches!(
            entry.state,
            TerrainChunkState::NotRequested | TerrainChunkState::Failed
        ) && !already_queued;
        if needs_request {
            runtime.queued_chunks.insert(key);
            runtime.chunk_queue.push_back(key);
        }
    }
}

pub(super) fn build_chunks_incremental(
    mode: Res<ViewModeState>,
    config: Res<Terrain3dConfig>,
    mut runtime: ResMut<TerrainRuntime>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    crate::perf_scope!("terrain.chunk_build");
    if mode.mode != ViewMode::Terrain3D {
        return;
    }
    let Some(manifest) = runtime.manifest.clone() else {
        return;
    };
    let start = Instant::now();
    let mut started_requests = 0usize;
    while started_requests < config.terrain_chunk_requests_per_frame
        && runtime.pending_chunks.len() < config.terrain_chunk_max_inflight
    {
        let Some(key) = runtime.chunk_queue.pop_front() else {
            break;
        };
        runtime.queued_chunks.remove(&key);
        if runtime.pending_chunks.contains_key(&key) {
            continue;
        }
        let Some(entry) = runtime.chunks.get(&key) else {
            continue;
        };
        if entry.state == TerrainChunkState::Ready && entry.chunk.is_some() {
            continue;
        }
        let url = manifest.manifest.chunk_url(key.raw());
        runtime.pending_chunks.insert(
            key,
            PendingTerrainChunkRequest {
                receiver: spawn_bytes_request(url),
            },
        );
        if let Some(entry) = runtime.chunks.get_mut(&key) {
            entry.state = TerrainChunkState::Building;
        }
        started_requests += 1;
    }

    let mut build_keys = Vec::new();
    for (key, entry) in &runtime.chunks {
        if entry.state == TerrainChunkState::Building && entry.chunk.is_some() {
            build_keys.push(*key);
        }
    }
    build_keys.sort_by_key(|key| (key.level(), key.cy(), key.cx()));

    let mut built = 0usize;
    if !config.use_chunk_aligned_drape {
        for key in build_keys {
            let Some(entry) = runtime.chunks.get_mut(&key) else {
                continue;
            };
            if entry.chunk.is_none() {
                continue;
            }
            entry.state = TerrainChunkState::Ready;
        }

        enforce_terrain_cache_limits(&mut runtime, &config, &mut commands);
        sync_resident_chunk_counts(&mut runtime);
        update_avg_build_ms(&mut runtime, start);
        return;
    }

    for key in build_keys {
        if built >= config.build_chunks_per_frame {
            break;
        }
        if start.elapsed().as_secs_f64() * 1000.0 > config.build_ms_per_frame as f64 {
            break;
        }
        let Some(entry) = runtime.chunks.get_mut(&key) else {
            continue;
        };
        let Some(chunk) = entry.chunk.as_ref() else {
            continue;
        };
        let mesh = {
            crate::perf_scope!("terrain.mesh_build");
            build_chunk_mesh_from_data(chunk, &manifest.manifest, MapToWorld::default())
        };
        match mesh {
            Some(mesh) => {
                if let Some(old) = entry.entity.take() {
                    commands.entity(old).despawn();
                }
                let mesh_handle = meshes.add(mesh);
                entry.entity = None;
                entry.mesh = Some(mesh_handle);
                entry.state = TerrainChunkState::Ready;
                built += 1;
            }
            None => {
                entry.state = TerrainChunkState::Failed;
                entry.chunk = None;
            }
        }
    }

    enforce_terrain_cache_limits(&mut runtime, &config, &mut commands);
    sync_resident_chunk_counts(&mut runtime);
    update_avg_build_ms(&mut runtime, start);
}

fn sync_resident_chunk_counts(runtime: &mut TerrainRuntime) {
    let mut resident = BTreeMap::new();
    for (key, entry) in &runtime.chunks {
        if entry.state == TerrainChunkState::Ready && entry.chunk.is_some() {
            *resident.entry(key.level()).or_default() += 1;
        }
    }
    runtime.resident_chunks_by_level = resident;
}

fn update_avg_build_ms(runtime: &mut TerrainRuntime, start: Instant) {
    let frame_ms = (start.elapsed().as_secs_f64() * 1000.0) as f32;
    runtime.avg_build_ms = if runtime.avg_build_ms <= 0.0 {
        frame_ms
    } else {
        runtime.avg_build_ms * 0.9 + frame_ms * 0.1
    };
}

fn residency_rank(residency: TerrainChunkResidency) -> i32 {
    match residency {
        TerrainChunkResidency::Protected => 2,
        TerrainChunkResidency::Warm => 1,
        TerrainChunkResidency::Evictable => 0,
    }
}

fn enforce_terrain_cache_limits(
    runtime: &mut TerrainRuntime,
    config: &Terrain3dConfig,
    commands: &mut Commands,
) {
    crate::perf_scope!("terrain.cache_eviction");
    let max_entries = config.terrain_cache_max_chunks.max(64);
    let resident = runtime
        .chunks
        .iter()
        .filter(|(_, entry)| entry.state == TerrainChunkState::Ready && entry.chunk.is_some())
        .count();
    if resident <= max_entries {
        return;
    }

    let mut candidates = Vec::new();
    for (key, entry) in &runtime.chunks {
        if entry.state != TerrainChunkState::Ready || entry.chunk.is_none() {
            continue;
        }
        let protected = matches!(entry.residency, TerrainChunkResidency::Protected)
            || runtime.render_chunks.contains(key)
            || runtime.pinned_chunks.contains(key);
        if protected {
            continue;
        }
        candidates.push((*key, entry.last_touched, entry.residency));
    }
    candidates.sort_by(|lhs, rhs| {
        let lhs_rank = residency_rank(lhs.2);
        let rhs_rank = residency_rank(rhs.2);
        lhs_rank
            .cmp(&rhs_rank)
            .then_with(|| lhs.0.level().cmp(&rhs.0.level()))
            .then_with(|| lhs.1.cmp(&rhs.1))
    });

    let mut remove_count = resident.saturating_sub(max_entries);
    for (key, _, _) in candidates {
        if remove_count == 0 {
            break;
        }
        if let Some(entry) = runtime.chunks.get_mut(&key) {
            if let Some(entity) = entry.entity.take() {
                commands.entity(entity).despawn();
            }
            entry.mesh = None;
            entry.chunk = None;
            entry.state = TerrainChunkState::NotRequested;
            runtime.cache_evictions = runtime.cache_evictions.saturating_add(1);
            crate::perf_counter_add!("terrain.cache_evictions", 1);
            remove_count -= 1;
        }
    }
}
