use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;

use crate::map::layers::{LayerRenderKind, LayerSpec};
use crate::map::raster::TileKey;
use crate::map::spaces::layer_transform::{LayerTransform, TileSpace, WorldTransform};
use crate::map::spaces::{LayerPoint, LayerRect};
use crate::prelude::*;

const AFFINE_QUAD_EPS: f64 = 1e-9;

pub(super) fn needs_affine_quad(layer: &LayerSpec, world_transform: WorldTransform) -> bool {
    if layer.render_kind() == LayerRenderKind::IdentitySprite {
        return false;
    }
    let affine = world_transform.layer_to_world;
    affine.b.abs() > AFFINE_QUAD_EPS
        || affine.c.abs() > AFFINE_QUAD_EPS
        || affine.a < 0.0
        || affine.d < 0.0
        || matches!(layer.transform, LayerTransform::AffineToMap(_))
}

pub(crate) fn tile_quad_mesh(
    key: &TileKey,
    layer: &LayerSpec,
    tile_space: TileSpace,
    world_transform: WorldTransform,
) -> Option<Mesh> {
    let corners = tile_layer_corners(
        key.tx,
        key.ty,
        key.z,
        tile_space,
        layer_bounds(layer, world_transform),
    )?;
    let world = corners.map(|corner| world_transform.layer_to_world(corner));

    let positions = vec![
        [world[0].x as f32, world[0].z as f32, 0.0],
        [world[1].x as f32, world[1].z as f32, 0.0],
        [world[2].x as f32, world[2].z as f32, 0.0],
        [world[3].x as f32, world[3].z as f32, 0.0],
    ];
    let uvs = vec![[0.0_f32, 0.0_f32], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    Some(mesh)
}

pub(super) fn tile_world_rect(
    key: &TileKey,
    layer: &LayerSpec,
    tile_space: TileSpace,
    world_transform: WorldTransform,
) -> Option<(f32, f32, f32, f32)> {
    let corners = tile_layer_corners(
        key.tx,
        key.ty,
        key.z,
        tile_space,
        layer_bounds(layer, world_transform),
    )?;
    let world_corners = corners.map(|corner| world_transform.layer_to_world(corner));
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for corner in world_corners {
        min_x = min_x.min(corner.x);
        max_x = max_x.max(corner.x);
        min_z = min_z.min(corner.z);
        max_z = max_z.max(corner.z);
    }
    Some((
        min_x as f32,
        min_z as f32,
        (max_x - min_x) as f32,
        (max_z - min_z) as f32,
    ))
}

fn layer_bounds(layer: &LayerSpec, world_transform: WorldTransform) -> Option<LayerRect> {
    match layer.transform {
        LayerTransform::IdentityMapSpace | LayerTransform::AffineToMap(_) => {
            let corners = world_transform
                .map_to_world
                .map_bounds()
                .corners()
                .map(|corner| world_transform.map_to_layer(corner));
            let mut min_x = f64::INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut max_y = f64::NEG_INFINITY;
            for corner in corners {
                min_x = min_x.min(corner.x);
                min_y = min_y.min(corner.y);
                max_x = max_x.max(corner.x);
                max_y = max_y.max(corner.y);
            }
            Some(LayerRect {
                min: LayerPoint::new(min_x, min_y),
                max: LayerPoint::new(max_x, max_y),
            })
        }
        LayerTransform::AffineToWorld(_) => None,
    }
}

fn tile_layer_corners(
    tile_x: i32,
    tile_y: i32,
    z: i32,
    tile_space: TileSpace,
    layer_bounds: Option<LayerRect>,
) -> Option<[LayerPoint; 4]> {
    let tile_span = tile_space.tile_span_px(z)?;
    let x0 = tile_x as f64 * tile_span;
    let x1 = x0 + tile_span;
    let y0 = tile_y as f64 * tile_span;
    let y1 = y0 + tile_span;
    let (x0, x1, y0, y1) = if let Some(bounds) = layer_bounds {
        (
            x0.max(bounds.min.x),
            x1.min(bounds.max.x),
            y0.max(bounds.min.y),
            y1.min(bounds.max.y),
        )
    } else {
        (x0, x1, y0, y1)
    };
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    if tile_space.y_flip {
        Some([
            LayerPoint::new(x0, y1),
            LayerPoint::new(x1, y1),
            LayerPoint::new(x1, y0),
            LayerPoint::new(x0, y0),
        ])
    } else {
        Some([
            LayerPoint::new(x0, y0),
            LayerPoint::new(x1, y0),
            LayerPoint::new(x1, y1),
            LayerPoint::new(x0, y1),
        ])
    }
}

pub(super) fn collect_tile_zone_rgbs(data: &[u8]) -> Vec<u32> {
    let mut zones = std::collections::HashSet::new();
    for px in data.chunks_exact(4) {
        zones.insert(fishystuff_core::masks::pack_rgb_u32(px[0], px[1], px[2]));
    }
    zones.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::{layer_bounds, tile_layer_corners};
    use crate::map::layers::{LayerId, LayerKind, LayerSpec, LodPolicy, PickMode};
    use crate::map::raster::TileKey;
    use crate::map::spaces::layer_transform::{LayerTransform, TileSpace, WorldTransform};
    use crate::map::spaces::world::MapToWorld;

    fn identity_layer() -> LayerSpec {
        LayerSpec {
            id: LayerId::from_raw(0),
            key: "zone_mask".to_string(),
            name: "Zone Mask".to_string(),
            visible_default: true,
            opacity_default: 1.0,
            z_base: 0.0,
            kind: LayerKind::TiledRaster,
            tileset_url: "/images/tiles/mask/v1/tileset.json".to_string(),
            tile_url_template: "/images/tiles/mask/v1/{level}/{x}_{y}.png".to_string(),
            tileset_version: "v1".to_string(),
            vector_source: None,
            transform: LayerTransform::IdentityMapSpace,
            tile_px: 512,
            max_level: 0,
            y_flip: false,
            lod_policy: LodPolicy {
                target_tiles: 64,
                hysteresis_hi: 80.0,
                hysteresis_lo: 40.0,
                margin_tiles: 0,
                enable_refine: true,
                refine_debounce_ms: 0,
                max_detail_tiles: 128,
                max_resident_tiles: 256,
                pinned_coarse_levels: 2,
                coarse_pin_min_level: None,
                warm_margin_tiles: 1,
                protected_margin_tiles: 0,
                detail_eviction_weight: 4.0,
                max_detail_requests_while_camera_moving: 1,
                motion_suppresses_refine: true,
            },
            request_weight: 1.0,
            pick_mode: PickMode::ExactTilePixel,
            display_order: 0,
        }
    }

    #[test]
    fn southeast_identity_tile_clamps_to_map_bounds() {
        let map_to_world = MapToWorld::default();
        let layer = identity_layer();
        let world_transform = WorldTransform::new(layer.transform, map_to_world).expect("invert");
        let key = TileKey {
            layer: layer.id,
            map_version: 0,
            z: 0,
            tx: 22,
            ty: 20,
        };
        let corners = tile_layer_corners(
            key.tx,
            key.ty,
            key.z,
            TileSpace::new(layer.tile_px, layer.y_flip),
            layer_bounds(&layer, world_transform),
        )
        .expect("edge tile corners");

        assert_eq!(corners[0].x, 11_264.0);
        assert_eq!(corners[1].x, 11_560.0);
        assert_eq!(corners[0].y, 10_240.0);
        assert_eq!(corners[2].y, 10_540.0);
    }
}
