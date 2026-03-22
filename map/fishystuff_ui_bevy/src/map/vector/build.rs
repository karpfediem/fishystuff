use std::collections::HashMap;
use std::time::Duration;

use async_channel::Receiver;
use bevy::platform::time::Instant;
use serde_json::{Map, Value};

use crate::map::layers::{LayerVectorStatus, VectorSourceSpec};
use crate::map::spaces::world::MapToWorld;
use crate::map::vector::cache::{
    BuiltVectorChunk, BuiltVectorGeometry, HoverFeature, HoverPolygon, VectorLayerStats,
};
use crate::map::vector::geojson::parse_geojson;
use crate::map::vector::style::{style_bucket_key, StyleBucketKey};
use crate::map::vector::triangulate::{
    project_polygon, triangulate_projected_polygon, PolygonPiece, ProjectedPolygon,
};
use crate::runtime_io;

pub const DEFAULT_FRAME_BUDGET_MS: f64 = 3.0;
pub const DEFAULT_FEATURES_PER_FRAME: usize = 64;
pub const DEFAULT_CHUNK_THRESHOLD_VERTICES: usize = 120_000;
pub const DEFAULT_CHUNK_THRESHOLD_TRIANGLES: usize = 220_000;
const MAP_CHUNK_PX: f64 = 2048.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VectorBuildLimits {
    pub max_features_per_frame: usize,
    pub max_build_ms_per_frame: f64,
    pub max_chunk_vertices: usize,
    pub max_chunk_triangles: usize,
}

impl Default for VectorBuildLimits {
    fn default() -> Self {
        Self {
            max_features_per_frame: DEFAULT_FEATURES_PER_FRAME,
            max_build_ms_per_frame: DEFAULT_FRAME_BUDGET_MS,
            max_chunk_vertices: DEFAULT_CHUNK_THRESHOLD_VERTICES,
            max_chunk_triangles: DEFAULT_CHUNK_THRESHOLD_TRIANGLES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceResult {
    InProgress,
    Complete,
}

#[derive(Default)]
pub enum VectorBuildState {
    #[default]
    NotRequested,
    Fetching {
        source: VectorSourceSpec,
        revision: String,
        url: String,
        receiver: Receiver<Result<Vec<u8>, String>>,
        started_at: Instant,
    },
    Parsing {
        source: VectorSourceSpec,
        revision: String,
        bytes: Vec<u8>,
        started_at: Instant,
    },
    Building {
        job: VectorBuildJob,
    },
    Ready {
        revision: String,
    },
    Failed {
        revision: String,
        error: String,
    },
}

pub struct VectorBuildJob {
    source: VectorSourceSpec,
    revision: String,
    features: Vec<PreparedFeature>,
    hover_features: Vec<HoverFeature>,
    next_feature: usize,
    pieces: Vec<PolygonPiece>,
    pub stats: VectorLayerStats,
    started_at: Instant,
}

struct PreparedFeature {
    bucket: StyleBucketKey,
    properties: Map<String, Value>,
    polygons: Vec<Vec<Vec<[f64; 2]>>>,
}

impl VectorBuildJob {
    pub fn revision(&self) -> &str {
        self.revision.as_str()
    }
}

pub fn begin_fetch(source: VectorSourceSpec, revision: String) -> VectorBuildState {
    let url = source.url.clone();
    VectorBuildState::Fetching {
        source,
        revision,
        receiver: spawn_geojson_fetch(url.clone()),
        url,
        started_at: Instant::now(),
    }
}

pub fn revision_matches(state: &VectorBuildState, revision: &str) -> bool {
    state_revision(state)
        .map(|value| value == revision)
        .unwrap_or(false)
}

pub fn state_revision(state: &VectorBuildState) -> Option<&str> {
    match state {
        VectorBuildState::NotRequested => None,
        VectorBuildState::Fetching { revision, .. }
        | VectorBuildState::Parsing { revision, .. }
        | VectorBuildState::Ready { revision }
        | VectorBuildState::Failed { revision, .. } => Some(revision.as_str()),
        VectorBuildState::Building { job } => Some(job.revision.as_str()),
    }
}

pub fn state_status(state: &VectorBuildState) -> LayerVectorStatus {
    match state {
        VectorBuildState::NotRequested => LayerVectorStatus::NotRequested,
        VectorBuildState::Fetching { .. } => LayerVectorStatus::Fetching,
        VectorBuildState::Parsing { .. } => LayerVectorStatus::Parsing,
        VectorBuildState::Building { .. } => LayerVectorStatus::Building,
        VectorBuildState::Ready { .. } => LayerVectorStatus::Ready,
        VectorBuildState::Failed { .. } => LayerVectorStatus::Failed,
    }
}

pub fn state_stats(state: &VectorBuildState) -> Option<VectorLayerStats> {
    match state {
        VectorBuildState::NotRequested => None,
        VectorBuildState::Fetching { started_at, .. } => Some(VectorLayerStats {
            progress: 0.0,
            build_ms: started_at.elapsed().as_secs_f32() * 1000.0,
            ..Default::default()
        }),
        VectorBuildState::Parsing {
            bytes, started_at, ..
        } => Some(VectorLayerStats {
            fetched_bytes: bytes.len() as u32,
            progress: 0.0,
            build_ms: started_at.elapsed().as_secs_f32() * 1000.0,
            ..Default::default()
        }),
        VectorBuildState::Building { job } => Some(job.stats),
        VectorBuildState::Ready { .. } | VectorBuildState::Failed { .. } => None,
    }
}

pub fn parse_into_job(
    source: VectorSourceSpec,
    revision: String,
    bytes: Vec<u8>,
) -> Result<VectorBuildJob, String> {
    let parsed = parse_geojson(&bytes)?;
    let mut features = Vec::with_capacity(parsed.features.len());

    crate::perf_scope!("vector.feature_iteration");
    for (feature_index, feature) in parsed.features.into_iter().enumerate() {
        let bucket = style_bucket_key(&source, &feature.properties, feature_index);
        let polygons: Vec<Vec<Vec<[f64; 2]>>> = feature
            .polygons
            .into_iter()
            .map(|polygon| polygon.rings)
            .filter(|rings| !rings.is_empty())
            .collect();
        if polygons.is_empty() {
            continue;
        }
        features.push(PreparedFeature {
            bucket,
            properties: feature.properties,
            polygons,
        });
    }

    let feature_count = features.len() as u32;
    Ok(VectorBuildJob {
        source,
        revision,
        features,
        hover_features: Vec::with_capacity(feature_count as usize),
        next_feature: 0,
        pieces: Vec::new(),
        stats: VectorLayerStats {
            fetched_bytes: bytes.len() as u32,
            feature_count,
            features_processed: 0,
            polygon_count: parsed.stats.polygon_count,
            multipolygon_count: parsed.stats.multipolygon_count,
            hole_ring_count: parsed.stats.hole_ring_count,
            progress: 0.0,
            ..Default::default()
        },
        started_at: Instant::now(),
    })
}

pub fn advance_job(
    job: &mut VectorBuildJob,
    map_to_world: MapToWorld,
    limits: VectorBuildLimits,
) -> Result<AdvanceResult, String> {
    if job.features.is_empty() {
        job.stats.progress = 1.0;
        job.stats.features_processed = 0;
        job.stats.build_ms = job.started_at.elapsed().as_secs_f32() * 1000.0;
        job.stats.last_frame_build_ms = 0.0;
        return Ok(AdvanceResult::Complete);
    }

    let frame_budget_ms = limits.max_build_ms_per_frame.max(0.0);
    if limits.max_features_per_frame == 0 || frame_budget_ms <= 0.0 {
        job.stats.features_processed = job.next_feature as u32;
        job.stats.progress = (job.next_feature as f32 / job.features.len() as f32).clamp(0.0, 1.0);
        job.stats.build_ms = job.started_at.elapsed().as_secs_f32() * 1000.0;
        job.stats.last_frame_build_ms = 0.0;
        return Ok(AdvanceResult::InProgress);
    }
    let frame_budget = Duration::from_secs_f64(frame_budget_ms / 1000.0);
    let frame_start = Instant::now();
    let mut processed = 0usize;

    while job.next_feature < job.features.len() {
        if processed >= limits.max_features_per_frame {
            break;
        }
        if frame_budget > Duration::ZERO && frame_start.elapsed() >= frame_budget {
            break;
        }

        let feature = &job.features[job.next_feature];
        let mut hover_polygons = Vec::new();
        for polygon in &feature.polygons {
            let Some(projected) = project_polygon(polygon, job.source.geometry_space, map_to_world)
            else {
                continue;
            };
            if let Some(mut piece) = triangulate_projected_polygon(&projected)? {
                piece.color_rgba = feature.bucket.rgba;
                let vertex_count = piece.positions.len() as u32;
                let triangle_count = (piece.indices.len() / 3) as u32;
                job.pieces.push(piece);
                job.stats.vertex_count = job.stats.vertex_count.saturating_add(vertex_count);
                job.stats.triangle_count = job.stats.triangle_count.saturating_add(triangle_count);
            }
            hover_polygons.push(projected_polygon_to_hover_polygon(&projected));
        }
        if !hover_polygons.is_empty() {
            let (min_world_x, max_world_x, min_world_z, max_world_z) =
                hover_feature_bounds(&hover_polygons);
            job.hover_features.push(HoverFeature {
                properties: feature.properties.clone(),
                polygons: hover_polygons,
                min_world_x,
                max_world_x,
                min_world_z,
                max_world_z,
            });
        }

        job.next_feature += 1;
        processed += 1;
    }

    job.stats.features_processed = job.next_feature as u32;
    job.stats.progress = (job.next_feature as f32 / job.features.len() as f32).clamp(0.0, 1.0);
    job.stats.build_ms = job.started_at.elapsed().as_secs_f32() * 1000.0;
    job.stats.last_frame_build_ms = frame_start.elapsed().as_secs_f32() * 1000.0;

    if job.next_feature >= job.features.len() {
        Ok(AdvanceResult::Complete)
    } else {
        Ok(AdvanceResult::InProgress)
    }
}

pub fn finalize_job(job: VectorBuildJob, limits: VectorBuildLimits) -> BuiltVectorGeometry {
    crate::perf_scope!("vector.chunk_mesh_build");
    let mut stats = job.stats;
    stats.progress = 1.0;
    stats.features_processed = stats.feature_count;
    stats.build_ms = job.started_at.elapsed().as_secs_f32() * 1000.0;

    let mut chunks = Vec::new();
    let use_chunking = stats.vertex_count as usize > limits.max_chunk_vertices
        || stats.triangle_count as usize > limits.max_chunk_triangles;

    if use_chunking {
        stats.chunked_bucket_count = 1;
        let mut groups: HashMap<(i32, i32), Vec<PolygonPiece>> = HashMap::new();
        for piece in job.pieces {
            let chunk_x = (piece.centroid_map[0] / MAP_CHUNK_PX).floor() as i32;
            let chunk_y = (piece.centroid_map[1] / MAP_CHUNK_PX).floor() as i32;
            groups.entry((chunk_x, chunk_y)).or_default().push(piece);
        }
        for pieces in groups.into_values() {
            if let Some(chunk) = merge_pieces(pieces) {
                chunks.push(chunk);
            }
        }
    } else if let Some(chunk) = merge_pieces(job.pieces) {
        chunks.push(chunk);
    }

    stats.mesh_count = chunks.len() as u32;
    BuiltVectorGeometry {
        chunks,
        hover_features: job.hover_features,
        stats,
    }
}

fn merge_pieces(pieces: Vec<PolygonPiece>) -> Option<BuiltVectorChunk> {
    if pieces.is_empty() {
        return None;
    }

    let mut positions = Vec::new();
    let mut vertex_colors = Vec::new();
    let mut indices = Vec::new();

    for piece in pieces {
        let base_index = positions.len() as u32;
        vertex_colors.extend(std::iter::repeat_n(piece.color_rgba, piece.positions.len()));
        positions.extend(piece.positions);
        indices.extend(piece.indices.into_iter().map(|value| value + base_index));
    }

    if positions.is_empty() || indices.is_empty() {
        return None;
    }

    let mut min_world_x = f32::INFINITY;
    let mut max_world_x = f32::NEG_INFINITY;
    let mut min_world_z = f32::INFINITY;
    let mut max_world_z = f32::NEG_INFINITY;
    for position in &positions {
        min_world_x = min_world_x.min(position[0]);
        max_world_x = max_world_x.max(position[0]);
        min_world_z = min_world_z.min(position[1]);
        max_world_z = max_world_z.max(position[1]);
    }

    Some(BuiltVectorChunk {
        color_rgba: vertex_colors.first().copied().unwrap_or([0, 0, 0, 0]),
        vertex_colors,
        positions,
        indices,
        min_world_x,
        max_world_x,
        min_world_z,
        max_world_z,
    })
}

fn projected_polygon_to_hover_polygon(projected: &ProjectedPolygon) -> HoverPolygon {
    let mut min_world_x = f32::INFINITY;
    let mut max_world_x = f32::NEG_INFINITY;
    let mut min_world_z = f32::INFINITY;
    let mut max_world_z = f32::NEG_INFINITY;
    for ring in &projected.world_rings {
        for point in ring {
            min_world_x = min_world_x.min(point[0]);
            max_world_x = max_world_x.max(point[0]);
            min_world_z = min_world_z.min(point[1]);
            max_world_z = max_world_z.max(point[1]);
        }
    }
    HoverPolygon {
        rings: projected.world_rings.clone(),
        min_world_x,
        max_world_x,
        min_world_z,
        max_world_z,
    }
}

fn hover_feature_bounds(polygons: &[HoverPolygon]) -> (f32, f32, f32, f32) {
    let mut min_world_x = f32::INFINITY;
    let mut max_world_x = f32::NEG_INFINITY;
    let mut min_world_z = f32::INFINITY;
    let mut max_world_z = f32::NEG_INFINITY;
    for polygon in polygons {
        min_world_x = min_world_x.min(polygon.min_world_x);
        max_world_x = max_world_x.max(polygon.max_world_x);
        min_world_z = min_world_z.min(polygon.min_world_z);
        max_world_z = max_world_z.max(polygon.max_world_z);
    }
    (min_world_x, max_world_x, min_world_z, max_world_z)
}

fn spawn_geojson_fetch(url: String) -> Receiver<Result<Vec<u8>, String>> {
    runtime_io::spawn_bytes_request(url)
}

#[cfg(test)]
mod tests {
    use fishystuff_api::models::layers::{
        GeometrySpace as GeometrySpaceDto, LayerDescriptor, LayerKind, LayerTransformDto,
        LayerUiInfo, LayersResponse, LodPolicyDto, StyleMode as StyleModeDto, TilesetRef,
        VectorSourceRef,
    };

    use super::{advance_job, finalize_job, parse_into_job, AdvanceResult, VectorBuildLimits};
    use crate::map::layers::{GeometrySpace, LayerRegistry, StyleMode, VectorSourceSpec};
    use crate::map::spaces::world::MapToWorld;
    use crate::map::spaces::MapPoint;

    fn tiny_source() -> VectorSourceSpec {
        VectorSourceSpec {
            url: "/tests/tiny.geojson".to_string(),
            revision: "tiny-v1".to_string(),
            geometry_space: GeometrySpace::MapPixels,
            style_mode: StyleMode::FeaturePropertyPalette,
            feature_id_property: Some("id".to_string()),
            color_property: Some("c".to_string()),
        }
    }

    #[test]
    fn tiny_fixture_build_completes_incrementally() {
        let bytes = include_bytes!("../../../tests/fixtures/tiny_vector.geojson").to_vec();
        let mut job = parse_into_job(tiny_source(), "tiny-v1".to_string(), bytes).expect("parse");
        let limits = VectorBuildLimits {
            max_features_per_frame: 1,
            max_build_ms_per_frame: 10.0,
            max_chunk_vertices: 10_000,
            max_chunk_triangles: 10_000,
        };

        for _ in 0..32 {
            let result = advance_job(&mut job, MapToWorld::default(), limits).expect("advance");
            if matches!(result, AdvanceResult::Complete) {
                break;
            }
        }

        assert_eq!(job.stats.feature_count, job.stats.features_processed);
        assert!(job.stats.last_frame_build_ms >= 0.0);
        let built = finalize_job(job, limits);
        assert_eq!(built.stats.triangle_count, 6);
        assert_eq!(built.stats.polygon_count, 3);
        assert_eq!(built.stats.multipolygon_count, 1);
        assert_eq!(built.stats.hole_ring_count, 0);
        assert_eq!(built.stats.mesh_count, 1);
    }

    #[test]
    fn advance_job_respects_zero_feature_budget() {
        let bytes = include_bytes!("../../../tests/fixtures/tiny_vector.geojson").to_vec();
        let mut job = parse_into_job(tiny_source(), "tiny-v1".to_string(), bytes).expect("parse");
        let limits = VectorBuildLimits {
            max_features_per_frame: 0,
            max_build_ms_per_frame: 5.0,
            max_chunk_vertices: 10_000,
            max_chunk_triangles: 10_000,
        };

        let result = advance_job(&mut job, MapToWorld::default(), limits).expect("advance");
        assert!(matches!(result, AdvanceResult::InProgress));
        assert_eq!(job.stats.features_processed, 0);
        assert_eq!(job.stats.progress, 0.0);
    }

    #[test]
    fn advance_job_respects_zero_time_budget() {
        let bytes = include_bytes!("../../../tests/fixtures/tiny_vector.geojson").to_vec();
        let mut job = parse_into_job(tiny_source(), "tiny-v1".to_string(), bytes).expect("parse");
        let limits = VectorBuildLimits {
            max_features_per_frame: 32,
            max_build_ms_per_frame: 0.0,
            max_chunk_vertices: 10_000,
            max_chunk_triangles: 10_000,
        };

        let result = advance_job(&mut job, MapToWorld::default(), limits).expect("advance");
        assert!(matches!(result, AdvanceResult::InProgress));
        assert_eq!(job.stats.features_processed, 0);
        assert_eq!(job.stats.last_frame_build_ms, 0.0);
    }

    #[test]
    fn tiny_fixture_map_pixels_smoke_aligns_with_map_to_world() {
        let bytes = include_bytes!("../../../tests/fixtures/tiny_vector.geojson").to_vec();
        let mut job = parse_into_job(tiny_source(), "tiny-v1".to_string(), bytes).expect("parse");
        let limits = VectorBuildLimits {
            max_features_per_frame: 64,
            max_build_ms_per_frame: 20.0,
            max_chunk_vertices: 10_000,
            max_chunk_triangles: 10_000,
        };
        for _ in 0..8 {
            let result = advance_job(&mut job, MapToWorld::default(), limits).expect("advance");
            if matches!(result, AdvanceResult::Complete) {
                break;
            }
        }
        let built = finalize_job(job, limits);
        let expected = MapToWorld::default().map_to_world(MapPoint::new(5100.0, 8200.0));

        let mut found = false;
        for chunk in &built.chunks {
            for pos in &chunk.positions {
                if ((pos[0] as f64) - expected.x).abs() < 0.1
                    && ((pos[1] as f64) - expected.z).abs() < 0.1
                {
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(found, "expected canonical map->world transformed vertex");
    }

    #[test]
    fn metadata_to_ready_build_pipeline_is_deterministic() {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(LayersResponse {
            revision: "rg-v1".to_string(),
            map_version_id: None,
            layers: vec![LayerDescriptor {
                layer_id: "region_groups".to_string(),
                name: "Region Groups".to_string(),
                enabled: true,
                kind: LayerKind::VectorGeoJson,
                transform: LayerTransformDto::IdentityMapSpace,
                tileset: TilesetRef::default(),
                tile_px: 512,
                max_level: 0,
                y_flip: false,
                vector_source: Some(VectorSourceRef {
                    url: "/tests/tiny.geojson".to_string(),
                    revision: "tiny-v1".to_string(),
                    geometry_space: GeometrySpaceDto::MapPixels,
                    style_mode: StyleModeDto::FeaturePropertyPalette,
                    feature_id_property: Some("id".to_string()),
                    color_property: Some("c".to_string()),
                }),
                lod_policy: LodPolicyDto::default(),
                ui: LayerUiInfo::default(),
                request_weight: 1.0,
                pick_mode: "none".to_string(),
            }],
        });
        let source = registry
            .ordered()
            .first()
            .and_then(|layer| layer.vector_source.clone())
            .expect("vector source from metadata");

        let bytes = include_bytes!("../../../tests/fixtures/tiny_vector.geojson").to_vec();
        let mut job = parse_into_job(source, "tiny-v1".to_string(), bytes).expect("parse");
        let limits = VectorBuildLimits {
            max_features_per_frame: 1,
            max_build_ms_per_frame: 8.0,
            max_chunk_vertices: 10_000,
            max_chunk_triangles: 10_000,
        };
        let mut is_complete = false;
        for _ in 0..16 {
            let result = advance_job(&mut job, MapToWorld::default(), limits).expect("advance");
            if matches!(result, AdvanceResult::Complete) {
                is_complete = true;
                break;
            }
        }
        assert!(is_complete, "vector job should complete incrementally");

        let built = finalize_job(job, limits);
        assert_eq!(built.stats.progress, 1.0);
        assert_eq!(built.stats.feature_count, built.stats.features_processed);
        assert!(built.stats.mesh_count > 0);
    }
}
