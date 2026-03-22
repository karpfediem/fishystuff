use bevy::ecs::system::SystemParam;
use bevy::window::PrimaryWindow;

use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::layers::{LayerManifestStatus, LayerRegistry, LayerRuntime, PickMode};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::{WorldPoint, WorldRect};
use crate::map::terrain::runtime::TerrainViewEstimate;
use crate::plugins::api::{ApiBootstrapState, MapDisplayState};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::input::PanState;
use crate::plugins::points::EvidenceZoneFilter;
use crate::plugins::vector_layers::VectorLayerRuntime;
use crate::prelude::*;

use super::cache::{ZoneMaskMaterial, ZoneMaskMaterialPlugin};
use super::{
    apply_layer_residency_plan, build_layer_requests, build_layer_residency_plan,
    compute_cache_budget, compute_desired_layer_tiles, desired_change_is_minor,
    ensure_manifest_request, implicit_identity_tileset, layer_map_version, layer_tileset_url,
    log_tile_stats, merge_level_counts, start_tile_requests, sum_level_counts,
    update_camera_motion_state, BuildResult, CameraMotionState, DesiredTileComputation,
    LayerManifestCache, LayerRequestBuild, LayerViewState, PendingLayerManifests,
    RasterLoadedAssets, RasterLoadedContext, RasterTileCache, StartTileRequests, TileDebugControls,
    TileFrameClock, TileResidencyState, TileStats, VisibilityUpdateContext, VisualFilterContext,
    REQUEST_REFRESH_INTERVAL_FRAMES,
};

pub(crate) fn build_plugin(app: &mut App) {
    app.add_plugins(ZoneMaskMaterialPlugin)
        .init_resource::<LayerRegistry>()
        .init_resource::<LayerRuntime>()
        .init_resource::<LayerViewState>()
        .init_resource::<TileFrameClock>()
        .init_resource::<CameraMotionState>()
        .init_resource::<TileResidencyState>()
        .init_resource::<LayerManifestCache>()
        .init_resource::<PendingLayerManifests>()
        .init_resource::<crate::map::streaming::TileStreamer>()
        .init_resource::<RasterTileCache>()
        .init_resource::<ExactLookupCache>()
        .init_resource::<TileStats>()
        .init_resource::<TileDebugControls>()
        .add_systems(Update, update_tiles);
}

#[derive(SystemParam)]
struct RasterUpdateContext<'w, 's> {
    commands: Commands<'w, 's>,
    asset_server: Res<'w, AssetServer>,
    images: ResMut<'w, Assets<Image>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<ColorMaterial>>,
    zone_mask_materials: ResMut<'w, Assets<ZoneMaskMaterial>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    camera_q: Query<'w, 's, (&'static Camera, &'static Transform), With<Map2dCamera>>,
    pan_state: Res<'w, PanState>,
    view_mode: Res<'w, ViewModeState>,
    terrain_view: Res<'w, TerrainViewEstimate>,
    bootstrap: ResMut<'w, ApiBootstrapState>,
    display_state: ResMut<'w, MapDisplayState>,
    debug_controls: Res<'w, TileDebugControls>,
    layer_runtime: ResMut<'w, LayerRuntime>,
    layer_registry: Res<'w, LayerRegistry>,
    frame_clock: ResMut<'w, TileFrameClock>,
    motion_state: ResMut<'w, CameraMotionState>,
    residency: ResMut<'w, TileResidencyState>,
    view_state: ResMut<'w, LayerViewState>,
    manifests: ResMut<'w, LayerManifestCache>,
    pending_manifests: ResMut<'w, PendingLayerManifests>,
    streamer: ResMut<'w, crate::map::streaming::TileStreamer>,
    cache: ResMut<'w, RasterTileCache>,
    stats: ResMut<'w, TileStats>,
    time: Res<'w, Time>,
    evidence_zone_filter: Res<'w, EvidenceZoneFilter>,
    vector_runtime: Res<'w, VectorLayerRuntime>,
    exact_lookups: Res<'w, ExactLookupCache>,
}

fn update_tiles(mut ctx: RasterUpdateContext<'_, '_>) {
    crate::perf_scope!("raster.update_tiles");
    let display_state_changed = ctx.display_state.is_changed();
    let bootstrap_changed = ctx.bootstrap.is_changed();
    let evidence_zone_filter_changed = ctx.evidence_zone_filter.is_changed();
    let vector_runtime_changed = ctx.vector_runtime.is_changed();
    let view_mode_changed = ctx.view_mode.is_changed();
    let commands = &mut ctx.commands;
    let asset_server = &ctx.asset_server;
    let images = &mut ctx.images;
    let meshes = &mut ctx.meshes;
    let materials = &mut ctx.materials;
    let zone_mask_materials = &mut ctx.zone_mask_materials;
    let windows = &ctx.windows;
    let camera_q = &ctx.camera_q;
    let pan_state = &ctx.pan_state;
    let view_mode = &ctx.view_mode;
    let terrain_view = &ctx.terrain_view;
    let bootstrap = &mut ctx.bootstrap;
    let display_state = &mut ctx.display_state;
    let debug_controls = &ctx.debug_controls;
    let layer_runtime = &mut ctx.layer_runtime;
    let layer_registry = &ctx.layer_registry;
    let frame_clock = &mut ctx.frame_clock;
    let motion_state = &mut ctx.motion_state;
    let residency = &mut ctx.residency;
    let view_state = &mut ctx.view_state;
    let manifests = &mut ctx.manifests;
    let pending_manifests = &mut ctx.pending_manifests;
    let streamer = &mut ctx.streamer;
    let cache = &mut ctx.cache;
    let stats = &mut ctx.stats;
    let time = &ctx.time;
    let evidence_zone_filter = &ctx.evidence_zone_filter;
    let vector_runtime = &ctx.vector_runtime;
    let exact_lookups = &ctx.exact_lookups;

    layer_runtime.sync_to_registry(layer_registry);

    if layer_registry.is_changed() {
        cache.clear_all(commands, images);
        view_state.per_layer.clear();
        streamer.clear();
        manifests.clear();
        pending_manifests.clear();
    }

    if let Some(mask_layer_id) = layer_registry.first_id_by_pick_mode(PickMode::ExactTilePixel) {
        if let Some(mask_state) = layer_runtime.get(mask_layer_id) {
            if display_state.show_zone_mask != mask_state.visible {
                display_state.show_zone_mask = mask_state.visible;
            }
            if (display_state.zone_mask_opacity - mask_state.opacity).abs() > f32::EPSILON {
                display_state.zone_mask_opacity = mask_state.opacity;
            }
        }
    }

    if bootstrap.map_version_dirty {
        for layer in layer_registry.ordered() {
            if layer.tile_url_template.contains("{map_version}")
                || layer.tileset_url.contains("{map_version}")
            {
                cache.clear_layer(layer.id, commands, images);
                view_state.per_layer.remove(&layer.id);
                streamer.clear_layer(layer.id);
                manifests.remove_layer(layer.id);
                pending_manifests.remove_layer(layer.id);
            }
        }
        bootstrap.map_version_dirty = false;
    }

    super::manifest::poll_manifest_requests(manifests, pending_manifests);

    let Ok(window) = windows.single() else {
        return;
    };
    let map_to_world = MapToWorld::default();
    let (view_world, cursor_world) = match view_mode.mode {
        ViewMode::Map2D => {
            let Ok((camera, camera_transform)) = camera_q.single() else {
                return;
            };
            let Some(view_world) = view_rect(camera, camera_transform, window) else {
                return;
            };
            let cursor_world = window.cursor_position().and_then(|cursor| {
                camera
                    .viewport_to_world_2d(&GlobalTransform::from(*camera_transform), cursor)
                    .ok()
                    .map(|world| (world.x, world.y))
            });
            (view_world, cursor_world)
        }
        ViewMode::Terrain3D => {
            let view = terrain_view
                .view_world
                .unwrap_or_else(|| map_to_world.world_bounds());
            let cursor_world = terrain_view
                .cursor_world
                .map(|world| (world.x as f32, world.z as f32));
            (view, cursor_world)
        }
    };

    stats.view_min = Some((view_world.min.x as f32, view_world.min.z as f32));
    stats.view_max = Some((view_world.max.x as f32, view_world.max.z as f32));
    stats.cursor_world = cursor_world;
    stats.cursor_map = cursor_world.map(|(x, z)| {
        let map = map_to_world.world_to_map(WorldPoint::new(x as f64, z as f64));
        (map.x as f32, map.y as f32)
    });

    frame_clock.frame = frame_clock.frame.wrapping_add(1);
    let frame = frame_clock.frame;
    update_camera_motion_state(motion_state, view_world, pan_state);
    residency.begin_frame(frame);

    stats.camera_unstable = motion_state.unstable;
    stats.camera_pan_fraction = motion_state.pan_fraction as f32;
    stats.camera_zoom_out_ratio = motion_state.zoom_out_ratio as f32;
    stats.resident_by_level.clear();
    stats.protected_by_level.clear();
    stats.warm_by_level.clear();
    stats.fallback_visible_by_level.clear();
    stats.blank_visible_by_layer.clear();
    stats.fallback_visible_tiles = 0;
    stats.blank_visible_tiles = 0;
    stats.detail_requests_queued = 0;
    stats.coverage_requests_queued = 0;
    let base_budget = compute_cache_budget(layer_registry, layer_runtime);
    let residency_floor = residency
        .protected
        .len()
        .saturating_add(residency.warm.len())
        .saturating_add(128);
    cache.max_entries = base_budget
        .max(residency_floor)
        .clamp(256, crate::config::TILE_CACHE_MAX);
    let mut any_zoom_level_changed = false;

    let clip_mask_source_ids = layer_runtime.clip_mask_source_ids();
    for layer in layer_registry.ordered() {
        let Some(runtime_state) = layer_runtime.get_mut(layer.id) else {
            continue;
        };

        runtime_state.visible_tile_count = 0;
        runtime_state.current_base_lod = None;
        runtime_state.current_detail_lod = None;
        runtime_state.resident_tile_count = 0;
        runtime_state.pending_count = 0;
        runtime_state.inflight_count = 0;

        if !layer.is_raster() {
            runtime_state.manifest_status = LayerManifestStatus::Missing;
            streamer.clear_layer(layer.id);
            view_state.per_layer.remove(&layer.id);
            continue;
        }

        let active_for_render_or_clip =
            runtime_state.visible || clip_mask_source_ids.contains(&layer.id);
        if !active_for_render_or_clip {
            streamer.clear_layer(layer.id);
            view_state.per_layer.remove(&layer.id);
            continue;
        }

        let Some((map_version_id, map_version)) =
            layer_map_version(layer, bootstrap.map_version.as_deref())
        else {
            runtime_state.manifest_status = LayerManifestStatus::Missing;
            streamer.clear_layer(layer.id);
            view_state.per_layer.remove(&layer.id);
            continue;
        };
        let manifest_url = layer_tileset_url(layer, map_version);

        ensure_manifest_request(layer.id, &manifest_url, manifests, pending_manifests);
        let manifest_status = manifests.status(layer.id, &manifest_url);
        let fallback_tileset = if manifests.get(layer.id, &manifest_url).is_none() {
            implicit_identity_tileset(layer, map_to_world)
        } else {
            None
        };
        let tileset = manifests
            .get(layer.id, &manifest_url)
            .or(fallback_tileset.as_ref());
        let Some(tileset) = tileset else {
            runtime_state.manifest_status = manifest_status;
            streamer.clear_layer(layer.id);
            view_state.per_layer.remove(&layer.id);
            continue;
        };
        runtime_state.manifest_status = LayerManifestStatus::Ready;

        let Some(world_transform) = layer.world_transform(map_to_world) else {
            runtime_state.manifest_status = LayerManifestStatus::Failed;
            streamer.clear_layer(layer.id);
            view_state.per_layer.remove(&layer.id);
            continue;
        };

        let previous = view_state.per_layer.get(&layer.id).copied();
        let desired = compute_desired_layer_tiles(DesiredTileComputation {
            layer,
            tileset,
            world_transform,
            view_world,
            map_version: map_version_id,
            frame,
            runtime: runtime_state,
            previous,
        });
        view_state.per_layer.insert(layer.id, desired);
        if previous.map(super::policy::lod_signature) != Some(super::policy::lod_signature(desired))
        {
            any_zoom_level_changed = true;
        }

        let plan = build_layer_residency_plan(
            layer,
            tileset,
            desired,
            map_version_id,
            cache,
            motion_state.unstable,
        );
        apply_layer_residency_plan(layer.id, plan, residency);

        let desired_changed = previous != Some(desired);
        let minor_shift = desired_change_is_minor(previous, desired, &layer.lod_policy);
        let refresh_slot = frame % REQUEST_REFRESH_INTERVAL_FRAMES
            == u64::from(layer.id.as_u16()) % REQUEST_REFRESH_INTERVAL_FRAMES;
        let queue_empty = streamer.pending_len_for_layer(layer.id) == 0;
        let should_rebuild_requests = if desired_changed {
            !minor_shift || queue_empty
        } else {
            queue_empty && refresh_slot
        };
        if should_rebuild_requests {
            let BuildResult {
                requests,
                cache_hits,
                cache_hits_by_level,
                cache_misses_by_level,
                detail_queued,
                coverage_queued,
            } = build_layer_requests(LayerRequestBuild {
                layer,
                tileset,
                desired,
                map_version,
                cache,
                map_version_id,
                camera_unstable: motion_state.unstable,
                residency,
            });
            stats.cache_hits = stats.cache_hits.saturating_add(cache_hits);
            merge_level_counts(
                &mut stats.cache_hits_by_level,
                layer.id,
                &cache_hits_by_level,
            );
            merge_level_counts(
                &mut stats.cache_misses_by_level,
                layer.id,
                &cache_misses_by_level,
            );
            stats.detail_requests_queued =
                stats.detail_requests_queued.saturating_add(detail_queued);
            stats.coverage_requests_queued = stats
                .coverage_requests_queued
                .saturating_add(coverage_queued);
            streamer.replace_layer(layer.id, requests);
        }
    }

    stats.protected_by_level = residency.protected_by_layer_level.clone();
    stats.warm_by_level = residency.warm_by_layer_level.clone();
    stats.fallback_visible_by_level = residency.fallback_by_layer_level.clone();
    stats.blank_visible_by_layer = residency.blank_visible_by_layer.clone();
    stats.fallback_visible_tiles = residency
        .fallback_by_layer_level
        .values()
        .map(sum_level_counts)
        .sum();
    stats.blank_visible_tiles = residency.blank_visible_by_layer.values().copied().sum();

    stats.inflight = cache.inflight_count_total().min(streamer.max_inflight);
    streamer.inflight = stats.inflight;

    start_tile_requests(StartTileRequests {
        streamer,
        cache,
        asset_server,
        layer_registry,
        layer_runtime,
        residency,
        camera_unstable: motion_state.unstable,
        stats,
    });

    let loaded_changed = cache.update_loaded(
        commands,
        RasterLoadedAssets {
            images,
            meshes,
            materials,
            zone_mask_materials,
        },
        RasterLoadedContext {
            asset_server,
            layer_registry,
            layer_runtime,
            map_to_world,
            view_mode: view_mode.mode,
            residency,
            stats,
        },
    );

    let (visible_by_layer, visibility_changed) = cache.update_visibility(
        commands,
        VisibilityUpdateContext {
            materials,
            zone_mask_materials,
            layer_registry,
            layer_runtime,
            residency,
            frame,
            camera_unstable: motion_state.unstable,
            view_mode: view_mode.mode,
            hovered_zone_rgb: display_state.hovered_zone_rgb,
        },
    );

    let has_active_raster_clip_masks = layer_runtime.iter().any(|(layer_id, state)| {
        state.clip_mask_layer.is_some()
            && layer_registry
                .get(layer_id)
                .map(|layer| layer.is_raster())
                .unwrap_or(false)
    });
    let should_sync_visual_filters = view_mode.mode == ViewMode::Map2D
        || has_active_raster_clip_masks
        || loaded_changed
        || visibility_changed
        || display_state_changed
        || bootstrap_changed
        || evidence_zone_filter_changed
        || vector_runtime_changed
        || view_mode_changed;
    if should_sync_visual_filters {
        cache.sync_visual_filters(
            images,
            commands,
            VisualFilterContext {
                filter: evidence_zone_filter,
                hover_zone_rgb: display_state.hovered_zone_rgb,
                layer_registry,
                layer_runtime,
                exact_lookups,
                vector_runtime,
                map_version: bootstrap.map_version.as_deref(),
                view_mode: view_mode.mode,
            },
        );
    }
    if any_zoom_level_changed && !debug_controls.disable_eviction {
        cache.evict(commands, images, stats, residency, layer_registry);
    }

    let mut total_visible = 0_u32;
    for layer in layer_registry.ordered() {
        let visible = *visible_by_layer.get(&layer.id).unwrap_or(&0);
        total_visible = total_visible.saturating_add(visible);
        if let Some(state) = layer_runtime.get_mut(layer.id) {
            state.visible_tile_count = visible;
            state.resident_tile_count = cache.resident_count_by_layer(layer.id);
            state.pending_count = streamer.pending_len_for_layer(layer.id) as u32;
            state.inflight_count = cache.inflight_count_by_layer(layer.id);
        }
    }

    stats.resident_by_level = cache.resident_counts_by_layer_level();
    stats.visible_tiles = total_visible;
    stats.inflight = cache.inflight_count_total().min(streamer.max_inflight);
    streamer.inflight = stats.inflight;
    stats.queue_len = streamer.pending_len();
    log_tile_stats(stats, time);
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
