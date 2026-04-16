use bevy::asset::{AssetServer, LoadState};
use bevy::image::ImageSampler;
use fishystuff_core::masks::ZoneLookupRows;

use crate::map::camera::mode::ViewMode;
use crate::map::layers::{LayerRegistry, LayerRuntime, PickMode};
use crate::map::render::tile_z;
use crate::map::spaces::layer_transform::TileSpace;
use crate::map::spaces::world::MapToWorld;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};
use crate::prelude::*;

use super::super::super::policy::{tile_should_render, TileResidencyState};
use super::super::{RasterTileCache, RasterTileEntity, TilePixelData, TileState, TileStats};
use super::geometry::{collect_tile_zone_rgbs, needs_affine_quad, tile_quad_mesh, tile_world_rect};

pub(crate) struct RasterLoadedAssets<'a> {
    pub(crate) images: &'a mut Assets<Image>,
    pub(crate) meshes: &'a mut Assets<Mesh>,
    pub(crate) materials: &'a mut Assets<ColorMaterial>,
}

pub(crate) struct RasterLoadedContext<'a> {
    pub(crate) asset_server: &'a AssetServer,
    pub(crate) layer_registry: &'a LayerRegistry,
    pub(crate) layer_runtime: &'a LayerRuntime,
    pub(crate) map_to_world: MapToWorld,
    pub(crate) view_mode: ViewMode,
    pub(crate) residency: &'a TileResidencyState,
    pub(crate) stats: &'a mut TileStats,
}

impl RasterTileCache {
    pub(crate) fn update_loaded(
        &mut self,
        commands: &mut Commands,
        assets: RasterLoadedAssets<'_>,
        context: RasterLoadedContext<'_>,
    ) -> bool {
        crate::perf_scope!("raster.tile_entity_update");
        let mut changed = false;
        let mut zone_tile_index_dirty = false;
        let RasterLoadedAssets {
            images,
            meshes,
            materials,
        } = assets;
        let RasterLoadedContext {
            asset_server,
            layer_registry,
            layer_runtime,
            map_to_world,
            view_mode,
            residency,
            stats,
        } = context;
        for (key, entry) in self.entries.iter_mut() {
            if entry.state != TileState::Loading {
                continue;
            }
            let asset_load_state = asset_server.get_load_state(&entry.handle);
            if matches!(asset_load_state, Some(LoadState::Failed(_))) {
                changed = true;
                entry.state = TileState::Failed;
                if stats.inflight > 0 {
                    stats.inflight -= 1;
                }
                continue;
            }
            if !matches!(asset_load_state, Some(LoadState::Loaded)) {
                continue;
            }

            changed = true;
            let Some(spec) = layer_registry.get(key.layer) else {
                entry.state = TileState::Failed;
                if stats.inflight > 0 {
                    stats.inflight -= 1;
                }
                continue;
            };
            let Some(world_transform) = spec.world_transform(map_to_world) else {
                entry.state = TileState::Failed;
                if stats.inflight > 0 {
                    stats.inflight -= 1;
                }
                continue;
            };

            if let Some(image) = images.get_mut(&entry.handle) {
                image.sampler = ImageSampler::nearest();
                // Keep exact-pick tile pixels available in every view mode. The same
                // textures are reused by terrain drapes, and filters may need to be
                // applied after a mode switch without forcing a tile reload.
                if spec.pick_mode == PickMode::ExactTilePixel {
                    if let Some(data) = image.data.clone() {
                        let size = image.texture_descriptor.size;
                        entry.zone_rgbs = collect_tile_zone_rgbs(&data);
                        entry.zone_lookup_rows =
                            ZoneLookupRows::from_rgba(size.width, size.height, &data).ok();
                        entry.pixel_data = Some(TilePixelData {
                            width: size.width,
                            height: size.height,
                            data,
                        });
                        zone_tile_index_dirty = true;
                    }
                } else {
                    entry.zone_rgbs.clear();
                    entry.zone_lookup_rows = None;
                    entry.pixel_data = None;
                    zone_tile_index_dirty = true;
                }
            }

            if entry.entity.is_none() {
                let tile_space = TileSpace::new(spec.tile_px, spec.y_flip);
                let depth = tile_z(layer_runtime.z_base(spec.id), spec.max_level, key.z);
                if needs_affine_quad(spec, world_transform) {
                    let Some(mesh) = tile_quad_mesh(key, spec, tile_space, world_transform) else {
                        entry.state = TileState::Failed;
                        if stats.inflight > 0 {
                            stats.inflight -= 1;
                        }
                        continue;
                    };
                    let mesh_handle = meshes.add(mesh);
                    let color_material_handle = materials.add(ColorMaterial {
                        texture: Some(entry.handle.clone()),
                        color: Color::srgba(1.0, 1.0, 1.0, entry.alpha),
                        ..default()
                    });
                    let entity = commands
                        .spawn((
                            RasterTileEntity,
                            World2dRenderEntity,
                            world_2d_layers(),
                            Mesh2d(mesh_handle),
                            MeshMaterial2d(color_material_handle.clone()),
                            Transform::from_translation(Vec3::new(0.0, 0.0, depth)),
                        ))
                        .id();
                    entry.entity = Some(entity);
                    entry.material = Some(color_material_handle);
                    entry.exact_quad = true;
                    entry.sprite_size = None;
                    entry.depth = depth;
                } else {
                    let Some((x0, y0, w, h)) =
                        tile_world_rect(key, spec, tile_space, world_transform)
                    else {
                        entry.state = TileState::Failed;
                        if stats.inflight > 0 {
                            stats.inflight -= 1;
                        }
                        continue;
                    };
                    let entity = commands
                        .spawn((
                            RasterTileEntity,
                            World2dRenderEntity,
                            world_2d_layers(),
                            Sprite {
                                image: entry.handle.clone(),
                                custom_size: Some(Vec2::new(w, h)),
                                color: Color::srgba(1.0, 1.0, 1.0, entry.alpha),
                                ..default()
                            },
                            Transform::from_translation(Vec3::new(
                                x0 + w * 0.5,
                                y0 + h * 0.5,
                                depth,
                            )),
                        ))
                        .id();
                    entry.entity = Some(entity);
                    entry.material = None;
                    entry.exact_quad = false;
                    entry.sprite_size = Some(Vec2::new(w, h));
                    entry.depth = depth;
                }
            }

            entry.state = TileState::Ready;
            let visible = tile_should_render(key, layer_runtime, residency);
            entry.visible = visible;
            if let Some(entity) = entry.entity {
                commands
                    .entity(entity)
                    .insert(if visible && view_mode == ViewMode::Map2D {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    });
            }
            if visible {
                self.use_counter = self.use_counter.wrapping_add(1);
                entry.last_used = self.use_counter;
            }
            if stats.inflight > 0 {
                stats.inflight -= 1;
            }
        }
        if zone_tile_index_dirty {
            self.mark_zone_tile_index_dirty();
        }
        changed
    }
}
