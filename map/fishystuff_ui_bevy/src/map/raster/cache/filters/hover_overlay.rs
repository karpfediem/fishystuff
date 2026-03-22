use crate::map::layers::LayerSpec;
use crate::map::raster::cache::render::geometry::tile_quad_mesh;
use crate::map::raster::cache::render::zone_mask_hover_material::ZoneMaskHoverMaterial;
use crate::map::raster::TileKey;
use crate::map::spaces::layer_transform::{TileSpace, WorldTransform};
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};
use crate::prelude::*;

use super::super::RasterTileEntry;

const HOVER_OVERLAY_Z_BIAS: f32 = 0.0005;

pub(crate) fn hover_overlay_depth(base_depth: f32) -> f32 {
    base_depth + HOVER_OVERLAY_Z_BIAS
}

pub(super) fn ensure_hover_overlay_mesh(
    entry: &mut RasterTileEntry,
    meshes: &mut Assets<Mesh>,
    key: TileKey,
    layer: &LayerSpec,
    world_transform: WorldTransform,
) -> Option<Handle<Mesh>> {
    if let Some(handle) = entry.hover_overlay_mesh.as_ref() {
        return Some(handle.clone());
    }
    let tile_space = TileSpace::new(layer.tile_px, layer.y_flip);
    let mesh = tile_quad_mesh(&key, layer, tile_space, world_transform)?;
    let handle = meshes.add(mesh);
    entry.hover_overlay_mesh = Some(handle.clone());
    Some(handle)
}

pub(super) fn ensure_hover_overlay_material(
    entry: &mut RasterTileEntry,
    materials: &mut Assets<ZoneMaskHoverMaterial>,
    hover_zone_rgb: u32,
) -> Handle<ZoneMaskHoverMaterial> {
    if let Some(handle) = entry.hover_overlay_material.as_ref() {
        if let Some(material) = materials.get_mut(handle) {
            material.update(hover_zone_rgb, entry.alpha);
            return handle.clone();
        }
    }
    let handle = materials.add(ZoneMaskHoverMaterial::new(
        entry.handle.clone(),
        hover_zone_rgb,
        entry.alpha,
    ));
    entry.hover_overlay_material = Some(handle.clone());
    handle
}

pub(super) fn spawn_hover_overlay_entity(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<ZoneMaskHoverMaterial>,
    depth: f32,
) -> Entity {
    commands
        .spawn((
            World2dRenderEntity,
            world_2d_layers(),
            Mesh2d(mesh),
            MeshMaterial2d(material),
            Transform::from_translation(Vec3::new(0.0, 0.0, hover_overlay_depth(depth))),
            Visibility::Visible,
        ))
        .id()
}
