use std::collections::{HashMap, HashSet};

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::mesh::Indices;
use bevy::prelude::{ColorMaterial, Mesh, Mesh2d, MeshMaterial2d};
use bevy::render::render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;

use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::exact_lookup::{
    ensure_exact_lookup_request, poll_exact_lookup_requests, ExactLookupCache, ExactLookupStatus,
    PendingExactLookups,
};
use crate::map::field_metadata::{
    ensure_field_metadata_request, poll_field_metadata_requests, FieldMetadataCache,
    PendingFieldMetadata,
};
use crate::map::field_view::{loaded_field_layer, FieldLayerView, LoadedFieldLayer};
use crate::map::layers::{LayerId, LayerManifestStatus, LayerRegistry, LayerRuntime, LodPolicy};
use crate::map::raster::cache::{clip_mask_allows_world_point, clip_mask_state_revision};
use crate::map::raster::{RasterTileCache, TileKey};
use crate::map::render::tile_z;
use crate::map::spaces::layer_transform::{LayerTransform, TileSpace};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::{LayerPoint, MapPoint, MapRect, WorldPoint, WorldRect};
use crate::plugins::api::{HoverState, LayerEffectiveFilterState, ZoneMembershipFilter};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::points::EvidenceZoneFilter;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};
use crate::plugins::vector_layers::VectorLayerRuntime;
use crate::prelude::*;

const FIELD_LAYER_MAP_VERSION: u64 = 0;
const FIELD_TILE_MARGIN: i32 = 1;
const FIELD_HIGHLIGHT_Z_BIAS: f32 = 0.0005;
const FIELD_HOVER_HIGHLIGHT_RGB: [u8; 3] = [48, 255, 96];

pub struct FieldTileLayersPlugin;

impl Plugin for FieldTileLayersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExactLookupCache>()
            .init_resource::<PendingExactLookups>()
            .init_resource::<FieldMetadataCache>()
            .init_resource::<PendingFieldMetadata>()
            .init_resource::<FieldTileRuntime>()
            .add_systems(
                PostUpdate,
                (sync_exact_lookup_cache, update_field_tile_layer_visuals).chain(),
            );
    }
}

fn should_render_field_layer_visual(layer: &crate::map::layers::LayerSpec) -> bool {
    layer.field_url().is_some()
}

#[derive(Resource, Default)]
struct FieldTileRuntime {
    layers: HashMap<LayerId, FieldTileLayerCache>,
}

#[derive(Default)]
struct FieldTileLayerCache {
    current_z: Option<i32>,
    use_counter: u64,
    tiles: HashMap<TileKey, FieldTileEntry>,
    highlight_overlay: Option<FieldHighlightOverlayEntry>,
}

struct FieldTileEntry {
    base_entity: Entity,
    base_image: Handle<Image>,
    visual_revision: u64,
    last_used: u64,
}

struct FieldHighlightOverlayEntry {
    entity_2d: Entity,
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
    highlighted_field_id: Option<u32>,
    opacity: f32,
}

#[derive(Clone, Copy, Debug)]
struct FieldTileBounds {
    min_tx: i32,
    max_tx: i32,
    min_ty: i32,
    max_ty: i32,
    z: i32,
}

#[derive(Clone, Copy, Debug)]
struct FieldLevelCandidate {
    z: i32,
    visible_count: usize,
}

struct FieldTileVisualContext<'a> {
    filter: &'a ZoneMembershipFilter,
    clip_mask_layer: Option<LayerId>,
    layer_registry: &'a LayerRegistry,
    layer_runtime: &'a LayerRuntime,
    exact_lookups: &'a ExactLookupCache,
    tile_cache: &'a RasterTileCache,
    vector_runtime: &'a VectorLayerRuntime,
    map_version: Option<&'a str>,
}

fn sync_exact_lookup_cache(
    layer_registry: Res<LayerRegistry>,
    mut layer_runtime: ResMut<LayerRuntime>,
    mut exact_lookups: ResMut<ExactLookupCache>,
    mut pending_lookups: ResMut<PendingExactLookups>,
    mut field_metadata: ResMut<FieldMetadataCache>,
    mut pending_field_metadata: ResMut<PendingFieldMetadata>,
) {
    poll_exact_lookup_requests(&mut exact_lookups, &mut pending_lookups);
    poll_field_metadata_requests(&mut field_metadata, &mut pending_field_metadata);

    let active_layer_ids = layer_registry
        .ordered()
        .iter()
        .filter(|layer| layer.field_url().is_some())
        .map(|layer| layer.id)
        .collect::<HashSet<_>>();

    for layer in layer_registry.ordered() {
        let Some(url) = layer.field_url() else {
            if layer.field_metadata_url().is_some() {
                ensure_field_metadata_request(
                    layer,
                    &mut field_metadata,
                    &mut pending_field_metadata,
                );
            }
            continue;
        };
        ensure_exact_lookup_request(layer, &mut exact_lookups, &mut pending_lookups);
        ensure_field_metadata_request(layer, &mut field_metadata, &mut pending_field_metadata);

        if let Some(runtime_state) = layer_runtime.get_mut(layer.id) {
            match exact_lookups.status(layer.id, &url) {
                ExactLookupStatus::Missing => {
                    runtime_state.manifest_status = LayerManifestStatus::Missing;
                    runtime_state.resident_tile_count = 0;
                    runtime_state.pending_count = 0;
                    runtime_state.inflight_count = 0;
                }
                ExactLookupStatus::Loading => {
                    runtime_state.manifest_status = LayerManifestStatus::Loading;
                    runtime_state.resident_tile_count = 0;
                    runtime_state.pending_count = 1;
                    runtime_state.inflight_count = 1;
                }
                ExactLookupStatus::Ready => {
                    runtime_state.manifest_status = LayerManifestStatus::Ready;
                    runtime_state.pending_count = 0;
                    runtime_state.inflight_count = 0;
                }
                ExactLookupStatus::Failed => {
                    runtime_state.manifest_status = LayerManifestStatus::Failed;
                    runtime_state.resident_tile_count = 0;
                    runtime_state.pending_count = 0;
                    runtime_state.inflight_count = 0;
                }
            }
        }
    }

    let stale_lookup_ids = exact_lookups
        .layer_ids()
        .into_iter()
        .filter(|layer_id| !active_layer_ids.contains(layer_id))
        .collect::<Vec<_>>();
    for layer_id in stale_lookup_ids {
        exact_lookups.remove_layer(layer_id);
    }

    let stale_pending_ids = pending_lookups
        .layer_ids()
        .into_iter()
        .filter(|layer_id| !active_layer_ids.contains(layer_id))
        .collect::<Vec<_>>();
    for layer_id in stale_pending_ids {
        pending_lookups.remove_layer(layer_id);
    }

    let active_metadata_ids = layer_registry
        .ordered()
        .iter()
        .filter(|layer| layer.field_metadata_url().is_some())
        .map(|layer| layer.id)
        .collect::<HashSet<_>>();
    let stale_metadata_ids = field_metadata
        .layer_ids()
        .into_iter()
        .filter(|layer_id| !active_metadata_ids.contains(layer_id))
        .collect::<Vec<_>>();
    for layer_id in stale_metadata_ids {
        field_metadata.remove_layer(layer_id);
    }
    let stale_pending_metadata_ids = pending_field_metadata
        .layer_ids()
        .into_iter()
        .filter(|layer_id| !active_metadata_ids.contains(layer_id))
        .collect::<Vec<_>>();
    for layer_id in stale_pending_metadata_ids {
        pending_field_metadata.remove_layer(layer_id);
    }
}

fn update_field_tile_layer_visuals(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials_2d: ResMut<Assets<ColorMaterial>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &Transform), With<Map2dCamera>>,
    view_mode: Res<ViewModeState>,
    layer_registry: Res<LayerRegistry>,
    mut layer_runtime: ResMut<LayerRuntime>,
    exact_lookups: Res<ExactLookupCache>,
    tile_cache: Res<RasterTileCache>,
    vector_runtime: Res<VectorLayerRuntime>,
    layer_filters: Res<LayerEffectiveFilterState>,
    hover: Res<HoverState>,
    mut field_runtime: ResMut<FieldTileRuntime>,
) {
    let active_exact_layers = layer_registry
        .ordered()
        .iter()
        .filter(|layer| should_render_field_layer_visual(layer))
        .map(|layer| layer.id)
        .collect::<HashSet<_>>();
    cleanup_stale_field_layers(
        &mut commands,
        &mut images,
        &mut meshes,
        &mut materials_2d,
        &mut field_runtime,
        &active_exact_layers,
    );

    if view_mode.mode != ViewMode::Map2D {
        hide_all_field_layers(&mut commands, &field_runtime);
        return;
    }

    let Ok(window) = windows.single() else {
        hide_all_field_layers(&mut commands, &field_runtime);
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.single() else {
        hide_all_field_layers(&mut commands, &field_runtime);
        return;
    };
    let Some(view_world) = view_rect(camera, camera_transform, window) else {
        hide_all_field_layers(&mut commands, &field_runtime);
        return;
    };
    let map_to_world = MapToWorld::default();
    let visible_map_rect = world_rect_to_clamped_map_rect(view_world, map_to_world);

    for layer in layer_registry.ordered() {
        let Some(runtime_state) = layer_runtime.get(layer.id) else {
            continue;
        };
        let runtime_visible = runtime_state.visible;
        let runtime_z_base = runtime_state.z_base;
        let runtime_opacity = runtime_state.opacity.clamp(0.0, 1.0);
        let Some(url) = layer.field_url() else {
            hide_field_layer(&mut commands, field_runtime.layers.get(&layer.id));
            continue;
        };
        if !should_render_field_layer_visual(layer) {
            hide_field_layer(&mut commands, field_runtime.layers.get(&layer.id));
            continue;
        }
        let Some(field) = loaded_field_layer(layer, &exact_lookups) else {
            hide_field_layer(&mut commands, field_runtime.layers.get(&layer.id));
            if let Some(runtime_state) = layer_runtime.get_mut(layer.id) {
                runtime_state.visible_tile_count = 0;
                runtime_state.resident_tile_count = 0;
            }
            continue;
        };
        let clip_mask_layer = layer_runtime.clip_mask_layer(layer.id);
        let inactive_filter = EvidenceZoneFilter::default();
        let zone_filter = layer_filters
            .zone_membership_filter(layer.key.as_str())
            .unwrap_or(&inactive_filter);
        let clip_mask_revision = clip_mask_state_revision(
            &layer_registry,
            &layer_runtime,
            clip_mask_layer,
            zone_filter,
        );
        if exact_lookups.get(layer.id, &url).is_none()
            || !runtime_visible
            || !matches!(layer.transform, LayerTransform::IdentityMapSpace)
        {
            hide_field_layer(&mut commands, field_runtime.layers.get(&layer.id));
            if let Some(runtime_state) = layer_runtime.get_mut(layer.id) {
                runtime_state.visible_tile_count = 0;
            }
            continue;
        }

        let generated_max_level =
            generated_field_max_level(field.width(), field.height(), layer.tile_px.max(1));
        let layer_cache = field_runtime.layers.entry(layer.id).or_default();
        let visual_revision =
            field_tile_visual_revision(layer, zone_filter, clip_mask_layer, clip_mask_revision);
        let visual_context = FieldTileVisualContext {
            filter: zone_filter,
            clip_mask_layer,
            layer_registry: &layer_registry,
            layer_runtime: &layer_runtime,
            exact_lookups: &exact_lookups,
            tile_cache: &tile_cache,
            vector_runtime: &vector_runtime,
            map_version: layer_registry.map_version_id(),
        };
        let z = choose_field_tile_level(
            visible_map_rect,
            layer.tile_px.max(1),
            generated_max_level,
            layer_cache.current_z,
            &layer.lod_policy,
        );
        layer_cache.current_z = Some(z);
        let Some(bounds) = visible_tile_bounds(
            visible_map_rect,
            field.width(),
            field.height(),
            layer.tile_px.max(1),
            z,
            FIELD_TILE_MARGIN,
        ) else {
            hide_field_layer(&mut commands, Some(layer_cache));
            if let Some(runtime_state) = layer_runtime.get_mut(layer.id) {
                runtime_state.visible_tile_count = 0;
            }
            continue;
        };
        let hovered_field_id = hovered_field_id_for_layer(hover.info.as_ref(), layer.key.as_str());
        let tile_space = TileSpace::new(layer.tile_px.max(1), layer.y_flip);
        let max_level_u8 = generated_max_level.min(i32::from(u8::MAX)) as u8;
        let mut active_keys = HashSet::new();
        let mut visible_count = 0_u32;

        for ty in bounds.min_ty..=bounds.max_ty {
            for tx in bounds.min_tx..=bounds.max_tx {
                let key = TileKey {
                    layer: layer.id,
                    map_version: FIELD_LAYER_MAP_VERSION,
                    z: bounds.z,
                    tx,
                    ty,
                };
                active_keys.insert(key);
                if !ensure_field_tile_entry(
                    &mut commands,
                    &mut images,
                    layer_cache,
                    key,
                    layer,
                    field,
                    visual_revision,
                    &visual_context,
                ) {
                    continue;
                }
                let Some(entry) = layer_cache.tiles.get_mut(&key) else {
                    continue;
                };
                layer_cache.use_counter = layer_cache.use_counter.wrapping_add(1);
                entry.last_used = layer_cache.use_counter;
                let Some((x0, y0, w, h)) = field_tile_world_rect(&key, tile_space, map_to_world)
                else {
                    continue;
                };
                let depth = tile_z(runtime_z_base, max_level_u8, bounds.z);
                commands.entity(entry.base_entity).insert((
                    Visibility::Visible,
                    Sprite {
                        image: entry.base_image.clone(),
                        custom_size: Some(Vec2::new(w, h)),
                        color: Color::srgba(1.0, 1.0, 1.0, runtime_opacity),
                        ..default()
                    },
                    Transform::from_translation(Vec3::new(x0 + w * 0.5, y0 + h * 0.5, depth)),
                ));
                visible_count = visible_count.saturating_add(1);
            }
        }

        update_field_layer_highlight_overlay(
            &mut commands,
            &mut meshes,
            &mut materials_2d,
            layer_cache,
            field,
            map_to_world,
            hovered_field_id,
            tile_z(runtime_z_base, max_level_u8, bounds.z) + FIELD_HIGHLIGHT_Z_BIAS,
            runtime_opacity,
        );
        hide_inactive_tiles(&mut commands, layer_cache, &active_keys);
        evict_excess_tiles(
            &mut commands,
            &mut images,
            layer_cache,
            &active_keys,
            layer.lod_policy.max_resident_tiles.max(active_keys.len()),
        );
        if let Some(runtime_state) = layer_runtime.get_mut(layer.id) {
            runtime_state.current_base_lod = u8::try_from(bounds.z).ok();
            runtime_state.current_detail_lod = None;
            runtime_state.visible_tile_count = visible_count;
            runtime_state.resident_tile_count = layer_cache.tiles.len() as u32;
            runtime_state.pending_count = 0;
            runtime_state.inflight_count = 0;
            runtime_state.manifest_status = LayerManifestStatus::Ready;
        }
    }
}

fn ensure_field_tile_entry(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    layer_cache: &mut FieldTileLayerCache,
    key: TileKey,
    layer: &crate::map::layers::LayerSpec,
    field: LoadedFieldLayer<'_>,
    visual_revision: u64,
    visual_context: &FieldTileVisualContext<'_>,
) -> bool {
    if let Some(entry) = layer_cache.tiles.get_mut(&key) {
        if entry.visual_revision == visual_revision {
            return true;
        }
        let Some(base_image) = render_field_tile_image(field, layer, key, visual_context) else {
            return false;
        };
        if let Some(existing) = images.get_mut(&entry.base_image) {
            *existing = base_image;
        } else {
            entry.base_image = images.add(base_image);
        }
        entry.visual_revision = visual_revision;
        return true;
    }
    let Some(base_image) = render_field_tile_image(field, layer, key, visual_context) else {
        return false;
    };
    let base_handle = images.add(base_image);
    let base_entity = commands
        .spawn((
            World2dRenderEntity,
            world_2d_layers(),
            Sprite {
                image: base_handle.clone(),
                ..default()
            },
            Transform::default(),
            Visibility::Hidden,
        ))
        .id();
    layer_cache.tiles.insert(
        key,
        FieldTileEntry {
            base_entity,
            base_image: base_handle,
            visual_revision,
            last_used: 0,
        },
    );
    true
}

fn cleanup_stale_field_layers(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    meshes: &mut Assets<Mesh>,
    materials_2d: &mut Assets<ColorMaterial>,
    field_runtime: &mut FieldTileRuntime,
    active_layer_ids: &HashSet<LayerId>,
) {
    let stale_ids = field_runtime
        .layers
        .keys()
        .filter(|layer_id| !active_layer_ids.contains(layer_id))
        .copied()
        .collect::<Vec<_>>();
    for layer_id in stale_ids {
        if let Some(layer_cache) = field_runtime.layers.remove(&layer_id) {
            despawn_field_tile_layer(commands, images, meshes, materials_2d, layer_cache);
        }
    }
}

fn hide_inactive_tiles(
    commands: &mut Commands,
    layer_cache: &mut FieldTileLayerCache,
    active_keys: &HashSet<TileKey>,
) {
    for (key, entry) in &layer_cache.tiles {
        if active_keys.contains(key) {
            continue;
        }
        commands
            .entity(entry.base_entity)
            .insert(Visibility::Hidden);
    }
}

fn evict_excess_tiles(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    layer_cache: &mut FieldTileLayerCache,
    active_keys: &HashSet<TileKey>,
    max_resident_tiles: usize,
) {
    if layer_cache.tiles.len() <= max_resident_tiles {
        return;
    }
    let mut inactive = layer_cache
        .tiles
        .iter()
        .filter(|(key, _)| !active_keys.contains(key))
        .map(|(key, entry)| (*key, entry.last_used))
        .collect::<Vec<_>>();
    inactive.sort_by_key(|(_, last_used)| *last_used);
    let excess = layer_cache.tiles.len().saturating_sub(max_resident_tiles);
    for (key, _) in inactive.into_iter().take(excess) {
        if let Some(entry) = layer_cache.tiles.remove(&key) {
            despawn_field_tile_entry(commands, images, entry);
        }
    }
}

fn hide_all_field_layers(commands: &mut Commands, field_runtime: &FieldTileRuntime) {
    for layer_cache in field_runtime.layers.values() {
        hide_field_layer(commands, Some(layer_cache));
    }
}

fn hide_field_layer(commands: &mut Commands, layer_cache: Option<&FieldTileLayerCache>) {
    let Some(layer_cache) = layer_cache else {
        return;
    };
    for entry in layer_cache.tiles.values() {
        commands
            .entity(entry.base_entity)
            .insert(Visibility::Hidden);
    }
    if let Some(overlay) = &layer_cache.highlight_overlay {
        commands
            .entity(overlay.entity_2d)
            .insert(Visibility::Hidden);
    }
}

fn despawn_field_tile_layer(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    meshes: &mut Assets<Mesh>,
    materials_2d: &mut Assets<ColorMaterial>,
    layer_cache: FieldTileLayerCache,
) {
    for entry in layer_cache.tiles.into_values() {
        despawn_field_tile_entry(commands, images, entry);
    }
    if let Some(overlay) = layer_cache.highlight_overlay {
        commands.entity(overlay.entity_2d).despawn();
        meshes.remove(overlay.mesh.id());
        materials_2d.remove(overlay.material.id());
    }
}

fn despawn_field_tile_entry(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    entry: FieldTileEntry,
) {
    commands.entity(entry.base_entity).despawn();
    images.remove(entry.base_image.id());
}

fn generated_field_max_level(width: u16, height: u16, tile_px: u32) -> i32 {
    let tile_px = tile_px.max(1);
    let max_dim = u32::from(width.max(height));
    let mut span = tile_px;
    let mut z = 0_i32;
    while span < max_dim {
        span = span.saturating_mul(2);
        z += 1;
        if z >= 30 {
            break;
        }
    }
    z
}

fn choose_field_tile_level(
    visible_map_rect: MapRect,
    tile_px: u32,
    max_level: i32,
    current_z: Option<i32>,
    policy: &LodPolicy,
) -> i32 {
    let mut candidates = Vec::new();
    for z in 0..=max_level.max(0) {
        let span = f64::from(tile_px.max(1)) * f64::from(1_u32 << z as u32);
        if span <= 0.0 {
            continue;
        }
        let width = (visible_map_rect.max.x - visible_map_rect.min.x).max(1.0);
        let height = (visible_map_rect.max.y - visible_map_rect.min.y).max(1.0);
        let count_x = (width / span).ceil().max(1.0) as usize;
        let count_y = (height / span).ceil().max(1.0) as usize;
        candidates.push(FieldLevelCandidate {
            z,
            visible_count: count_x.saturating_mul(count_y),
        });
    }
    if candidates.is_empty() {
        return 0;
    }

    let target = policy.target_tiles.max(1);
    let hi = policy.hysteresis_hi.max(target as f32) as usize;
    let lo = policy.hysteresis_lo.min(target as f32) as usize;
    let ideal = candidates
        .iter()
        .find(|candidate| candidate.visible_count <= target)
        .copied()
        .unwrap_or_else(|| *candidates.last().expect("non-empty"));

    let Some(current_z) = current_z else {
        return ideal.z;
    };
    let Some(current) = candidates
        .iter()
        .find(|candidate| candidate.z == current_z)
        .copied()
    else {
        return ideal.z;
    };
    if current.visible_count > hi {
        return candidates
            .iter()
            .filter(|candidate| candidate.z >= current_z)
            .find(|candidate| candidate.visible_count <= target)
            .copied()
            .or_else(|| {
                candidates
                    .iter()
                    .rfind(|candidate| candidate.z >= current_z)
                    .copied()
            })
            .unwrap_or(ideal)
            .z;
    }
    if current.visible_count < lo {
        return candidates
            .iter()
            .filter(|candidate| candidate.z <= current_z)
            .find(|candidate| candidate.visible_count <= target)
            .copied()
            .unwrap_or(current)
            .z;
    }
    current.z
}

fn visible_tile_bounds(
    visible_map_rect: MapRect,
    field_width: u16,
    field_height: u16,
    tile_px: u32,
    z: i32,
    margin_tiles: i32,
) -> Option<FieldTileBounds> {
    let tile_space = TileSpace::new(tile_px.max(1), false);
    let span = tile_space.tile_span_px(z)?;
    if span <= 0.0 {
        return None;
    }
    let max_tx = ((f64::from(field_width) / span).ceil() as i32 - 1).max(0);
    let max_ty = ((f64::from(field_height) / span).ceil() as i32 - 1).max(0);
    let mut min_tx = (visible_map_rect.min.x / span).floor() as i32 - margin_tiles;
    let mut max_tx_visible = (visible_map_rect.max.x / span).floor() as i32 + margin_tiles;
    let mut min_ty = (visible_map_rect.min.y / span).floor() as i32 - margin_tiles;
    let mut max_ty_visible = (visible_map_rect.max.y / span).floor() as i32 + margin_tiles;
    min_tx = min_tx.clamp(0, max_tx);
    max_tx_visible = max_tx_visible.clamp(0, max_tx);
    min_ty = min_ty.clamp(0, max_ty);
    max_ty_visible = max_ty_visible.clamp(0, max_ty);
    (min_tx <= max_tx_visible && min_ty <= max_ty_visible).then_some(FieldTileBounds {
        min_tx,
        max_tx: max_tx_visible,
        min_ty,
        max_ty: max_ty_visible,
        z,
    })
}

fn render_field_tile_image(
    field: LoadedFieldLayer<'_>,
    layer: &crate::map::layers::LayerSpec,
    key: TileKey,
    visual_context: &FieldTileVisualContext<'_>,
) -> Option<Image> {
    let (
        source_origin_x,
        source_origin_y,
        source_width,
        source_height,
        output_width,
        output_height,
    ) = tile_render_dims(field, layer, key)?;
    let chunk = if layer.is_zone_mask_visual_layer() && visual_context.clip_mask_layer.is_none() {
        field.render_rgba_chunk_with(
            source_origin_x,
            source_origin_y,
            source_width,
            source_height,
            output_width,
            output_height,
            |field_id| {
                if field_id == 0 {
                    return [0, 0, 0, 0];
                }
                if visual_context.filter.active
                    && !visual_context.filter.zone_rgbs.contains(&field_id)
                {
                    return [0, 0, 0, 0];
                }
                [
                    ((field_id >> 16) & 0xff) as u8,
                    ((field_id >> 8) & 0xff) as u8,
                    (field_id & 0xff) as u8,
                    255,
                ]
            },
        )
    } else {
        field.render_rgba_chunk(
            source_origin_x,
            source_origin_y,
            source_width,
            source_height,
            output_width,
            output_height,
        )
    };
    let mut image = image_from_chunk(chunk.width(), chunk.height(), chunk.into_data());
    apply_field_tile_visual_filters(
        field,
        layer,
        &mut image,
        source_origin_x,
        source_origin_y,
        source_width,
        source_height,
        visual_context,
    );
    Some(image)
}

fn field_tile_visual_revision(
    layer: &crate::map::layers::LayerSpec,
    filter: &ZoneMembershipFilter,
    clip_mask_layer: Option<LayerId>,
    clip_mask_revision: u64,
) -> u64 {
    if !layer.is_zone_mask_visual_layer() {
        return 0;
    }
    let mut revision = clip_mask_revision
        .wrapping_mul(2)
        .wrapping_add(u64::from(clip_mask_layer.is_some()));
    if filter.active {
        revision = revision
            .wrapping_mul(31)
            .wrapping_add(filter.revision.wrapping_add(1));
    }
    revision
}

fn apply_field_tile_visual_filters(
    field: LoadedFieldLayer<'_>,
    layer: &crate::map::layers::LayerSpec,
    image: &mut Image,
    source_origin_x: i32,
    source_origin_y: i32,
    source_width: u32,
    source_height: u32,
    visual_context: &FieldTileVisualContext<'_>,
) {
    if !layer.is_zone_mask_visual_layer() {
        return;
    }
    if visual_context.clip_mask_layer.is_none() {
        return;
    }
    let Some(target_transform) = layer.world_transform(MapToWorld::default()) else {
        return;
    };
    let Some(data) = image.data.as_mut() else {
        return;
    };
    let output_width = image.texture_descriptor.size.width.max(1) as usize;
    let output_height = image.texture_descriptor.size.height.max(1) as usize;
    let px_scale_x = f64::from(source_width.max(1)) / output_width as f64;
    let px_scale_y = f64::from(source_height.max(1)) / output_height as f64;

    for row_idx in 0..output_height {
        for col_idx in 0..output_width {
            let pixel_offset = (row_idx * output_width + col_idx) * 4;
            let layer_point = LayerPoint::new(
                f64::from(source_origin_x) + (col_idx as f64 + 0.5) * px_scale_x,
                f64::from(source_origin_y) + (row_idx as f64 + 0.5) * px_scale_y,
            );
            let field_id = field.field_id_at_layer_point(layer_point).unwrap_or(0);
            if field_id == 0 {
                data[pixel_offset + 3] = 0;
                continue;
            }
            let zone_rgb = field
                .rgb_at_layer_point(layer_point)
                .map(|rgb| rgb.to_u32());
            if visual_context.filter.active
                && !zone_rgb.is_some_and(|rgb| visual_context.filter.zone_rgbs.contains(&rgb))
            {
                data[pixel_offset + 3] = 0;
                continue;
            }
            let Some(mask_layer_id) = visual_context.clip_mask_layer else {
                continue;
            };
            let world_point = target_transform.layer_to_world(layer_point);
            let Some(allowed) = clip_mask_allows_world_point(
                mask_layer_id,
                world_point,
                visual_context.layer_registry,
                visual_context.layer_runtime,
                visual_context.exact_lookups,
                visual_context.tile_cache,
                visual_context.vector_runtime,
                visual_context.filter,
                visual_context.map_version,
            ) else {
                continue;
            };
            if !allowed {
                data[pixel_offset + 3] = 0;
            }
        }
    }
}

fn tile_render_dims(
    field: LoadedFieldLayer<'_>,
    layer: &crate::map::layers::LayerSpec,
    key: TileKey,
) -> Option<(i32, i32, u32, u32, u16, u16)> {
    if !matches!(layer.transform, LayerTransform::IdentityMapSpace) || key.z < 0 {
        return None;
    }
    let scale = 1_u32.checked_shl(key.z as u32)?;
    let source_span = layer.tile_px.checked_mul(scale)?;
    if source_span == 0 {
        return None;
    }
    let source_origin_x = key.tx.checked_mul(source_span as i32)?;
    let source_origin_y = key.ty.checked_mul(source_span as i32)?;
    let visible_source_width =
        (i32::from(field.width()) - source_origin_x).clamp(0, source_span as i32) as u32;
    let visible_source_height =
        (i32::from(field.height()) - source_origin_y).clamp(0, source_span as i32) as u32;
    if visible_source_width == 0 || visible_source_height == 0 {
        return None;
    }
    let output_width = visible_source_width.div_ceil(scale) as u16;
    let output_height = visible_source_height.div_ceil(scale) as u16;
    Some((
        source_origin_x,
        source_origin_y,
        visible_source_width,
        visible_source_height,
        output_width,
        output_height,
    ))
}

fn field_tile_world_rect(
    key: &TileKey,
    tile_space: TileSpace,
    map_to_world: MapToWorld,
) -> Option<(f32, f32, f32, f32)> {
    let span = tile_space.tile_span_px(key.z)?;
    let x0 = (key.tx as f64 * span).clamp(0.0, f64::from(map_to_world.image_size_x));
    let y0 = (key.ty as f64 * span).clamp(0.0, f64::from(map_to_world.image_size_y));
    let x1 = (x0 + span).clamp(0.0, f64::from(map_to_world.image_size_x));
    let y1 = (y0 + span).clamp(0.0, f64::from(map_to_world.image_size_y));
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let min = map_to_world.map_to_world(MapPoint::new(x0, y0));
    let max = map_to_world.map_to_world(MapPoint::new(x1, y1));
    let min_x = min.x.min(max.x) as f32;
    let max_x = min.x.max(max.x) as f32;
    let min_z = min.z.min(max.z) as f32;
    let max_z = min.z.max(max.z) as f32;
    Some((min_x, min_z, max_x - min_x, max_z - min_z))
}

fn image_from_chunk(width: u16, height: u16, data: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: u32::from(width),
            height: u32::from(height),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = bevy::image::ImageSampler::nearest();
    image
}

fn update_field_layer_highlight_overlay(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials_2d: &mut Assets<ColorMaterial>,
    layer_cache: &mut FieldTileLayerCache,
    field: LoadedFieldLayer<'_>,
    map_to_world: MapToWorld,
    hovered_field_id: Option<u32>,
    depth: f32,
    opacity: f32,
) {
    let Some(highlight_field_id) = hovered_field_id.filter(|field_id| *field_id != 0) else {
        if let Some(overlay) = &mut layer_cache.highlight_overlay {
            overlay.highlighted_field_id = None;
            commands
                .entity(overlay.entity_2d)
                .insert(Visibility::Hidden);
        }
        return;
    };

    let Some(overlay) =
        ensure_field_highlight_overlay_entry(commands, meshes, materials_2d, layer_cache, opacity)
    else {
        return;
    };

    if overlay.highlighted_field_id != Some(highlight_field_id) {
        let Some(mesh) = build_field_highlight_mesh(field, map_to_world, highlight_field_id) else {
            overlay.highlighted_field_id = None;
            commands
                .entity(overlay.entity_2d)
                .insert(Visibility::Hidden);
            return;
        };
        replace_highlight_mesh(meshes, &mut overlay.mesh, mesh);
        overlay.highlighted_field_id = Some(highlight_field_id);
    }

    if (overlay.opacity - opacity).abs() > f32::EPSILON {
        replace_highlight_material_color(materials_2d, &mut overlay.material, opacity);
        overlay.opacity = opacity;
    }

    commands.entity(overlay.entity_2d).insert((
        Visibility::Visible,
        Transform::from_translation(Vec3::new(0.0, 0.0, depth)),
    ));
}

fn ensure_field_highlight_overlay_entry<'a>(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials_2d: &mut Assets<ColorMaterial>,
    layer_cache: &'a mut FieldTileLayerCache,
    opacity: f32,
) -> Option<&'a mut FieldHighlightOverlayEntry> {
    if layer_cache.highlight_overlay.is_none() {
        let mesh = meshes.add(empty_highlight_mesh());
        let material = materials_2d.add(ColorMaterial {
            color: highlight_material_color(opacity),
            ..default()
        });
        let entity_2d = commands
            .spawn((
                World2dRenderEntity,
                world_2d_layers(),
                Mesh2d(mesh.clone()),
                MeshMaterial2d(material.clone()),
                Transform::default(),
                Visibility::Hidden,
            ))
            .id();
        layer_cache.highlight_overlay = Some(FieldHighlightOverlayEntry {
            entity_2d,
            mesh,
            material,
            highlighted_field_id: None,
            opacity,
        });
    }
    layer_cache.highlight_overlay.as_mut()
}

fn build_field_highlight_mesh(
    field: LoadedFieldLayer<'_>,
    map_to_world: MapToWorld,
    highlight_field_id: u32,
) -> Option<Mesh> {
    let mut positions = Vec::<[f32; 3]>::new();
    let mut vertex_colors = Vec::<[f32; 4]>::new();
    let mut indices = Vec::<u32>::new();

    field.for_each_merged_rect_matching(highlight_field_id, |start_y, end_y, start_x, end_x| {
        if start_x >= end_x || start_y >= end_y {
            return;
        }
        let min = map_to_world.map_to_world(MapPoint::new(start_x as f64, start_y as f64));
        let max = map_to_world.map_to_world(MapPoint::new(end_x as f64, end_y as f64));
        let min_x = min.x.min(max.x) as f32;
        let max_x = min.x.max(max.x) as f32;
        let min_z = min.z.min(max.z) as f32;
        let max_z = min.z.max(max.z) as f32;
        if max_x <= min_x || max_z <= min_z {
            return;
        }
        push_highlight_rect(
            &mut positions,
            &mut vertex_colors,
            &mut indices,
            min_x,
            max_x,
            min_z,
            max_z,
        );
    });

    if positions.is_empty() {
        return None;
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors);
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
}

fn push_highlight_rect(
    positions: &mut Vec<[f32; 3]>,
    vertex_colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    min_x: f32,
    max_x: f32,
    min_z: f32,
    max_z: f32,
) {
    let base = positions.len() as u32;
    positions.extend_from_slice(&[
        [min_x, min_z, 0.0],
        [max_x, min_z, 0.0],
        [max_x, max_z, 0.0],
        [min_x, max_z, 0.0],
    ]);
    vertex_colors.extend_from_slice(&[[1.0, 1.0, 1.0, 1.0]; 4]);
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn empty_highlight_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new());
    mesh.insert_indices(Indices::U32(Vec::new()));
    mesh
}

fn replace_highlight_mesh(meshes: &mut Assets<Mesh>, handle: &mut Handle<Mesh>, mesh: Mesh) {
    if let Some(existing) = meshes.get_mut(handle.id()) {
        *existing = mesh;
        return;
    }
    *handle = meshes.add(mesh);
}

fn replace_highlight_material_color(
    materials_2d: &mut Assets<ColorMaterial>,
    handle: &mut Handle<ColorMaterial>,
    opacity: f32,
) {
    let material = ColorMaterial {
        color: highlight_material_color(opacity),
        ..default()
    };
    if let Some(existing) = materials_2d.get_mut(handle.id()) {
        *existing = material;
        return;
    }
    *handle = materials_2d.add(material);
}

fn highlight_material_color(opacity: f32) -> Color {
    Color::srgba(
        f32::from(FIELD_HOVER_HIGHLIGHT_RGB[0]) / 255.0,
        f32::from(FIELD_HOVER_HIGHLIGHT_RGB[1]) / 255.0,
        f32::from(FIELD_HOVER_HIGHLIGHT_RGB[2]) / 255.0,
        opacity.clamp(0.0, 1.0),
    )
}

fn hovered_field_id_for_layer(
    hover: Option<&crate::plugins::api::HoverInfo>,
    layer_key: &str,
) -> Option<u32> {
    hover
        .and_then(|hover| {
            hover
                .layer_samples
                .iter()
                .find(|sample| sample.layer_id == layer_key)
        })
        .and_then(|sample| sample.field_id)
        .filter(|field_id| *field_id != 0)
}

fn world_rect_to_clamped_map_rect(world_rect: WorldRect, map_to_world: MapToWorld) -> MapRect {
    let map_min = map_to_world.world_to_map(WorldPoint::new(world_rect.min.x, world_rect.max.z));
    let map_max = map_to_world.world_to_map(WorldPoint::new(world_rect.max.x, world_rect.min.z));
    MapRect {
        min: MapPoint::new(
            map_min
                .x
                .min(map_max.x)
                .clamp(0.0, f64::from(map_to_world.image_size_x)),
            map_min
                .y
                .min(map_max.y)
                .clamp(0.0, f64::from(map_to_world.image_size_y)),
        ),
        max: MapPoint::new(
            map_min
                .x
                .max(map_max.x)
                .clamp(0.0, f64::from(map_to_world.image_size_x)),
            map_min
                .y
                .max(map_max.y)
                .clamp(0.0, f64::from(map_to_world.image_size_y)),
        ),
    }
}

fn view_rect(camera: &Camera, camera_transform: &Transform, window: &Window) -> Option<WorldRect> {
    let global = GlobalTransform::from(*camera_transform);
    let min = camera
        .viewport_to_world_2d(&global, Vec2::new(0.0, 0.0))
        .ok()?;
    let max = camera
        .viewport_to_world_2d(&global, Vec2::new(window.width(), window.height()))
        .ok()?;
    Some(WorldRect {
        min: WorldPoint::new(min.x.min(max.x) as f64, min.y.min(max.y) as f64),
        max: WorldPoint::new(min.x.max(max.x) as f64, min.y.max(max.y) as f64),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        build_field_highlight_mesh, choose_field_tile_level, generated_field_max_level,
        render_field_tile_image, should_render_field_layer_visual, visible_tile_bounds,
        FieldTileVisualContext,
    };
    use crate::map::exact_lookup::ExactLookupCache;
    use crate::map::field_view::LoadedFieldLayer;
    use crate::map::layers::{
        FieldColorMode, FieldSourceSpec, LayerId, LayerKind, LayerRegistry, LayerRuntime,
        LayerSpec, LodPolicy, PickMode,
    };
    use crate::map::raster::{RasterTileCache, TileKey};
    use crate::map::spaces::layer_transform::LayerTransform;
    use crate::map::spaces::world::MapToWorld;
    use crate::map::spaces::{MapPoint, MapRect};
    use crate::plugins::points::EvidenceZoneFilter;
    use crate::plugins::vector_layers::VectorLayerRuntime;
    use bevy::mesh::{Indices, VertexAttributeValues};
    use bevy::prelude::Mesh;
    use fishystuff_core::field::DiscreteFieldRows;
    use std::collections::HashSet;

    fn test_policy() -> LodPolicy {
        LodPolicy {
            target_tiles: 64,
            hysteresis_hi: 80.0,
            hysteresis_lo: 40.0,
            margin_tiles: 0,
            enable_refine: false,
            refine_debounce_ms: 0,
            max_detail_tiles: 0,
            max_resident_tiles: 256,
            pinned_coarse_levels: 0,
            coarse_pin_min_level: None,
            warm_margin_tiles: 1,
            protected_margin_tiles: 0,
            detail_eviction_weight: 4.0,
            max_detail_requests_while_camera_moving: 1,
            motion_suppresses_refine: true,
        }
    }

    fn layer(kind: LayerKind, field_source: bool) -> LayerSpec {
        LayerSpec {
            id: LayerId::from_raw(1),
            key: "test".to_string(),
            name: "Test".to_string(),
            visible_default: true,
            opacity_default: 1.0,
            z_base: 0.0,
            kind,
            tileset_url: String::new(),
            tile_url_template: String::new(),
            tileset_version: "v1".to_string(),
            vector_source: None,
            waypoint_source: None,
            transform: LayerTransform::IdentityMapSpace,
            tile_px: 256,
            max_level: 0,
            y_flip: false,
            field_source: field_source.then(|| FieldSourceSpec {
                url: "/fields/test.v1.bin".to_string(),
                revision: "test".to_string(),
                color_mode: FieldColorMode::DebugHash,
            }),
            field_metadata_source: None,
            lod_policy: test_policy(),
            request_weight: 1.0,
            pick_mode: PickMode::None,
            display_order: 0,
            filter_bindings: Vec::new(),
        }
    }

    #[test]
    fn zone_mask_raster_field_layers_still_render_field_visuals() {
        assert!(should_render_field_layer_visual(&layer(
            LayerKind::TiledRaster,
            true
        )));
        assert!(should_render_field_layer_visual(&layer(
            LayerKind::VectorGeoJson,
            true
        )));
        assert!(!should_render_field_layer_visual(&layer(
            LayerKind::VectorGeoJson,
            false
        )));
    }

    #[test]
    fn zone_mask_field_tiles_hide_unselected_zone_rgbs() {
        let field_rows =
            DiscreteFieldRows::from_u32_grid(2, 1, &[0x123456, 0x654321]).expect("field");
        let field = LoadedFieldLayer::from_parts(&field_rows, FieldColorMode::RgbU24);
        let mut layer = layer(LayerKind::TiledRaster, true);
        layer.key = "zone_mask".to_string();
        layer.pick_mode = PickMode::ExactTilePixel;
        layer.tile_px = 2;

        let filter = EvidenceZoneFilter {
            active: true,
            zone_rgbs: HashSet::from([0x123456]),
            revision: 7,
        };
        let layer_registry = LayerRegistry::default();
        let layer_runtime = LayerRuntime::default();
        let exact_lookups = ExactLookupCache::default();
        let tile_cache = RasterTileCache::default();
        let vector_runtime = VectorLayerRuntime::default();
        let image = render_field_tile_image(
            field,
            &layer,
            TileKey {
                layer: layer.id,
                map_version: 0,
                z: 0,
                tx: 0,
                ty: 0,
            },
            &FieldTileVisualContext {
                filter: &filter,
                clip_mask_layer: None,
                layer_registry: &layer_registry,
                layer_runtime: &layer_runtime,
                exact_lookups: &exact_lookups,
                tile_cache: &tile_cache,
                vector_runtime: &vector_runtime,
                map_version: None,
            },
        )
        .expect("image");
        let data = image.data.expect("image data");

        assert_eq!(data[3], 255);
        assert_eq!(data[7], 0);
    }

    #[test]
    fn generated_field_max_level_covers_full_map() {
        assert_eq!(generated_field_max_level(11_560, 10_540, 512), 5);
    }

    #[test]
    fn choose_field_tile_level_prefers_coarser_level_for_full_map() {
        let z = choose_field_tile_level(
            MapRect {
                min: MapPoint::new(0.0, 0.0),
                max: MapPoint::new(11_560.0, 10_540.0),
            },
            512,
            5,
            None,
            &test_policy(),
        );
        assert_eq!(z, 2);
    }

    #[test]
    fn visible_tile_bounds_clamp_to_edge_tiles() {
        let bounds = visible_tile_bounds(
            MapRect {
                min: MapPoint::new(11_200.0, 10_200.0),
                max: MapPoint::new(11_560.0, 10_540.0),
            },
            11_560,
            10_540,
            512,
            0,
            1,
        )
        .expect("bounds");
        assert_eq!(bounds.max_tx, 22);
        assert_eq!(bounds.max_ty, 20);
    }

    #[test]
    fn highlight_mesh_merges_rectangles_from_field_rows() {
        let field = DiscreteFieldRows::from_u32_grid(
            4,
            4,
            &[
                0, 7, 7, 0, //
                0, 7, 7, 0, //
                0, 7, 0, 0, //
                0, 7, 0, 0,
            ],
        )
        .expect("field");
        let view = LoadedFieldLayer::from_parts(&field, FieldColorMode::DebugHash);

        let mesh = build_field_highlight_mesh(view, MapToWorld::default(), 7).expect("mesh");
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("expected float32x3 positions");
        };
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!("expected u32 indices");
        };

        assert_eq!(positions.len(), 8);
        assert_eq!(indices.len(), 12);
    }
}
