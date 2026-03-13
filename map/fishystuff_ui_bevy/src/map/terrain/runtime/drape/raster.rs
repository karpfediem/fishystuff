use super::*;
use crate::map::raster::RasterTileCache;
use crate::map::spaces::layer_transform::TileSpace;
use crate::map::spaces::world::MapToWorld;
use crate::map::terrain::drape::tile_map_corners_from_key;

pub(super) fn update_raster_tile_drapes(
    config: &Terrain3dConfig,
    runtime: &mut TerrainRuntime,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
    raster_tiles: &RasterTileCache,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let Some(loaded_manifest) = runtime.manifest.clone() else {
        for entry in runtime.drape_entries.values() {
            commands.entity(entry.entity).insert(Visibility::Hidden);
        }
        runtime.drape_missing_textures = 0;
        runtime.drape_min_z = None;
        runtime.drape_max_z = None;
        return;
    };

    let map_to_world = MapToWorld::default();
    let frame = runtime.frame;
    let mut missing = 0usize;
    let mut desired = HashSet::new();
    let mut attempted = 0usize;
    let start = Instant::now();

    let mut tiles = raster_tiles.ready_visible_tiles();
    tiles.sort_by_key(|tile| {
        (
            tile.key.layer.as_u16(),
            tile.key.z,
            tile.key.ty,
            tile.key.tx,
            tile.key.map_version,
        )
    });

    for tile in tiles {
        let Some(spec) = layer_registry.get(tile.key.layer) else {
            continue;
        };
        if !layer_supports_terrain_drape(spec) || !layer_runtime.visible(spec.id) {
            continue;
        }
        let alpha = layer_runtime.opacity(spec.id);
        let layer_offset = layer_surface_offset(config, layer_runtime.display_order(spec.id));
        desired.insert(tile.key);

        if let Some(entry) = runtime.drape_entries.get_mut(&tile.key) {
            entry.last_touched = frame;
            commands.entity(entry.entity).insert(Visibility::Visible);
            if let Some(material) = materials.get_mut(&entry.material) {
                apply_drape_material_alpha(material, alpha);
            }
            continue;
        }

        if attempted >= config.drape_builds_per_frame {
            continue;
        }
        if start.elapsed().as_secs_f64() * 1000.0 > config.build_ms_per_frame as f64 {
            continue;
        }

        let Some(world_transform) = spec.world_transform(map_to_world) else {
            missing = missing.saturating_add(1);
            continue;
        };
        let tile_space = TileSpace::new(spec.tile_px, spec.y_flip);
        let Some(map_corners) = tile_map_corners_from_key(&tile.key, tile_space, world_transform)
        else {
            missing = missing.saturating_add(1);
            continue;
        };
        attempted += 1;
        let Some(mesh) = mesh::build_raster_tile_drape_mesh(
            map_corners,
            config.drape_subdivisions.max(2),
            layer_offset,
            &loaded_manifest,
            runtime,
            map_to_world,
        ) else {
            missing = missing.saturating_add(1);
            continue;
        };

        let mesh_handle = meshes.add(mesh);
        let material = make_drape_material(materials, tile.texture.clone(), alpha);
        let entity = commands
            .spawn((
                TerrainDrapeEntity {
                    key: TerrainDrapeKey::Raster(tile.key),
                },
                World3dRenderEntity,
                world_3d_layers(),
                Mesh3d(mesh_handle),
                MeshMaterial3d(material.clone()),
                Transform::default(),
                Visibility::Visible,
            ))
            .id();
        runtime.drape_entries.insert(
            tile.key,
            TerrainDrapeEntry {
                entity,
                material,
                last_touched: frame,
            },
        );
    }

    let stale: Vec<TileKey> = runtime
        .drape_entries
        .iter()
        .filter_map(|(key, entry)| {
            let stale = !desired.contains(key)
                || frame.saturating_sub(entry.last_touched) > STALE_RETENTION_FRAMES;
            stale.then_some(*key)
        })
        .collect();
    for key in stale {
        if let Some(entry) = runtime.drape_entries.remove(&key) {
            commands.entity(entry.entity).despawn();
        }
    }

    runtime.drape_missing_textures = missing;
    runtime.drape_min_z = desired.iter().map(|key| key.z).min();
    runtime.drape_max_z = desired.iter().map(|key| key.z).max();
}
