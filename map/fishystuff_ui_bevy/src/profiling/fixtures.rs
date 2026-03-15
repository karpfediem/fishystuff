use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use bevy::prelude::World;
use fishystuff_api::models::events::EventsSnapshotResponse;
use fishystuff_api::models::fish::FishListResponse;
use fishystuff_api::models::layers::LayersResponse;

use crate::map::events::EventsSnapshotState;
use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::MapPoint;
use crate::map::terrain::Terrain3dConfig;
use crate::plugins::api::{
    ApiBootstrapState, FishCatalog, FishEntry, MapDisplayState, PatchFilterState,
};
use crate::runtime_io;

#[derive(Debug, Clone)]
pub struct FixtureData {
    pub root: PathBuf,
    pub layers: LayersResponse,
    pub fish: FishListResponse,
    pub events_snapshot: EventsSnapshotResponse,
}

impl FixtureData {
    pub fn load(root: &Path) -> Result<Self> {
        let layers = load_json::<LayersResponse>(root, "layers.json")?;
        let fish = load_json::<FishListResponse>(root, "fish_catalog.json")?;
        let events_snapshot = load_json::<EventsSnapshotResponse>(root, "events_snapshot.json")?;
        Ok(Self {
            root: root.to_path_buf(),
            layers,
            fish,
            events_snapshot,
        })
    }

    pub fn seed_world(&self, world: &mut World) {
        runtime_io::set_base_dir(self.root.clone());

        let map_version = self
            .layers
            .map_version_id
            .as_ref()
            .map(|value| value.0.clone())
            .unwrap_or_else(|| "perf-v1".to_string());

        {
            let mut bootstrap = world.resource_mut::<ApiBootstrapState>();
            bootstrap.meta_status = "meta: fixture".to_string();
            bootstrap.layers_status = "layers: fixture".to_string();
            bootstrap.zones_status = "zones: fixture".to_string();
            bootstrap.map_version = Some(map_version.clone());
            bootstrap.layers_loaded_map_version = Some(map_version);
            bootstrap.map_version_dirty = false;
        }

        {
            let mut registry = world.resource_mut::<LayerRegistry>();
            registry.apply_layers_response(self.layers.clone());
        }

        {
            let registry = world.resource::<LayerRegistry>().clone();
            world
                .resource_mut::<LayerRuntime>()
                .reset_from_registry(&registry);
        }

        {
            let mut display = world.resource_mut::<MapDisplayState>();
            display.show_points = false;
            display.show_point_icons = false;
            display.show_zone_mask = true;
            display.zone_mask_opacity = 0.5;
        }

        {
            let mut patch_filter = world.resource_mut::<PatchFilterState>();
            patch_filter.from_ts = Some(1_700_000_000);
            patch_filter.to_ts = Some(1_700_086_400);
        }

        {
            let mut catalog = world.resource_mut::<FishCatalog>();
            catalog.replace(
                self.fish
                    .fish
                    .iter()
                    .map(|entry| {
                        let canonical_id = entry.encyclopedia_key.unwrap_or(entry.item_id);
                        FishEntry {
                            id: canonical_id,
                            item_id: entry.item_id,
                            encyclopedia_key: entry.encyclopedia_key,
                            encyclopedia_id: entry.encyclopedia_id,
                            name: entry.name.clone(),
                            name_lower: entry.name.to_lowercase(),
                            grade: entry.grade.clone(),
                            is_prize: entry.is_prize.unwrap_or(false),
                        }
                    })
                    .collect(),
            );
            catalog.status = "fish: fixture".to_string();
        }

        {
            let mut snapshot = world.resource_mut::<EventsSnapshotState>();
            snapshot.apply_snapshot(self.events_snapshot.clone());
            snapshot.last_meta_poll_at_secs = 0.0;
            snapshot.snapshot_refresh_reason = "fixture".to_string();
        }

        *world.resource_mut::<Terrain3dConfig>() = terrain_config_fixture();
    }
}

fn load_json<T>(root: &Path, relative: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let path = root.join(relative);
    runtime_io::load_json(path.to_string_lossy().as_ref())
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("load fixture {}", path.display()))
}

fn terrain_config_fixture() -> Terrain3dConfig {
    let map_to_world = MapToWorld::default();
    let top_left = map_to_world.map_to_world(MapPoint::new(0.0, 0.0));
    Terrain3dConfig {
        enabled_default: true,
        terrain_manifest_url: "images/terrain/v1/manifest.json".to_string(),
        map_width: 2048,
        map_height: 2048,
        bbox_y_min: -80.0,
        bbox_y_max: 420.0,
        terrain_chunk_requests_per_frame: 8,
        terrain_chunk_max_inflight: 16,
        terrain_cache_max_chunks: 64,
        terrain_pinned_coarse_levels: 1,
        terrain_target_chunks_radius: 4.0,
        use_chunk_aligned_drape: false,
        drape_manifest_url: String::new(),
        height_tile_root_url: "images/terrain_height/v1".to_string(),
        height_tile_size: 1024,
        height_tile_source_width: 2048,
        height_tile_source_height: 2048,
        height_tile_min_tx: 0,
        height_tile_max_tx: 1,
        height_tile_min_ty: 0,
        height_tile_max_ty: 1,
        height_tile_flip_y: false,
        height_tile_world_left: top_left.x as f32,
        height_tile_world_top: top_left.z as f32,
        height_tile_world_units_per_px: map_to_world.distance_per_pixel as f32,
        height_tile_cache_max: 8,
        height_tile_loads_per_frame: 8,
        chunk_map_px: 1024,
        verts_per_chunk_edge: 33,
        build_chunks_per_frame: 4,
        build_ms_per_frame: 4.0,
        show_drape: true,
        drape_subdivisions: 6,
        drape_builds_per_frame: 4,
        drape_offset_base: 10.0,
        drape_offset_per_layer: 0.5,
    }
}
