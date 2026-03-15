use std::collections::HashMap;

use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::{MapPoint, WorldPoint};
use crate::map::terrain::Terrain3dConfig;
use crate::prelude::*;
use crate::runtime_io;
use async_channel::{Receiver, TryRecvError};
use fishystuff_core::terrain::{
    packed_rgb24_norm_from_rgb, world_height_from_normalized, TerrainManifest,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeightTileKey {
    pub tx: i32,
    pub ty: i32,
}

#[derive(Debug)]
pub struct HeightTileEntry {
    pub state: HeightTileState,
    pub last_touched: u64,
}

#[derive(Debug)]
pub enum HeightTileState {
    Pending(Receiver<Result<DecodedHeightTile, String>>),
    Ready(DecodedHeightTile),
    Failed,
}

#[derive(Debug, Clone)]
pub struct DecodedHeightTile {
    width: u32,
    height: u32,
    rgb: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
pub struct HeightTileLoadBudget {
    remaining: usize,
}

impl HeightTileLoadBudget {
    pub fn new(remaining: usize) -> Self {
        Self { remaining }
    }

    fn try_take(&mut self) -> bool {
        if self.remaining == 0 {
            return false;
        }
        self.remaining -= 1;
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeightTileSampleError {
    MissingTile(HeightTileKey),
    FailedTile(HeightTileKey),
}

impl HeightTileSampleError {
    pub fn key(self) -> HeightTileKey {
        match self {
            Self::MissingTile(key) | Self::FailedTile(key) => key,
        }
    }
}

pub fn sample_world_height(
    cache: &mut HashMap<HeightTileKey, HeightTileEntry>,
    frame: u64,
    load_budget: &mut HeightTileLoadBudget,
    config: &Terrain3dConfig,
    manifest: &TerrainManifest,
    map_x: f32,
    map_y: f32,
) -> Result<f32, HeightTileSampleError> {
    let norm = sample_normalized_height(cache, frame, load_budget, config, manifest, map_x, map_y)?;
    Ok(world_height_from_normalized(
        norm,
        manifest.bbox_y_min,
        manifest.bbox_y_max,
    ))
}

pub fn evict_stale_height_tiles(
    cache: &mut HashMap<HeightTileKey, HeightTileEntry>,
    frame: u64,
    max_entries: usize,
) {
    if cache.len() <= max_entries.max(16) {
        return;
    }

    let mut keys = cache
        .iter()
        .map(|(key, entry)| (*key, entry.last_touched))
        .collect::<Vec<_>>();
    keys.sort_by_key(|(_, last_touched)| *last_touched);

    let remove_count = cache.len().saturating_sub(max_entries.max(16));
    for (key, _) in keys.into_iter().take(remove_count) {
        let stale = cache
            .get(&key)
            .map(|entry| frame.saturating_sub(entry.last_touched) > 1)
            .unwrap_or(false);
        if !stale {
            continue;
        }
        cache.remove(&key);
    }
}

fn sample_normalized_height(
    cache: &mut HashMap<HeightTileKey, HeightTileEntry>,
    frame: u64,
    load_budget: &mut HeightTileLoadBudget,
    config: &Terrain3dConfig,
    manifest: &TerrainManifest,
    map_x: f32,
    map_y: f32,
) -> Result<f32, HeightTileSampleError> {
    let clamped_x = map_x.clamp(0.0, manifest.map_width.saturating_sub(1) as f32);
    let clamped_y = map_y.clamp(0.0, manifest.map_height.saturating_sub(1) as f32);
    let map_to_world = MapToWorld::default();
    let world = map_to_world.map_to_world(MapPoint::new(clamped_x as f64, clamped_y as f64));
    let (src_x, src_y) = world_to_source_px(config, world);
    let src_w_max = config.height_tile_source_width.saturating_sub(1).max(1) as f32;
    let src_h_max = config.height_tile_source_height.saturating_sub(1).max(1) as f32;
    let src_x = src_x.clamp(0.0, src_w_max);
    let src_y = src_y.clamp(0.0, src_h_max);

    let x0 = src_x.floor() as i32;
    let y0 = src_y.floor() as i32;
    let x1 = (x0 + 1).min(config.height_tile_source_width as i32 - 1);
    let y1 = (y0 + 1).min(config.height_tile_source_height as i32 - 1);
    let tx = (src_x - x0 as f32).clamp(0.0, 1.0);
    let ty = (src_y - y0 as f32).clamp(0.0, 1.0);

    let h00 = sample_norm_texel(cache, frame, load_budget, config, x0, y0)?;
    let h10 = sample_norm_texel(cache, frame, load_budget, config, x1, y0)?;
    let h01 = sample_norm_texel(cache, frame, load_budget, config, x0, y1)?;
    let h11 = sample_norm_texel(cache, frame, load_budget, config, x1, y1)?;

    Ok(bilerp_scalar(h00, h10, h01, h11, tx, ty))
}

fn world_to_source_px(config: &Terrain3dConfig, world: WorldPoint) -> (f32, f32) {
    let units_per_px = config.height_tile_world_units_per_px.max(1.0);
    let src_x = (world.x as f32 - config.height_tile_world_left) / units_per_px;
    let src_y = (config.height_tile_world_top - world.z as f32) / units_per_px;
    (src_x, src_y)
}

fn sample_norm_texel(
    cache: &mut HashMap<HeightTileKey, HeightTileEntry>,
    frame: u64,
    load_budget: &mut HeightTileLoadBudget,
    config: &Terrain3dConfig,
    src_x: i32,
    src_y: i32,
) -> Result<f32, HeightTileSampleError> {
    let rgb = sample_rgb_texel(cache, frame, load_budget, config, src_x, src_y)?;
    Ok(packed_rgb24_norm_from_rgb(rgb))
}

fn sample_rgb_texel(
    cache: &mut HashMap<HeightTileKey, HeightTileEntry>,
    frame: u64,
    load_budget: &mut HeightTileLoadBudget,
    config: &Terrain3dConfig,
    src_x: i32,
    src_y: i32,
) -> Result<[u8; 3], HeightTileSampleError> {
    let tile_size = config.height_tile_size.max(1) as i32;
    let tile_offset_x = src_x.div_euclid(tile_size);
    let tile_offset_y = src_y.div_euclid(tile_size);
    let key = HeightTileKey {
        tx: config.height_tile_min_tx + tile_offset_x,
        ty: if config.height_tile_flip_y {
            config.height_tile_max_ty - tile_offset_y
        } else {
            config.height_tile_min_ty + tile_offset_y
        },
    };
    let local_x = src_x.rem_euclid(tile_size) as u32;
    let local_y = src_y.rem_euclid(tile_size) as u32;

    if let std::collections::hash_map::Entry::Vacant(entry) = cache.entry(key) {
        if !load_budget.try_take() {
            return Err(HeightTileSampleError::MissingTile(key));
        }
        entry.insert(HeightTileEntry {
            state: HeightTileState::Pending(spawn_height_tile_request(config, key)),
            last_touched: frame,
        });
    }

    let Some(entry) = cache.get_mut(&key) else {
        return Err(HeightTileSampleError::MissingTile(key));
    };
    entry.last_touched = frame;

    match &mut entry.state {
        HeightTileState::Pending(receiver) => match receiver.try_recv() {
            Ok(Ok(tile)) => {
                entry.state = HeightTileState::Ready(tile);
            }
            Ok(Err(err)) => {
                bevy::log::warn!(
                    "terrain height tile fetch failed for {}_{}: {}",
                    key.tx,
                    key.ty,
                    err
                );
                entry.state = HeightTileState::Failed;
                return Err(HeightTileSampleError::FailedTile(key));
            }
            Err(TryRecvError::Empty) => {
                return Err(HeightTileSampleError::MissingTile(key));
            }
            Err(TryRecvError::Closed) => {
                bevy::log::warn!(
                    "terrain height tile request closed for {}_{}",
                    key.tx,
                    key.ty
                );
                entry.state = HeightTileState::Failed;
                return Err(HeightTileSampleError::FailedTile(key));
            }
        },
        HeightTileState::Ready(_) => {}
        HeightTileState::Failed => {
            return Err(HeightTileSampleError::FailedTile(key));
        }
    }

    let HeightTileState::Ready(tile) = &entry.state else {
        return Err(HeightTileSampleError::MissingTile(key));
    };
    tile.sample(local_x, local_y)
        .ok_or(HeightTileSampleError::FailedTile(key))
}

impl DecodedHeightTile {
    fn sample(&self, x: u32, y: u32) -> Option<[u8; 3]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = (y as usize * self.width as usize + x as usize) * 3;
        if idx + 2 >= self.rgb.len() {
            return None;
        }
        Some([self.rgb[idx], self.rgb[idx + 1], self.rgb[idx + 2]])
    }
}

fn bilerp_scalar(h00: f32, h10: f32, h01: f32, h11: f32, tx: f32, ty: f32) -> f32 {
    let top = h00 + (h10 - h00) * tx;
    let bottom = h01 + (h11 - h01) * tx;
    top + (bottom - top) * ty
}

fn spawn_height_tile_request(
    config: &Terrain3dConfig,
    key: HeightTileKey,
) -> Receiver<Result<DecodedHeightTile, String>> {
    let root = config.height_tile_root_url.trim_end_matches('/');
    let url = format!("{root}/{}_{}.png", key.tx, key.ty);
    let (sender, receiver) = async_channel::bounded(1);

    #[cfg(target_arch = "wasm32")]
    bevy::tasks::IoTaskPool::get()
        .spawn_local(async move {
            let result = runtime_io::load_bytes_async(&url).await.and_then(|bytes| {
                decode_height_tile(bytes.as_slice()).map_err(|err| format!("decode {url}: {err}"))
            });
            let _ = sender.send(result).await;
        })
        .detach();

    #[cfg(not(target_arch = "wasm32"))]
    bevy::tasks::IoTaskPool::get()
        .spawn(async move {
            let result = runtime_io::load_bytes_async(&url).await.and_then(|bytes| {
                decode_height_tile(bytes.as_slice()).map_err(|err| format!("decode {url}: {err}"))
            });
            let _ = sender.send(result).await;
        })
        .detach();
    receiver
}

fn decode_height_tile(bytes: &[u8]) -> Result<DecodedHeightTile, String> {
    crate::perf_scope!("terrain.height_tile_decode");
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|err| err.to_string())?;
    let rgb = image.to_rgb8();
    Ok(DecodedHeightTile {
        width: rgb.width(),
        height: rgb.height(),
        rgb: rgb.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use super::world_to_source_px;
    use crate::map::spaces::WorldPoint;
    use crate::map::terrain::Terrain3dConfig;

    #[test]
    fn fullres_world_transform_matches_raster_extents() {
        let config = Terrain3dConfig::default();

        let (left, top) = world_to_source_px(
            &config,
            WorldPoint::new(
                config.height_tile_world_left as f64,
                config.height_tile_world_top as f64,
            ),
        );
        assert!((left - 0.0).abs() < 1e-6);
        assert!((top - 0.0).abs() < 1e-6);

        let right_world = config.height_tile_world_left
            + (config.height_tile_source_width - 1) as f32 * config.height_tile_world_units_per_px;
        let bottom_world = config.height_tile_world_top
            - (config.height_tile_source_height - 1) as f32 * config.height_tile_world_units_per_px;
        let (right, bottom) = world_to_source_px(
            &config,
            WorldPoint::new(right_world as f64, bottom_world as f64),
        );
        assert!((right - (config.height_tile_source_width - 1) as f32).abs() < 1e-6);
        assert!((bottom - (config.height_tile_source_height - 1) as f32).abs() < 1e-6);
    }
}
