use super::*;

pub(super) fn update_chunk_aligned_drapes(
    config: &Terrain3dConfig,
    runtime: &mut TerrainRuntime,
    asset_server: &AssetServer,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    images: &Assets<Image>,
) {
    let Some(drape_manifest) = runtime.drape_manifest.clone() else {
        for entry in runtime.chunk_drape_entries.values() {
            commands.entity(entry.entity).insert(Visibility::Hidden);
        }
        runtime.pending_chunk_drape_textures.clear();
        runtime.drape_missing_textures = runtime.render_chunks.len();
        runtime.drape_min_z = None;
        runtime.drape_max_z = None;
        return;
    };

    let (layer_visible, layer_alpha, layer_offset) = layer_registry
        .get_by_key(&drape_manifest.manifest.layer)
        .map(|layer| {
            (
                layer_supports_terrain_drape(layer) && layer_runtime.visible(layer.id),
                layer_runtime.opacity(layer.id),
                layer_surface_offset(config, layer_runtime.display_order(layer.id)),
            )
        })
        .unwrap_or((true, 1.0, layer_surface_offset(config, 0)));

    if !layer_visible {
        for entry in runtime.chunk_drape_entries.values() {
            commands.entity(entry.entity).insert(Visibility::Hidden);
        }
        runtime.pending_chunk_drape_textures.clear();
        runtime.drape_missing_textures = 0;
        runtime.drape_min_z = None;
        runtime.drape_max_z = None;
        return;
    }

    let frame = runtime.frame;
    let mut missing_textures = 0usize;
    let render_keys: Vec<TerrainChunkKey> = runtime.render_chunks.iter().copied().collect();

    for key in render_keys {
        if !drape_manifest.contains(key) {
            missing_textures = missing_textures.saturating_add(1);
            continue;
        }
        if let Some(entry) = runtime.chunk_drape_entries.get_mut(&key) {
            entry.last_touched = frame;
            commands.entity(entry.entity).insert((
                Visibility::Visible,
                Transform::from_xyz(0.0, layer_offset, 0.0),
            ));
            if let Some(material) = materials.get_mut(&entry.material) {
                apply_drape_material_alpha(material, layer_alpha);
            }
            continue;
        }
        if runtime.queued_chunk_drapes.insert(key) {
            runtime.chunk_drape_queue.push_back(key);
        }
    }

    let start = Instant::now();
    let mut built = 0usize;
    while built < config.drape_builds_per_frame {
        if start.elapsed().as_secs_f64() * 1000.0 > config.build_ms_per_frame as f64 {
            break;
        }
        let Some(key) = runtime.chunk_drape_queue.pop_front() else {
            break;
        };
        runtime.queued_chunk_drapes.remove(&key);

        if !runtime.render_chunks.contains(&key) || runtime.chunk_drape_entries.contains_key(&key) {
            continue;
        }
        if !drape_manifest.contains(key) {
            missing_textures = missing_textures.saturating_add(1);
            continue;
        }

        let Some(chunk_mesh) = runtime
            .chunks
            .get(&key)
            .and_then(|entry| entry.mesh.as_ref().cloned())
        else {
            continue;
        };

        let texture = if let Some(handle) = runtime.pending_chunk_drape_textures.get(&key) {
            handle.clone()
        } else {
            let handle =
                asset_server.load(drape_manifest.manifest.chunk_url(key.raw()).to_string());
            runtime
                .pending_chunk_drape_textures
                .insert(key, handle.clone());
            handle
        };
        if images.get(&texture).is_none() {
            missing_textures = missing_textures.saturating_add(1);
            if runtime.queued_chunk_drapes.insert(key) {
                runtime.chunk_drape_queue.push_back(key);
            }
            continue;
        }
        runtime.pending_chunk_drape_textures.remove(&key);

        let material = make_drape_material(materials, texture.clone(), layer_alpha);
        let entity = commands
            .spawn((
                TerrainDrapeEntity {
                    key: TerrainDrapeKey::Chunk(key),
                },
                World3dRenderEntity,
                world_3d_layers(),
                Mesh3d(chunk_mesh),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(0.0, layer_offset, 0.0),
                Visibility::Visible,
            ))
            .id();

        runtime.chunk_drape_entries.insert(
            key,
            TerrainDrapeEntry {
                entity,
                material,
                last_touched: frame,
            },
        );
        built += 1;
    }

    let stale_keys: Vec<TerrainChunkKey> = runtime
        .chunk_drape_entries
        .iter()
        .filter_map(|(key, entry)| {
            let stale = frame.saturating_sub(entry.last_touched) > STALE_RETENTION_FRAMES
                || !runtime.render_chunks.contains(key);
            stale.then_some(*key)
        })
        .collect();
    for key in stale_keys {
        if let Some(entry) = runtime.chunk_drape_entries.remove(&key) {
            commands.entity(entry.entity).despawn();
        }
        runtime.pending_chunk_drape_textures.remove(&key);
    }
    let render_chunks = runtime.render_chunks.clone();
    runtime
        .pending_chunk_drape_textures
        .retain(|key, _| render_chunks.contains(key));

    runtime.drape_missing_textures = missing_textures;
    runtime.drape_min_z = None;
    runtime.drape_max_z = None;
}
