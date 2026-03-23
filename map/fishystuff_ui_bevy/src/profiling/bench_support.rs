use std::fs;
use std::path::PathBuf;

use bevy::image::Image;
use bevy::prelude::Handle;
use fishystuff_api::models::events::{EventPointCompact, EventsSnapshotResponse, MapBboxPx};
use fishystuff_core::terrain::{decode_terrain_chunk, TerrainChunkData, TerrainManifest};

use crate::map::events::{cluster_view_events, SpatialIndex};
use crate::map::layers::{
    GeometrySpace, LayerId, LayerKind, LayerRuntimeState, LayerSpec, LodPolicy, PickMode,
    StyleMode, VectorSourceSpec,
};
use crate::map::raster::cache::{RasterTileCache, RasterTileEntry, TileState};
use crate::map::raster::manifest::{LevelInfo, LoadedTileset};
use crate::map::raster::policy::{
    apply_layer_residency_plan, build_layer_requests, build_layer_residency_plan,
    compute_desired_layer_tiles, eviction_priority_score, DesiredLayerTiles,
    DesiredTileComputation, LayerRequestBuild, TileBounds, TileResidencyState,
};
use crate::map::raster::TileKey;
use crate::map::spaces::layer_transform::LayerTransform;
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::{MapPoint, WorldPoint, WorldRect};
use crate::map::terrain::mesh::build_chunk_mesh_from_data;
use crate::map::vector::build::{
    advance_job, finalize_job, parse_into_job, AdvanceResult, VectorBuildLimits,
};
use crate::map::vector::triangulate::{triangle_count, triangulate_polygon};

#[derive(Clone)]
pub struct RasterBenchFixture {
    layer: LayerSpec,
    tileset: LoadedTileset,
    world_transform: crate::map::spaces::layer_transform::WorldTransform,
    view_world: WorldRect,
    map_version_id: u64,
    frame: u64,
    previous: Option<DesiredLayerTiles>,
    cache_entries: Vec<(TileKey, u64)>,
}

pub struct VectorBenchFixture {
    pub bytes: Vec<u8>,
    pub sample_polygon: Vec<Vec<[f64; 2]>>,
}

pub struct TerrainBenchFixture {
    pub manifest: TerrainManifest,
    pub chunk: TerrainChunkData,
}

pub struct EventsBenchFixture {
    pub events: Vec<EventPointCompact>,
    pub spatial_index: SpatialIndex,
    pub bbox: MapBboxPx,
    pub filtered_indices: Vec<usize>,
    pub cluster_bucket_px: i32,
}

pub fn raster_fixture() -> RasterBenchFixture {
    let layer = LayerSpec {
        id: LayerId::from_raw(0),
        key: "bench_raster".to_string(),
        name: "Bench Raster".to_string(),
        visible_default: true,
        opacity_default: 1.0,
        z_base: 0.0,
        kind: LayerKind::TiledRaster,
        tileset_url: "bench://tileset.json".to_string(),
        tile_url_template: "bench://tiles/{z}/{x}_{y}.png".to_string(),
        tileset_version: "bench-v1".to_string(),
        field_source: None,
        field_metadata_source: None,
        vector_source: None,
        transform: LayerTransform::IdentityMapSpace,
        tile_px: 512,
        max_level: 3,
        y_flip: false,
        lod_policy: LodPolicy {
            target_tiles: 24,
            hysteresis_hi: 28.0,
            hysteresis_lo: 16.0,
            margin_tiles: 1,
            enable_refine: true,
            refine_debounce_ms: 0,
            max_detail_tiles: 48,
            max_resident_tiles: 128,
            pinned_coarse_levels: 1,
            coarse_pin_min_level: None,
            warm_margin_tiles: 1,
            protected_margin_tiles: 1,
            detail_eviction_weight: 4.0,
            max_detail_requests_while_camera_moving: 4,
            motion_suppresses_refine: true,
        },
        request_weight: 1.0,
        pick_mode: PickMode::None,
        display_order: 0,
    };
    let tileset = full_tileset(3);
    let map_to_world = MapToWorld::default();
    let world_transform = layer
        .world_transform(map_to_world)
        .expect("identity transform must build");
    let min = map_to_world.map_to_world(MapPoint::new(384.0, 512.0));
    let max = map_to_world.map_to_world(MapPoint::new(1664.0, 1536.0));
    let view_world = WorldRect {
        min: WorldPoint::new(min.x, min.z),
        max: WorldPoint::new(max.x, max.z),
    };
    let previous = Some(DesiredLayerTiles {
        base: Some(TileBounds {
            min_tx: 0,
            max_tx: 2,
            min_ty: 1,
            max_ty: 3,
            z: 1,
            map_version: 1,
        }),
        detail: Some(TileBounds {
            min_tx: 1,
            max_tx: 4,
            min_ty: 2,
            max_ty: 5,
            z: 0,
            map_version: 1,
        }),
    });
    let cache_entries = (0..=3)
        .flat_map(|z| {
            let width = 1_i32 << (3 - z);
            (0..width).flat_map(move |ty| {
                (0..width).map(move |tx| {
                    (
                        TileKey {
                            layer: layer.id,
                            map_version: 1,
                            z,
                            tx,
                            ty,
                        },
                        ((z + 1) * 1_000 + ty * 17 + tx * 31) as u64,
                    )
                })
            })
        })
        .collect();
    RasterBenchFixture {
        layer,
        tileset,
        world_transform,
        view_world,
        map_version_id: 1,
        frame: 240,
        previous,
        cache_entries,
    }
}

pub fn raster_visible_tile_computation(fixture: &RasterBenchFixture) -> usize {
    let mut runtime = runtime_state_for(&fixture.layer);
    let desired = compute_desired_layer_tiles(DesiredTileComputation {
        layer: &fixture.layer,
        tileset: &fixture.tileset,
        world_transform: fixture.world_transform,
        view_world: fixture.view_world,
        map_version: fixture.map_version_id,
        frame: fixture.frame,
        runtime: &mut runtime,
        previous: fixture.previous,
    });
    tile_bounds_count(desired.base) + tile_bounds_count(desired.detail)
}

pub fn raster_desired_set_build(fixture: &RasterBenchFixture) -> usize {
    let mut runtime = runtime_state_for(&fixture.layer);
    let desired = compute_desired_layer_tiles(DesiredTileComputation {
        layer: &fixture.layer,
        tileset: &fixture.tileset,
        world_transform: fixture.world_transform,
        view_world: fixture.view_world,
        map_version: fixture.map_version_id,
        frame: fixture.frame,
        runtime: &mut runtime,
        previous: fixture.previous,
    });
    let cache = cache_from_fixture(fixture);
    let plan = build_layer_residency_plan(
        &fixture.layer,
        &fixture.tileset,
        desired,
        fixture.map_version_id,
        &cache,
        false,
    );
    let mut residency = TileResidencyState::default();
    apply_layer_residency_plan(fixture.layer.id, plan, &mut residency);
    residency
        .render_visible
        .len()
        .saturating_add(residency.protected.len())
        .saturating_add(residency.warm.len())
}

pub fn raster_request_scheduling(fixture: &RasterBenchFixture) -> usize {
    let mut runtime = runtime_state_for(&fixture.layer);
    let desired = compute_desired_layer_tiles(DesiredTileComputation {
        layer: &fixture.layer,
        tileset: &fixture.tileset,
        world_transform: fixture.world_transform,
        view_world: fixture.view_world,
        map_version: fixture.map_version_id,
        frame: fixture.frame,
        runtime: &mut runtime,
        previous: fixture.previous,
    });
    let cache = cache_from_fixture(fixture);
    let result = build_layer_requests(LayerRequestBuild {
        layer: &fixture.layer,
        tileset: &fixture.tileset,
        desired,
        map_version: Some("bench-v1"),
        cache: &cache,
        map_version_id: fixture.map_version_id,
        camera_unstable: false,
        residency: &TileResidencyState::default(),
    });
    result
        .requests
        .len()
        .saturating_add(result.cache_hits as usize)
}

pub fn raster_eviction_score_sum(fixture: &RasterBenchFixture) -> f64 {
    let cache = cache_from_fixture(fixture);
    let mut runtime = runtime_state_for(&fixture.layer);
    let desired = compute_desired_layer_tiles(DesiredTileComputation {
        layer: &fixture.layer,
        tileset: &fixture.tileset,
        world_transform: fixture.world_transform,
        view_world: fixture.view_world,
        map_version: fixture.map_version_id,
        frame: fixture.frame,
        runtime: &mut runtime,
        previous: fixture.previous,
    });
    let plan = build_layer_residency_plan(
        &fixture.layer,
        &fixture.tileset,
        desired,
        fixture.map_version_id,
        &cache,
        false,
    );
    let mut residency = TileResidencyState::default();
    apply_layer_residency_plan(fixture.layer.id, plan, &mut residency);
    cache
        .entries
        .iter()
        .map(|(key, entry)| {
            eviction_priority_score(
                cache.use_counter,
                *key,
                entry,
                &residency,
                Some(&fixture.layer),
            )
        })
        .sum()
}

pub fn vector_fixture() -> VectorBenchFixture {
    let mut features = Vec::new();
    for y in 0..16 {
        for x in 0..16 {
            let left = 128.0 + x as f64 * 96.0;
            let top = 128.0 + y as f64 * 96.0;
            let right = left + 72.0;
            let bottom = top + 72.0;
            let hole_left = left + 18.0;
            let hole_top = top + 18.0;
            let hole_right = right - 18.0;
            let hole_bottom = bottom - 18.0;
            features.push(serde_json::json!({
                "type": "Feature",
                "properties": {
                    "id": y * 16 + x,
                    "c": [((x * 17) % 255), ((y * 29) % 255), (((x + y) * 13) % 255), 180]
                },
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [
                        [[left, top], [right, top], [right, bottom], [left, bottom], [left, top]],
                        [[hole_left, hole_top], [hole_right, hole_top], [hole_right, hole_bottom], [hole_left, hole_bottom], [hole_left, hole_top]]
                    ]
                }
            }));
        }
    }
    let bytes = serde_json::to_vec(&serde_json::json!({
        "type": "FeatureCollection",
        "features": features,
    }))
    .expect("encode synthetic geojson");
    let sample_polygon = vec![
        vec![
            [0.0, 0.0],
            [256.0, 0.0],
            [256.0, 256.0],
            [0.0, 256.0],
            [0.0, 0.0],
        ],
        vec![
            [64.0, 64.0],
            [192.0, 64.0],
            [192.0, 192.0],
            [64.0, 192.0],
            [64.0, 64.0],
        ],
    ];
    VectorBenchFixture {
        bytes,
        sample_polygon,
    }
}

pub fn vector_triangulation(fixture: &VectorBenchFixture) -> usize {
    let piece = triangulate_polygon(
        &fixture.sample_polygon,
        GeometrySpace::MapPixels,
        MapToWorld::default(),
    )
    .expect("triangulate synthetic polygon")
    .expect("synthetic polygon must produce a piece");
    triangle_count(&piece)
}

pub fn terrain_fixture() -> TerrainBenchFixture {
    let root = profiling_fixture_root();
    let manifest = serde_json::from_slice::<TerrainManifest>(
        &fs::read(root.join("images/terrain/v1/manifest.json")).expect("read terrain manifest"),
    )
    .expect("decode terrain manifest");
    let chunk = decode_terrain_chunk(
        &fs::read(root.join("images/terrain/v1/levels/0/0_0.thc")).expect("read terrain chunk"),
    )
    .expect("decode terrain chunk");
    TerrainBenchFixture { manifest, chunk }
}

pub fn terrain_mesh_build(fixture: &TerrainBenchFixture) -> usize {
    let mesh = build_chunk_mesh_from_data(&fixture.chunk, &fixture.manifest, MapToWorld::default())
        .expect("fixture chunk should build a mesh");
    mesh.count_vertices()
}

pub fn events_fixture() -> EventsBenchFixture {
    let root = profiling_fixture_root();
    let snapshot = serde_json::from_slice::<EventsSnapshotResponse>(
        &fs::read(root.join("events_snapshot.json")).expect("read events snapshot"),
    )
    .expect("decode events snapshot");
    let mut spatial_index = SpatialIndex::new(128);
    spatial_index.rebuild(&snapshot.events);
    let bbox = MapBboxPx {
        min_x: 320,
        min_y: 320,
        max_x: 1680,
        max_y: 1600,
    };
    let filtered_indices = spatial_index.query_bbox(&bbox, &snapshot.events);
    EventsBenchFixture {
        events: snapshot.events,
        spatial_index,
        bbox,
        filtered_indices,
        cluster_bucket_px: 96,
    }
}

pub fn event_bbox_query(fixture: &EventsBenchFixture) -> usize {
    fixture
        .spatial_index
        .query_bbox(&fixture.bbox, &fixture.events)
        .len()
}

pub fn event_clustering(fixture: &EventsBenchFixture) -> usize {
    let output = cluster_view_events(
        &fixture.events,
        &fixture.filtered_indices,
        fixture.cluster_bucket_px,
    );
    output.points.len()
}

pub fn vector_pipeline_build(fixture: &VectorBenchFixture) -> usize {
    let source = VectorSourceSpec {
        url: "bench://vector.geojson".to_string(),
        revision: "bench-rg-v1".to_string(),
        geometry_space: GeometrySpace::MapPixels,
        style_mode: StyleMode::FeaturePropertyPalette,
        feature_id_property: Some("id".to_string()),
        color_property: Some("c".to_string()),
    };
    let mut job = parse_into_job(source, "bench-rg-v1".to_string(), fixture.bytes.clone())
        .expect("build synthetic vector job");
    let limits = VectorBuildLimits {
        max_features_per_frame: usize::MAX,
        max_build_ms_per_frame: f64::MAX,
        max_chunk_vertices: 20_000,
        max_chunk_triangles: 24_000,
    };
    while let AdvanceResult::InProgress =
        advance_job(&mut job, MapToWorld::default(), limits).expect("advance vector job")
    {}
    finalize_job(job, limits).stats.triangle_count as usize
}

fn profiling_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/profiling")
}

fn runtime_state_for(layer: &LayerSpec) -> LayerRuntimeState {
    LayerRuntimeState {
        visible: true,
        opacity: 1.0,
        clip_mask_layer: None,
        z_base: layer.z_base,
        display_order: layer.display_order,
        current_base_lod: None,
        current_detail_lod: None,
        last_view_update_frame: 0,
        visible_tile_count: 0,
        resident_tile_count: 0,
        pending_count: 0,
        inflight_count: 0,
        manifest_status: crate::map::layers::LayerManifestStatus::Ready,
        vector_status: crate::map::layers::LayerVectorStatus::Inactive,
        vector_progress: 0.0,
        vector_fetched_bytes: 0,
        vector_feature_count: 0,
        vector_features_processed: 0,
        vector_polygon_count: 0,
        vector_multipolygon_count: 0,
        vector_hole_ring_count: 0,
        vector_vertex_count: 0,
        vector_triangle_count: 0,
        vector_mesh_count: 0,
        vector_chunked_bucket_count: 0,
        vector_build_ms: 0.0,
        vector_last_frame_build_ms: 0.0,
        vector_cache_hits: 0,
        vector_cache_misses: 0,
        vector_cache_last_hit: false,
        vector_cache_entries: 0,
    }
}

fn cache_from_fixture(fixture: &RasterBenchFixture) -> RasterTileCache {
    let mut cache = RasterTileCache {
        use_counter: 50_000,
        ..Default::default()
    };
    for (key, last_used) in &fixture.cache_entries {
        cache.entries.insert(
            *key,
            RasterTileEntry {
                handle: Handle::<Image>::default(),
                entity: None,
                material: None,
                state: TileState::Ready,
                visible: false,
                alpha: 1.0,
                depth: 0.0,
                last_used: *last_used,
                exact_quad: false,
                sprite_size: None,
                pixel_data: None,
                zone_rgbs: Vec::new(),
                zone_lookup_rows: None,
                filter_active: false,
                filter_revision: 0,
                pixel_filtered: false,
                hover_highlight_zone: None,
                clip_mask_layer: None,
                clip_mask_revision: 0,
                clip_mask_applied: false,
                linger_until_frame: 0,
            },
        );
    }
    cache
}

fn full_tileset(max_level: i32) -> LoadedTileset {
    let mut levels = Vec::new();
    for z in 0..=max_level {
        let width = 1_u32 << (max_level - z);
        levels.push(full_level(z, width, width));
    }
    LoadedTileset {
        tile_px: 512,
        max_level: max_level as u8,
        levels,
    }
}

fn full_level(z: i32, width: u32, height: u32) -> LevelInfo {
    let bits = width as usize * height as usize;
    let mut occupancy = vec![0_u8; bits.div_ceil(8)];
    for bit in 0..bits {
        occupancy[bit >> 3] |= 1 << (bit & 7);
    }
    LevelInfo {
        z,
        min_x: 0,
        min_y: 0,
        max_x: width as i32 - 1,
        max_y: height as i32 - 1,
        width,
        height,
        tile_count: bits,
        occupancy,
    }
}

fn tile_bounds_count(bounds: Option<TileBounds>) -> usize {
    let Some(bounds) = bounds else {
        return 0;
    };
    let width = (bounds.max_tx - bounds.min_tx + 1).max(0) as usize;
    let height = (bounds.max_ty - bounds.min_ty + 1).max(0) as usize;
    width.saturating_mul(height)
}
