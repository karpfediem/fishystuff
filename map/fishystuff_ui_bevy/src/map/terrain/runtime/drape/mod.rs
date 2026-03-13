use super::*;
use crate::map::layers::{LayerRegistry, LayerRuntime, LayerSpec};
use crate::map::raster::RasterTileCache;
use crate::map::terrain::materials::{apply_drape_material_alpha, make_drape_material};

mod chunk_aligned;
mod mesh;
mod raster;

pub(super) fn update_draped_tiles(
    mode: Res<ViewModeState>,
    config: Res<Terrain3dConfig>,
    mut runtime: ResMut<TerrainRuntime>,
    layer_registry: Res<LayerRegistry>,
    layer_runtime: Res<LayerRuntime>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    raster_tiles: Res<RasterTileCache>,
) {
    runtime.frame = runtime.frame.wrapping_add(1);
    runtime.queued_drapes.clear();
    runtime.drape_queue.clear();

    if mode.mode != ViewMode::Terrain3D || !config.show_drape {
        for entry in runtime.chunk_drape_entries.values() {
            commands.entity(entry.entity).insert(Visibility::Hidden);
        }
        for entry in runtime.drape_entries.values() {
            commands.entity(entry.entity).insert(Visibility::Hidden);
        }
        runtime.pending_chunk_drape_textures.clear();
        runtime.drape_missing_textures = 0;
        runtime.drape_min_z = None;
        runtime.drape_max_z = None;
        return;
    }

    for entry in runtime.chunk_drape_entries.values() {
        commands.entity(entry.entity).insert(Visibility::Hidden);
    }
    runtime.pending_chunk_drape_textures.clear();
    runtime.queued_chunk_drapes.clear();
    runtime.chunk_drape_queue.clear();

    if config.use_chunk_aligned_drape {
        for entry in runtime.drape_entries.values() {
            commands.entity(entry.entity).insert(Visibility::Hidden);
        }
        chunk_aligned::update_chunk_aligned_drapes(
            &config,
            &mut runtime,
            &asset_server,
            &layer_registry,
            &layer_runtime,
            &mut commands,
            &mut materials,
            &images,
        );
        return;
    }

    raster::update_raster_tile_drapes(
        &config,
        &mut runtime,
        &layer_registry,
        &layer_runtime,
        &raster_tiles,
        &mut commands,
        &mut meshes,
        &mut materials,
    );
}

fn layer_surface_offset(config: &Terrain3dConfig, display_order: i32) -> f32 {
    config.drape_offset_base + display_order as f32 * config.drape_offset_per_layer
}

fn layer_supports_terrain_drape(layer: &LayerSpec) -> bool {
    layer.is_raster() && matches!(layer.key.as_str(), "minimap" | "zone_mask")
}
