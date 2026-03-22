use super::*;
use crate::map::layers::{LayerRegistry, LayerRuntime, LayerSpec};
use crate::map::raster::RasterTileCache;
use crate::map::terrain::materials::{apply_drape_material_alpha, make_drape_material};
use bevy::ecs::system::SystemParam;

mod chunk_aligned;
mod mesh;
mod raster;

pub(super) fn update_draped_tiles(mut drapes: DrapeUpdate<'_, '_>) {
    drapes.runtime.frame = drapes.runtime.frame.wrapping_add(1);
    drapes.runtime.queued_drapes.clear();
    drapes.runtime.drape_queue.clear();

    if drapes.mode.mode != ViewMode::Terrain3D || !drapes.config.show_drape {
        for entry in drapes.runtime.chunk_drape_entries.values() {
            drapes
                .commands
                .entity(entry.entity)
                .insert(Visibility::Hidden);
        }
        for entry in drapes.runtime.drape_entries.values() {
            drapes
                .commands
                .entity(entry.entity)
                .insert(Visibility::Hidden);
        }
        drapes.runtime.pending_chunk_drape_textures.clear();
        drapes.runtime.drape_missing_textures = 0;
        drapes.runtime.drape_min_z = None;
        drapes.runtime.drape_max_z = None;
        return;
    }

    for entry in drapes.runtime.chunk_drape_entries.values() {
        drapes
            .commands
            .entity(entry.entity)
            .insert(Visibility::Hidden);
    }
    drapes.runtime.pending_chunk_drape_textures.clear();
    drapes.runtime.queued_chunk_drapes.clear();
    drapes.runtime.chunk_drape_queue.clear();

    if drapes.config.use_chunk_aligned_drape {
        for entry in drapes.runtime.drape_entries.values() {
            drapes
                .commands
                .entity(entry.entity)
                .insert(Visibility::Hidden);
        }
        chunk_aligned::update_chunk_aligned_drapes(chunk_aligned::ChunkAlignedDrapeUpdate {
            config: &drapes.config,
            runtime: &mut drapes.runtime,
            asset_server: &drapes.asset_server,
            layer_registry: &drapes.layer_registry,
            layer_runtime: &drapes.layer_runtime,
            commands: &mut drapes.commands,
            materials: &mut drapes.materials,
            images: &drapes.images,
        });
        return;
    }

    raster::update_raster_tile_drapes(raster::RasterTileDrapeUpdate {
        config: &drapes.config,
        runtime: &mut drapes.runtime,
        layer_registry: &drapes.layer_registry,
        layer_runtime: &drapes.layer_runtime,
        raster_tiles: &drapes.raster_tiles,
        commands: &mut drapes.commands,
        meshes: &mut drapes.meshes,
        materials: &mut drapes.materials,
    });
}

#[derive(SystemParam)]
pub(super) struct DrapeUpdate<'w, 's> {
    mode: Res<'w, ViewModeState>,
    config: Res<'w, Terrain3dConfig>,
    runtime: ResMut<'w, TerrainRuntime>,
    layer_registry: Res<'w, LayerRegistry>,
    layer_runtime: Res<'w, LayerRuntime>,
    asset_server: Res<'w, AssetServer>,
    images: Res<'w, Assets<Image>>,
    commands: Commands<'w, 's>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    raster_tiles: Res<'w, RasterTileCache>,
}

fn layer_surface_offset(config: &Terrain3dConfig, display_order: i32) -> f32 {
    config.drape_offset_base + display_order as f32 * config.drape_offset_per_layer
}

fn layer_supports_terrain_drape(layer: &LayerSpec) -> bool {
    layer.is_raster() && matches!(layer.key.as_str(), "minimap" | "zone_mask")
}
