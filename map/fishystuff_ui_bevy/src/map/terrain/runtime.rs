use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use async_channel::Receiver;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::platform::time::Instant;
use bevy::window::PrimaryWindow;
use fishystuff_core::terrain::{
    chunk_grid_dims_for_level, decode_terrain_chunk, lod_for_view_distance,
    nearest_available_ancestor, DecodedTerrainLevel, TerrainChunkData, TerrainDrapeManifest,
    TerrainHeightEncoding, TerrainManifest,
};
use serde::Deserialize;

use crate::map::camera::map2d::Map2dViewState;
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::camera::terrain3d::{
    apply_terrain3d_camera_state, camera_controls_x_mirrored, estimate_view_world_rect,
    reset_terrain3d_view, Terrain3dViewState,
};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::{WorldPoint, WorldRect};
use crate::map::streaming::TileKey;
use crate::map::terrain::chunks::{visible_chunk_range, TerrainChunkKey, TerrainChunkState};
use crate::map::terrain::height_tiles::{HeightTileEntry, HeightTileKey};
use crate::map::terrain::mesh::build_chunk_mesh_from_data;
use crate::map::terrain::mode::{
    apply_mode_to_camera_and_lighting, clear_camera_control_mutation_flags,
    debug_assert_camera_control_mode_gating, debug_assert_render_isolation,
    ensure_terrain3d_projection, log_camera_activation_state, terrain3d_controls_should_run,
    AppliedViewMode, CameraControlMutationFlags, TerrainLightTag,
};
use crate::map::terrain::Terrain3dConfig;
use crate::plugins::camera::Terrain3dCamera;
use crate::plugins::input::CursorState;
use crate::plugins::render_domain::{world_3d_layers, World3dRenderEntity};
use crate::plugins::ui::UiPointerCapture;
use crate::prelude::*;

mod camera;
mod chunks;
mod diagnostics;
mod drape;
mod manifest;

pub use self::camera::TerrainViewEstimate;

const STALE_RETENTION_FRAMES: u64 = 180;
const ORBIT_SENSITIVITY: f32 = 0.006;
const DOLLY_DRAG_SPEED: f32 = 0.015;
const DOLLY_WHEEL_SPEED: f32 = 0.08;
const DOLLY_DIRECTION: f32 = -1.0;

pub struct Terrain3dPlugin;

impl Plugin for Terrain3dPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewModeState>()
            .init_resource::<Terrain3dConfig>()
            .init_resource::<TerrainRuntime>()
            .init_resource::<Terrain3dViewState>()
            .init_resource::<Map2dViewState>()
            .init_resource::<AppliedViewMode>()
            .init_resource::<camera::OrbitInputState>()
            .init_resource::<CameraControlMutationFlags>()
            .init_resource::<TerrainViewEstimate>()
            .init_resource::<TerrainDiagnostics>()
            .add_systems(
                Startup,
                (camera::initialize_default_mode, camera::spawn_terrain_light),
            )
            .add_systems(PreUpdate, clear_camera_control_mutation_flags)
            .add_systems(
                Update,
                (
                    manifest::invalidate_on_config_change,
                    manifest::ensure_terrain_manifest_request,
                    manifest::poll_terrain_manifest_ready,
                    camera::update_terrain3d_camera_controls,
                    camera::update_view_estimate,
                    chunks::queue_visible_chunks,
                    chunks::build_chunks_incremental,
                    diagnostics::sync_terrain_diagnostics,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                (
                    apply_mode_to_camera_and_lighting,
                    log_camera_activation_state,
                    debug_assert_camera_control_mode_gating,
                    debug_assert_render_isolation,
                    drape::update_draped_tiles,
                )
                    .chain(),
            );
    }
}

#[derive(Component)]
pub struct TerrainChunkEntity {
    pub key: TerrainChunkKey,
}

#[derive(Component)]
pub struct TerrainDrapeEntity {
    pub key: TerrainDrapeKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainDrapeKey {
    Raster(TileKey),
    Chunk(TerrainChunkKey),
}

#[derive(Debug, Clone)]
struct TerrainChunkEntry {
    state: TerrainChunkState,
    entity: Option<Entity>,
    mesh: Option<Handle<Mesh>>,
    chunk: Option<TerrainChunkData>,
    residency: TerrainChunkResidency,
    last_touched: u64,
}

impl Default for TerrainChunkEntry {
    fn default() -> Self {
        Self {
            state: TerrainChunkState::NotRequested,
            entity: None,
            mesh: None,
            chunk: None,
            residency: TerrainChunkResidency::Evictable,
            last_touched: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerrainChunkResidency {
    Protected,
    Warm,
    Evictable,
}

#[derive(Debug, Clone)]
struct TerrainDrapeEntry {
    entity: Entity,
    material: Handle<StandardMaterial>,
    last_touched: u64,
}

#[derive(Debug)]
struct PendingTerrainManifestRequest {
    url: String,
    receiver: Receiver<Result<TerrainManifest, String>>,
}

#[derive(Debug)]
struct PendingTerrainChunkRequest {
    receiver: Receiver<Result<Vec<u8>, String>>,
}

#[derive(Debug, Clone)]
struct LoadedTerrainManifest {
    manifest: TerrainManifest,
    levels: HashMap<u8, DecodedTerrainLevel>,
}

impl LoadedTerrainManifest {
    fn level(&self, level: u8) -> Option<&DecodedTerrainLevel> {
        self.levels.get(&level)
    }

    fn contains(&self, key: TerrainChunkKey) -> bool {
        self.level(key.level())
            .map(|level| level.contains(key.cx(), key.cy()))
            .unwrap_or(false)
    }
}

#[derive(Debug)]
struct PendingTerrainDrapeManifestRequest {
    url: String,
    receiver: Receiver<Result<TerrainDrapeManifest, String>>,
}

#[derive(Debug, Clone)]
struct LoadedTerrainDrapeManifest {
    manifest: TerrainDrapeManifest,
    levels: HashMap<u8, DecodedTerrainLevel>,
}

impl LoadedTerrainDrapeManifest {
    fn level(&self, level: u8) -> Option<&DecodedTerrainLevel> {
        self.levels.get(&level)
    }

    fn contains(&self, key: TerrainChunkKey) -> bool {
        self.level(key.level())
            .map(|level| level.contains(key.cx(), key.cy()))
            .unwrap_or(false)
    }
}

#[derive(Resource, Debug, Clone)]
pub struct TerrainDiagnostics {
    pub enabled: bool,
    pub terrain_ready: bool,
    pub terrain_revision: Option<String>,
    pub manifest_ready: bool,
    pub manifest_failed: bool,
    pub chunk_map_px_runtime: u32,
    pub grid_size_runtime: u16,
    pub max_level_runtime: u8,
    pub bbox_y_min: f32,
    pub bbox_y_max: f32,
    pub show_drape: bool,
    pub chunks_requested: usize,
    pub chunks_building: usize,
    pub chunks_ready: usize,
    pub drape_patch_count: usize,
    pub drape_missing_textures: usize,
    pub drape_min_z: Option<i32>,
    pub drape_max_z: Option<i32>,
    pub avg_build_ms: f32,
    pub camera_pivot: Vec3,
    pub camera_yaw: f32,
    pub camera_pitch: f32,
    pub camera_distance: f32,
    pub visible_chunks_by_level: BTreeMap<u8, u32>,
    pub resident_chunks_by_level: BTreeMap<u8, u32>,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_evictions: u64,
    pub fallback_chunks: u32,
}

impl Default for TerrainDiagnostics {
    fn default() -> Self {
        Self {
            enabled: false,
            terrain_ready: false,
            terrain_revision: None,
            manifest_ready: false,
            manifest_failed: false,
            chunk_map_px_runtime: 0,
            grid_size_runtime: 0,
            max_level_runtime: 0,
            bbox_y_min: 0.0,
            bbox_y_max: 0.0,
            show_drape: true,
            chunks_requested: 0,
            chunks_building: 0,
            chunks_ready: 0,
            drape_patch_count: 0,
            drape_missing_textures: 0,
            drape_min_z: None,
            drape_max_z: None,
            avg_build_ms: 0.0,
            camera_pivot: Vec3::ZERO,
            camera_yaw: 0.0,
            camera_pitch: 0.0,
            camera_distance: 0.0,
            visible_chunks_by_level: BTreeMap::new(),
            resident_chunks_by_level: BTreeMap::new(),
            cache_hits: 0,
            cache_misses: 0,
            cache_evictions: 0,
            fallback_chunks: 0,
        }
    }
}

#[derive(Resource, Default)]
pub struct TerrainRuntime {
    manifest: Option<LoadedTerrainManifest>,
    pending_manifest: Option<PendingTerrainManifestRequest>,
    manifest_failed: bool,
    active_manifest_url: Option<String>,
    active_drape_manifest_url: Option<String>,
    chunks: HashMap<TerrainChunkKey, TerrainChunkEntry>,
    queued_chunks: HashSet<TerrainChunkKey>,
    chunk_queue: VecDeque<TerrainChunkKey>,
    pending_chunks: HashMap<TerrainChunkKey, PendingTerrainChunkRequest>,
    render_chunks: HashSet<TerrainChunkKey>,
    pinned_chunks: HashSet<TerrainChunkKey>,
    visible_chunks_by_level: BTreeMap<u8, u32>,
    resident_chunks_by_level: BTreeMap<u8, u32>,
    fallback_chunks: u32,
    cache_hits: u64,
    cache_misses: u64,
    cache_evictions: u64,
    drape_manifest: Option<LoadedTerrainDrapeManifest>,
    pending_drape_manifest: Option<PendingTerrainDrapeManifestRequest>,
    chunk_drape_entries: HashMap<TerrainChunkKey, TerrainDrapeEntry>,
    pending_chunk_drape_textures: HashMap<TerrainChunkKey, Handle<Image>>,
    queued_chunk_drapes: HashSet<TerrainChunkKey>,
    chunk_drape_queue: VecDeque<TerrainChunkKey>,
    height_tiles: HashMap<HeightTileKey, HeightTileEntry>,
    drape_entries: HashMap<TileKey, TerrainDrapeEntry>,
    queued_drapes: HashSet<TileKey>,
    drape_queue: VecDeque<TileKey>,
    frame: u64,
    avg_build_ms: f32,
    drape_missing_textures: usize,
    drape_min_z: Option<i32>,
    drape_max_z: Option<i32>,
}

impl TerrainRuntime {
    fn clear_all_entities(&mut self, commands: &mut Commands) {
        for entry in self.chunks.values() {
            if let Some(entity) = entry.entity {
                commands.entity(entity).despawn();
            }
        }
        self.chunks.clear();
        self.queued_chunks.clear();
        self.chunk_queue.clear();
        self.pending_chunks.clear();
        self.render_chunks.clear();
        self.pinned_chunks.clear();
        self.visible_chunks_by_level.clear();
        self.resident_chunks_by_level.clear();
        self.fallback_chunks = 0;
        self.active_drape_manifest_url = None;
        self.drape_manifest = None;
        self.pending_drape_manifest = None;
        for entry in self.chunk_drape_entries.values() {
            commands.entity(entry.entity).despawn();
        }
        self.chunk_drape_entries.clear();
        self.pending_chunk_drape_textures.clear();
        self.queued_chunk_drapes.clear();
        self.chunk_drape_queue.clear();
        self.height_tiles.clear();

        for entry in self.drape_entries.values() {
            commands.entity(entry.entity).despawn();
        }
        self.drape_entries.clear();
        self.queued_drapes.clear();
        self.drape_queue.clear();
    }

    fn chunk_counts(&self) -> (usize, usize, usize) {
        let mut requested = 0usize;
        let mut building = 0usize;
        let mut ready = 0usize;
        for entry in self.chunks.values() {
            match entry.state {
                TerrainChunkState::NotRequested => {}
                TerrainChunkState::Building => {
                    requested += 1;
                    building += 1;
                }
                TerrainChunkState::Ready => {
                    requested += 1;
                    ready += 1;
                }
                TerrainChunkState::Failed => {
                    requested += 1;
                }
            }
        }
        (requested, building, ready)
    }

    fn manifest_ready(&self) -> bool {
        self.manifest.is_some()
    }
}
