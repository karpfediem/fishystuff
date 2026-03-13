use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;
use fishystuff_core::terrain::{
    chunk_local_uvs, chunk_vertex_positions, TerrainChunkData, TerrainManifest,
};

use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::MapPoint;
use crate::prelude::*;

pub fn build_chunk_mesh_from_data(
    chunk: &TerrainChunkData,
    manifest: &TerrainManifest,
    map_to_world: MapToWorld,
) -> Option<Mesh> {
    let edge = chunk.grid_size.max(2) as usize;
    let positions = chunk_vertex_positions(
        manifest.map_width,
        manifest.map_height,
        manifest.chunk_map_px,
        manifest.bbox_y_min,
        manifest.bbox_y_max,
        chunk,
        |map_x, map_y| {
            let world = map_to_world.map_to_world(MapPoint::new(map_x as f64, map_y as f64));
            (world.x as f32, world.z as f32)
        },
    )?;
    let uvs = chunk_local_uvs(chunk.grid_size);
    let normals = build_grid_normals(&positions, edge);

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
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
}

fn build_grid_normals(positions: &[[f32; 3]], edge: usize) -> Vec<[f32; 3]> {
    let mut normals = Vec::with_capacity(positions.len());
    for y in 0..edge {
        for x in 0..edge {
            let x_l = x.saturating_sub(1);
            let x_r = (x + 1).min(edge - 1);
            let y_d = y.saturating_sub(1);
            let y_u = (y + 1).min(edge - 1);

            let left = Vec3::from(positions[y * edge + x_l]);
            let right = Vec3::from(positions[y * edge + x_r]);
            let down = Vec3::from(positions[y_d * edge + x]);
            let up = Vec3::from(positions[y_u * edge + x]);
            let dx = right - left;
            let dy = up - down;
            let normal = dx.cross(dy).normalize_or_zero();
            let fixed = if normal.y < 0.0 { -normal } else { normal };
            normals.push([fixed.x, fixed.y, fixed.z]);
        }
    }
    normals
}

#[cfg(test)]
mod tests {
    use super::build_chunk_mesh_from_data;
    use crate::map::spaces::world::MapToWorld;
    use bevy::mesh::VertexAttributeValues;
    use bevy::prelude::Mesh;
    use fishystuff_core::terrain::{
        TerrainChunkData, TerrainChunkLodKey, TerrainHeightEncoding, TerrainManifest,
    };

    #[test]
    fn chunk_mesh_builds_from_manifest_encoded_heights() {
        let chunk = TerrainChunkData {
            key: TerrainChunkLodKey {
                level: 0,
                cx: 0,
                cy: 0,
            },
            grid_size: 2,
            encoding: TerrainHeightEncoding::U16Norm,
            heights: vec![0, u16::MAX, u16::MAX, 0],
        };
        let manifest = TerrainManifest {
            revision: "test".to_string(),
            map_width: 512,
            map_height: 512,
            chunk_map_px: 512,
            grid_size: 2,
            max_level: 0,
            bbox_y_min: -10.0,
            bbox_y_max: 30.0,
            encoding: TerrainHeightEncoding::U16Norm,
            root: "/terrain/test".to_string(),
            chunk_path: "levels/{level}/{x}_{y}.thc".to_string(),
            levels: Vec::new(),
        };

        let mesh =
            build_chunk_mesh_from_data(&chunk, &manifest, MapToWorld::default()).expect("mesh");
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("expected float32x3 positions");
        };

        assert_eq!(positions.len(), 4);
        assert!((positions[0][1] + 10.0).abs() < 1e-3);
        assert!((positions[1][1] - 30.0).abs() < 1e-3);
    }
}
