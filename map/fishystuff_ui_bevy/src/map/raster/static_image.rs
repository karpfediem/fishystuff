use std::collections::{HashMap, HashSet};

use bevy::asset::{AssetServer, LoadState};
use bevy::image::ImageSampler;
use bevy::prelude::Resource;
use bevy::render::render_resource::PrimitiveTopology;

use crate::map::camera::mode::ViewMode;
use crate::map::layers::{LayerId, LayerManifestStatus, LayerRegistry, LayerRuntime, LayerSpec};
use crate::map::render::tile_z;
use crate::map::spaces::layer_transform::{LayerTransform, WorldTransform};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::LayerRect;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};
use crate::prelude::*;

const AFFINE_QUAD_EPS: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StaticRasterState {
    Loading,
    Ready,
    Failed,
}

#[derive(Debug)]
struct StaticRasterEntry {
    url: String,
    handle: Handle<Image>,
    entity: Option<Entity>,
    material: Option<Handle<ColorMaterial>>,
    state: StaticRasterState,
    exact_quad: bool,
    sprite_rect: Option<(f32, f32, f32, f32)>,
}

#[derive(Resource, Default)]
pub(crate) struct StaticRasterCache {
    entries: HashMap<LayerId, StaticRasterEntry>,
}

pub(crate) fn update_static_images(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    layer_registry: Res<LayerRegistry>,
    mut layer_runtime: ResMut<LayerRuntime>,
    view_mode: Res<crate::map::camera::mode::ViewModeState>,
    mut cache: ResMut<StaticRasterCache>,
) {
    crate::perf_scope!("raster.static_image_update");
    let map_to_world = MapToWorld::default();
    let mut active_layer_ids = HashSet::new();

    for layer in layer_registry
        .ordered()
        .iter()
        .filter(|layer| layer.static_image_url().is_some())
    {
        active_layer_ids.insert(layer.id);
        ensure_static_entry(layer, &asset_server, &mut cache, &mut commands);
        let Some(entry) = cache.entries.get_mut(&layer.id) else {
            continue;
        };
        let Some(runtime_state) = layer_runtime.get_mut(layer.id) else {
            continue;
        };
        let visible = runtime_state.visible && view_mode.mode == ViewMode::Map2D;
        let depth = tile_z(runtime_state.z_base, 0, 0);

        match asset_server.get_load_state(&entry.handle) {
            Some(LoadState::Failed(_)) => {
                entry.state = StaticRasterState::Failed;
            }
            Some(LoadState::Loaded) => {
                if let Some(image) = images.get_mut(&entry.handle) {
                    image.sampler = ImageSampler::nearest();
                }
                if entry.entity.is_none() {
                    if spawn_static_entity(
                        layer,
                        entry,
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        map_to_world,
                        depth,
                        runtime_state.opacity,
                        visible,
                    )
                    .is_err()
                    {
                        entry.state = StaticRasterState::Failed;
                    } else {
                        entry.state = StaticRasterState::Ready;
                    }
                } else {
                    entry.state = StaticRasterState::Ready;
                    update_static_entity(
                        entry,
                        &mut commands,
                        &mut materials,
                        runtime_state.opacity,
                        visible,
                        depth,
                    );
                }
            }
            _ => {
                entry.state = StaticRasterState::Loading;
            }
        }

        runtime_state.current_base_lod = None;
        runtime_state.current_detail_lod = None;
        runtime_state.visible_tile_count =
            u32::from(entry.state == StaticRasterState::Ready && visible);
        runtime_state.resident_tile_count = u32::from(entry.state == StaticRasterState::Ready);
        runtime_state.pending_count = u32::from(entry.state == StaticRasterState::Loading);
        runtime_state.inflight_count = u32::from(entry.state == StaticRasterState::Loading);
        runtime_state.manifest_status = match entry.state {
            StaticRasterState::Loading => LayerManifestStatus::Loading,
            StaticRasterState::Ready => LayerManifestStatus::Ready,
            StaticRasterState::Failed => LayerManifestStatus::Failed,
        };
    }

    let stale_ids = cache
        .entries
        .keys()
        .copied()
        .filter(|layer_id| !active_layer_ids.contains(layer_id))
        .collect::<Vec<_>>();
    for layer_id in stale_ids {
        remove_static_entry(layer_id, &mut cache, &mut commands);
    }
}

fn ensure_static_entry(
    layer: &LayerSpec,
    asset_server: &AssetServer,
    cache: &mut StaticRasterCache,
    commands: &mut Commands,
) {
    let Some(url) = layer.static_image_url() else {
        remove_static_entry(layer.id, cache, commands);
        return;
    };
    let needs_reload = cache
        .entries
        .get(&layer.id)
        .map(|entry| entry.url != url)
        .unwrap_or(true);
    if !needs_reload {
        return;
    }
    remove_static_entry(layer.id, cache, commands);
    let handle: Handle<Image> = asset_server.load(url.clone());
    cache.entries.insert(
        layer.id,
        StaticRasterEntry {
            url,
            handle,
            entity: None,
            material: None,
            state: StaticRasterState::Loading,
            exact_quad: false,
            sprite_rect: None,
        },
    );
}

fn remove_static_entry(layer_id: LayerId, cache: &mut StaticRasterCache, commands: &mut Commands) {
    let Some(entry) = cache.entries.remove(&layer_id) else {
        return;
    };
    if let Some(entity) = entry.entity {
        commands.entity(entity).despawn();
    }
}

fn spawn_static_entity(
    layer: &LayerSpec,
    entry: &mut StaticRasterEntry,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
    map_to_world: MapToWorld,
    depth: f32,
    opacity: f32,
    visible: bool,
) -> anyhow::Result<()> {
    let world_transform = layer
        .world_transform(map_to_world)
        .ok_or_else(|| anyhow::anyhow!("missing world transform"))?;
    let layer_bounds = layer
        .static_layer_bounds(map_to_world)
        .ok_or_else(|| anyhow::anyhow!("missing static layer bounds"))?;
    if needs_affine_quad(layer, world_transform) {
        let mesh = static_layer_quad_mesh(layer_bounds, world_transform)
            .ok_or_else(|| anyhow::anyhow!("failed to build static quad mesh"))?;
        let mesh_handle = meshes.add(mesh);
        let material_handle = materials.add(ColorMaterial {
            texture: Some(entry.handle.clone()),
            color: Color::srgba(1.0, 1.0, 1.0, opacity),
            ..default()
        });
        let entity = commands
            .spawn((
                World2dRenderEntity,
                world_2d_layers(),
                Mesh2d(mesh_handle),
                MeshMaterial2d(material_handle.clone()),
                Transform::from_translation(Vec3::new(0.0, 0.0, depth)),
                if visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                },
            ))
            .id();
        entry.entity = Some(entity);
        entry.material = Some(material_handle);
        entry.exact_quad = true;
        entry.sprite_rect = None;
        return Ok(());
    }

    let (x0, y0, w, h) = static_layer_world_rect(layer_bounds, world_transform)
        .ok_or_else(|| anyhow::anyhow!("failed to build static sprite rect"))?;
    let entity = commands
        .spawn((
            World2dRenderEntity,
            world_2d_layers(),
            Sprite {
                image: entry.handle.clone(),
                custom_size: Some(Vec2::new(w, h)),
                color: Color::srgba(1.0, 1.0, 1.0, opacity),
                ..default()
            },
            Transform::from_translation(Vec3::new(x0 + w * 0.5, y0 + h * 0.5, depth)),
            if visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
        ))
        .id();
    entry.entity = Some(entity);
    entry.material = None;
    entry.exact_quad = false;
    entry.sprite_rect = Some((x0, y0, w, h));
    Ok(())
}

fn update_static_entity(
    entry: &mut StaticRasterEntry,
    commands: &mut Commands,
    materials: &mut Assets<ColorMaterial>,
    opacity: f32,
    visible: bool,
    depth: f32,
) {
    let Some(entity) = entry.entity else {
        return;
    };
    commands.entity(entity).insert(if visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    });
    if entry.exact_quad {
        commands
            .entity(entity)
            .insert(Transform::from_translation(Vec3::new(0.0, 0.0, depth)));
        if let Some(material_handle) = entry.material.as_ref() {
            if let Some(material) = materials.get_mut(material_handle) {
                material.color = Color::srgba(1.0, 1.0, 1.0, opacity);
            }
        }
        return;
    }
    let Some((x0, y0, w, h)) = entry.sprite_rect else {
        return;
    };
    commands.entity(entity).insert((
        Sprite {
            image: entry.handle.clone(),
            custom_size: Some(Vec2::new(w, h)),
            color: Color::srgba(1.0, 1.0, 1.0, opacity),
            ..default()
        },
        Transform::from_translation(Vec3::new(x0 + w * 0.5, y0 + h * 0.5, depth)),
    ));
}

fn needs_affine_quad(layer: &LayerSpec, world_transform: WorldTransform) -> bool {
    if layer.render_kind() == crate::map::layers::LayerRenderKind::IdentitySprite {
        return false;
    }
    let affine = world_transform.layer_to_world;
    affine.b.abs() > AFFINE_QUAD_EPS
        || affine.c.abs() > AFFINE_QUAD_EPS
        || affine.a < 0.0
        || affine.d < 0.0
        || matches!(layer.transform, LayerTransform::AffineToMap(_))
}

fn static_layer_quad_mesh(
    layer_bounds: LayerRect,
    world_transform: WorldTransform,
) -> Option<Mesh> {
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::Indices;

    let world = layer_bounds
        .corners()
        .map(|corner| world_transform.layer_to_world(corner));
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

fn static_layer_world_rect(
    layer_bounds: LayerRect,
    world_transform: WorldTransform,
) -> Option<(f32, f32, f32, f32)> {
    let world_corners = layer_bounds
        .corners()
        .map(|corner| world_transform.layer_to_world(corner));
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
    if !(max_x > min_x && max_z > min_z) {
        return None;
    }
    Some((
        min_x as f32,
        min_z as f32,
        (max_x - min_x) as f32,
        (max_z - min_z) as f32,
    ))
}
