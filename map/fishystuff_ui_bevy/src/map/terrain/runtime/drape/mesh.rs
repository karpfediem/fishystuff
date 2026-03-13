use super::*;
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::MapPoint;
use fishystuff_core::terrain::{sample_chunk_norm_at_map_px, world_height_from_normalized};

pub(super) fn build_raster_tile_drape_mesh(
    map_corners: [Vec2; 4],
    subdivisions: u32,
    layer_offset: f32,
    loaded_manifest: &LoadedTerrainManifest,
    runtime: &TerrainRuntime,
    map_to_world: MapToWorld,
) -> Option<Mesh> {
    let edge = subdivisions.max(2) as usize + 1;
    let mut positions = Vec::with_capacity(edge * edge);
    let mut uvs = Vec::with_capacity(edge * edge);

    for gy in 0..edge {
        let v = gy as f32 / (edge - 1) as f32;
        for gx in 0..edge {
            let u = gx as f32 / (edge - 1) as f32;
            let map = bilinear_map_point(map_corners, u, v);
            let Some(base_height) =
                sample_world_height_from_chunks(runtime, loaded_manifest, map.x, map.y)
            else {
                continue;
            };
            let world = map_to_world.map_to_world(MapPoint::new(map.x as f64, map.y as f64));
            positions.push([world.x as f32, base_height + layer_offset, world.z as f32]);
            uvs.push([u, v]);
        }
    }

    if positions.len() != edge * edge {
        return None;
    }

    let mut indices = Vec::with_capacity((edge - 1) * (edge - 1) * 6);
    for y in 0..(edge - 1) {
        for x in 0..(edge - 1) {
            let i0 = (y * edge + x) as u32;
            let i1 = i0 + 1;
            let i2 = i0 + edge as u32;
            let i3 = i2 + 1;
            indices.extend_from_slice(&[i0, i1, i3, i0, i3, i2]);
        }
    }

    if indices.is_empty() {
        return None;
    }

    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0_f32, 1.0_f32, 0.0_f32]; edge * edge],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    Some(mesh)
}

fn sample_world_height_from_chunks(
    runtime: &TerrainRuntime,
    loaded_manifest: &LoadedTerrainManifest,
    map_x: f32,
    map_y: f32,
) -> Option<f32> {
    let manifest = &loaded_manifest.manifest;
    let target =
        crate::map::terrain::chunks::key_for_map_px(map_x, map_y, manifest.chunk_map_px, 0);
    let found = nearest_available_ancestor(target.raw(), manifest.max_level, |candidate| {
        runtime
            .chunks
            .get(&TerrainChunkKey(candidate))
            .map(|entry| entry.state == TerrainChunkState::Ready && entry.chunk.is_some())
            .unwrap_or(false)
    })?;
    let chunk = runtime
        .chunks
        .get(&TerrainChunkKey(found))
        .and_then(|entry| entry.chunk.as_ref())?;
    let norm = sample_chunk_norm_at_map_px(
        manifest.map_width,
        manifest.map_height,
        manifest.chunk_map_px,
        chunk,
        map_x,
        map_y,
    )?;
    Some(world_height_from_normalized(
        norm,
        manifest.bbox_y_min,
        manifest.bbox_y_max,
    ))
}

fn bilinear_map_point(corners: [Vec2; 4], u: f32, v: f32) -> Vec2 {
    // Corner order: [0]=TL, [1]=TR, [2]=BR, [3]=BL
    let top = corners[0].lerp(corners[1], u);
    let bottom = corners[3].lerp(corners[2], u);
    top.lerp(bottom, v)
}
