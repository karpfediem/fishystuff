use std::collections::{BTreeMap, HashMap};

use bevy::prelude::Image;

use crate::config::TILE_CACHE_MAX;
use crate::map::layers::{LayerId, LayerRegistry};
use crate::prelude::*;

use super::manifest::LoadedTileset;
use super::policy::{
    eviction_priority_score, incr_level_count, tile_residency_class, ResidencyClass,
    TileResidencyState,
};
use super::TileKey;

mod filters;
mod render;

#[derive(Component, Debug)]
pub struct RasterTileEntity;

#[derive(Resource, Default)]
pub struct TileStats {
    pub visible_tiles: u32,
    pub requested_tiles: u32,
    pub inflight: usize,
    pub queue_len: usize,
    pub view_min: Option<(f32, f32)>,
    pub view_max: Option<(f32, f32)>,
    pub cursor_world: Option<(f32, f32)>,
    pub cursor_map: Option<(f32, f32)>,
    pub cache_hits: u32,
    pub cache_evictions: u32,
    pub cache_hits_by_level: HashMap<LayerId, BTreeMap<i32, u32>>,
    pub cache_misses_by_level: HashMap<LayerId, BTreeMap<i32, u32>>,
    pub cache_evictions_by_level: HashMap<LayerId, BTreeMap<i32, u32>>,
    pub resident_by_level: HashMap<LayerId, BTreeMap<i32, u32>>,
    pub protected_by_level: HashMap<LayerId, BTreeMap<i32, u32>>,
    pub warm_by_level: HashMap<LayerId, BTreeMap<i32, u32>>,
    pub fallback_visible_by_level: HashMap<LayerId, BTreeMap<i32, u32>>,
    pub blank_visible_by_layer: HashMap<LayerId, u32>,
    pub fallback_visible_tiles: u32,
    pub blank_visible_tiles: u32,
    pub requests_suppressed_motion: u32,
    pub detail_requests_started: u32,
    pub coverage_requests_started: u32,
    pub detail_requests_queued: u32,
    pub coverage_requests_queued: u32,
    pub camera_unstable: bool,
    pub camera_pan_fraction: f32,
    pub camera_zoom_out_ratio: f32,
    pub(crate) last_log: f64,
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct TileDebugControls {
    pub disable_eviction: bool,
}

impl Default for TileDebugControls {
    fn default() -> Self {
        Self {
            disable_eviction: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TilePixelData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ReadyRasterTile {
    pub key: TileKey,
    pub texture: Handle<Image>,
    pub alpha: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TileState {
    Loading,
    Ready,
    Failed,
}

#[derive(Debug)]
pub(crate) struct RasterTileEntry {
    pub(crate) handle: Handle<Image>,
    pub(crate) entity: Option<Entity>,
    pub(crate) material: Option<Handle<ColorMaterial>>,
    pub(crate) state: TileState,
    pub(crate) visible: bool,
    pub(crate) alpha: f32,
    pub(crate) depth: f32,
    pub(crate) last_used: u64,
    pub(crate) exact_quad: bool,
    pub(crate) sprite_size: Option<Vec2>,
    pub(crate) pixel_data: Option<TilePixelData>,
    pub(crate) zone_rgbs: Vec<u32>,
    pub(crate) filter_active: bool,
    pub(crate) filter_revision: u64,
    pub(crate) pixel_filtered: bool,
    pub(crate) hover_highlight_zone: Option<u32>,
    pub(crate) clip_mask_layer: Option<LayerId>,
    pub(crate) clip_mask_revision: u64,
    pub(crate) clip_mask_applied: bool,
    pub(crate) linger_until_frame: u64,
}

#[derive(Resource)]
pub struct RasterTileCache {
    pub(crate) entries: HashMap<TileKey, RasterTileEntry>,
    pub(crate) use_counter: u64,
    pub(crate) max_entries: usize,
}

impl Default for RasterTileCache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            use_counter: 0,
            max_entries: TILE_CACHE_MAX,
        }
    }
}

impl RasterTileCache {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn contains(&self, key: &TileKey) -> bool {
        self.entries.contains_key(key)
    }

    pub fn ready_visible_tiles(&self) -> Vec<ReadyRasterTile> {
        self.entries
            .iter()
            .filter_map(|(key, entry)| {
                if entry.state == TileState::Ready && entry.visible {
                    Some(ReadyRasterTile {
                        key: *key,
                        texture: entry.handle.clone(),
                        alpha: entry.alpha,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    pub(crate) fn contains_ready(&self, key: &TileKey) -> bool {
        self.entries
            .get(key)
            .map(|entry| entry.state == TileState::Ready)
            .unwrap_or(false)
    }

    pub(crate) fn nearest_loaded_ancestor(
        &self,
        key: TileKey,
        tileset: &LoadedTileset,
    ) -> Option<TileKey> {
        let mut tx = key.tx;
        let mut ty = key.ty;
        let mut current_z = key.z;
        for level in tileset.levels.iter().filter(|level| level.z > key.z) {
            while current_z < level.z {
                tx = tx.div_euclid(2);
                ty = ty.div_euclid(2);
                current_z += 1;
            }
            if !level.contains(tx, ty) {
                continue;
            }
            let ancestor = TileKey {
                layer: key.layer,
                map_version: key.map_version,
                z: level.z,
                tx,
                ty,
            };
            if self.contains_ready(&ancestor) {
                return Some(ancestor);
            }
        }
        None
    }

    pub fn get_ready_pixel_data(&self, key: &TileKey) -> Option<&TilePixelData> {
        let entry = self.entries.get(key)?;
        if entry.state != TileState::Ready {
            return None;
        }
        entry.pixel_data.as_ref()
    }

    pub(crate) fn insert_loading(
        &mut self,
        key: TileKey,
        handle: Handle<Image>,
        visible: bool,
        alpha: f32,
    ) {
        self.use_counter = self.use_counter.wrapping_add(1);
        self.entries.insert(
            key,
            RasterTileEntry {
                handle,
                entity: None,
                material: None,
                state: TileState::Loading,
                visible,
                alpha,
                depth: 0.0,
                last_used: self.use_counter,
                exact_quad: false,
                sprite_size: None,
                pixel_data: None,
                zone_rgbs: Vec::new(),
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

    pub(crate) fn clear_layer(
        &mut self,
        layer: LayerId,
        commands: &mut Commands,
        images: &mut Assets<Image>,
    ) {
        let keys: Vec<TileKey> = self
            .entries
            .keys()
            .filter(|key| key.layer == layer)
            .copied()
            .collect();
        for key in keys {
            if let Some(entry) = self.entries.remove(&key) {
                if let Some(entity) = entry.entity {
                    commands.entity(entity).despawn();
                }
                images.remove(entry.handle.id());
            }
        }
    }

    pub(crate) fn clear_all(&mut self, commands: &mut Commands, images: &mut Assets<Image>) {
        let keys: Vec<TileKey> = self.entries.keys().copied().collect();
        for key in keys {
            if let Some(entry) = self.entries.remove(&key) {
                if let Some(entity) = entry.entity {
                    commands.entity(entity).despawn();
                }
                images.remove(entry.handle.id());
            }
        }
    }

    pub(crate) fn evict(
        &mut self,
        commands: &mut Commands,
        images: &mut Assets<Image>,
        stats: &mut TileStats,
        residency: &TileResidencyState,
        layer_registry: &LayerRegistry,
    ) {
        crate::perf_scope!("raster.eviction_policy");
        if self.entries.len() <= self.max_entries {
            return;
        }

        let mut evictable: Vec<(f64, TileKey)> = Vec::new();
        let mut warm: Vec<(f64, TileKey)> = Vec::new();

        for (key, entry) in &self.entries {
            let class = tile_residency_class(*key, residency);
            if class == ResidencyClass::Protected {
                continue;
            }
            if entry.state == TileState::Loading {
                continue;
            }
            let score = if entry.state == TileState::Failed {
                f64::MAX
            } else {
                eviction_priority_score(
                    self.use_counter,
                    *key,
                    entry,
                    residency,
                    layer_registry.get(key.layer),
                )
            };
            match class {
                ResidencyClass::Warm => warm.push((score, *key)),
                ResidencyClass::Evictable => evictable.push((score, *key)),
                ResidencyClass::Protected => {}
            }
        }

        evictable.sort_by(|lhs, rhs| rhs.0.total_cmp(&lhs.0));
        warm.sort_by(|lhs, rhs| rhs.0.total_cmp(&lhs.0));
        let mut order = Vec::with_capacity(evictable.len() + warm.len());
        order.extend(evictable.into_iter().map(|(_, key)| key));
        order.extend(warm.into_iter().map(|(_, key)| key));

        let remove_count = self.entries.len() - self.max_entries;
        for key in order.into_iter().take(remove_count) {
            if let Some(entry) = self.entries.remove(&key) {
                if let Some(entity) = entry.entity {
                    commands.entity(entity).despawn();
                }
                images.remove(entry.handle.id());
                stats.cache_evictions = stats.cache_evictions.saturating_add(1);
                incr_level_count(&mut stats.cache_evictions_by_level, key.layer, key.z, 1);
            }
        }
        crate::perf_counter_add!("raster.cache_evictions", remove_count);
    }

    pub(crate) fn resident_count_by_layer(&self, layer: LayerId) -> u32 {
        self.entries
            .iter()
            .filter(|(key, entry)| key.layer == layer && entry.state == TileState::Ready)
            .count() as u32
    }

    pub(crate) fn inflight_count_by_layer(&self, layer: LayerId) -> u32 {
        self.entries
            .iter()
            .filter(|(key, entry)| key.layer == layer && entry.state == TileState::Loading)
            .count() as u32
    }

    pub(crate) fn inflight_count_total(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| entry.state == TileState::Loading)
            .count()
    }

    pub(crate) fn resident_counts_by_layer_level(&self) -> HashMap<LayerId, BTreeMap<i32, u32>> {
        let mut counts: HashMap<LayerId, BTreeMap<i32, u32>> = HashMap::new();
        for (key, entry) in &self.entries {
            if entry.state != TileState::Ready {
                continue;
            }
            let per_layer = counts.entry(key.layer).or_default();
            *per_layer.entry(key.z).or_default() += 1;
        }
        counts
    }
}
