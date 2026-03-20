use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::{
    Assets, Color, ColorMaterial, Commands, Mesh, Mesh2d, Mesh3d, MeshMaterial2d, MeshMaterial3d,
    StandardMaterial, Transform, Vec3,
};
use bevy::render::render_resource::PrimitiveTopology;

use crate::map::vector::cache::{BuiltVectorGeometry, VectorMeshBundleSet, VectorMeshChunk};
use crate::plugins::render_domain::{
    world_2d_layers, world_3d_layers, World2dRenderEntity, World3dRenderEntity,
};

pub const VECTOR_3D_BASE_Y: f32 = 24_500.0;

pub fn spawn_vector_meshes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials_2d: &mut Assets<ColorMaterial>,
    materials_3d: &mut Assets<StandardMaterial>,
    geometry: BuiltVectorGeometry,
    z_base: f32,
    opacity: f32,
) -> VectorMeshBundleSet {
    let crate::map::vector::cache::BuiltVectorGeometry {
        chunks: geometry_chunks,
        hover_features,
        stats,
    } = geometry;
    let mut chunks = Vec::with_capacity(geometry_chunks.len());
    let mut hover_chunks = Vec::with_capacity(geometry_chunks.len());
    let alpha = opacity.clamp(0.0, 1.0);

    for chunk in geometry_chunks {
        hover_chunks.push(chunk.clone());
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, chunk.positions);
        mesh.insert_indices(Indices::U32(chunk.indices));

        let mesh_handle = meshes.add(mesh);
        let rgba = chunk.color_rgba;
        let color = Color::srgba_u8(rgba[0], rgba[1], rgba[2], rgba[3]).to_srgba();
        let material_2d = materials_2d.add(ColorMaterial {
            color: Color::srgba(color.red, color.green, color.blue, alpha),
            ..Default::default()
        });
        let material_3d = materials_3d.add(StandardMaterial {
            base_color: Color::srgba(color.red, color.green, color.blue, alpha),
            alpha_mode: if alpha >= 0.999 {
                bevy::prelude::AlphaMode::Opaque
            } else {
                bevy::prelude::AlphaMode::Blend
            },
            unlit: true,
            cull_mode: None,
            depth_bias: 2.0,
            ..Default::default()
        });

        let entity_2d = commands
            .spawn((
                World2dRenderEntity,
                world_2d_layers(),
                Mesh2d(mesh_handle.clone()),
                MeshMaterial2d(material_2d.clone()),
                Transform::from_translation(bevy::math::Vec3::new(0.0, 0.0, z_base)),
            ))
            .id();
        let entity_3d = commands
            .spawn((
                World3dRenderEntity,
                world_3d_layers(),
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(material_3d.clone()),
                Transform::from_translation(Vec3::new(0.0, VECTOR_3D_BASE_Y + z_base, 0.0)),
            ))
            .id();

        chunks.push(VectorMeshChunk {
            entity_2d,
            material_2d,
            entity_3d,
            material_3d,
        });
    }

    VectorMeshBundleSet {
        chunks,
        hover_chunks,
        hover_features,
        stats,
    }
}
