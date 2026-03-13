use std::collections::HashMap;

use async_channel::Receiver;
use base64::Engine as _;
use bevy::prelude::Resource;
use bevy::tasks::IoTaskPool;
use gloo_net::http::Request;
use serde::Deserialize;

use crate::map::layers::{LayerId, LayerManifestStatus, LayerSpec};
use crate::map::spaces::layer_transform::{LayerTransform, TileSpace};
use crate::map::spaces::world::MapToWorld;

#[derive(Debug, Clone, Deserialize)]
struct RawTilesetManifest {
    #[serde(default)]
    tile_size_px: u32,
    #[serde(default)]
    levels: Vec<RawTilesetLevel>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawTilesetLevel {
    z: u32,
    min_x: i32,
    min_y: i32,
    width: u32,
    height: u32,
    occupancy_b64: String,
    #[serde(default)]
    tile_count: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct LoadedTileset {
    pub tile_px: u32,
    pub max_level: u8,
    pub levels: Vec<LevelInfo>,
}

#[derive(Debug, Clone)]
pub struct LevelInfo {
    pub z: i32,
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
    pub width: u32,
    pub height: u32,
    pub tile_count: usize,
    pub occupancy: Vec<u8>,
}

impl LevelInfo {
    pub(crate) fn contains(&self, x: i32, y: i32) -> bool {
        if x < self.min_x || x > self.max_x || y < self.min_y || y > self.max_y {
            return false;
        }
        let gx = (x - self.min_x) as usize;
        let gy = (y - self.min_y) as usize;
        let idx = gy.saturating_mul(self.width as usize).saturating_add(gx);
        let byte = idx >> 3;
        let mask = 1_u8 << (idx & 7);
        self.occupancy
            .get(byte)
            .map(|value| (value & mask) != 0)
            .unwrap_or(false)
    }

    pub(crate) fn count_in_rect(&self, min_x: i32, max_x: i32, min_y: i32, max_y: i32) -> usize {
        let x0 = min_x.max(self.min_x);
        let x1 = max_x.min(self.max_x);
        let y0 = min_y.max(self.min_y);
        let y1 = max_y.min(self.max_y);
        if x0 > x1 || y0 > y1 {
            return 0;
        }
        let mut count = 0;
        for y in y0..=y1 {
            for x in x0..=x1 {
                if self.contains(x, y) {
                    count += 1;
                }
            }
        }
        count
    }
}

impl LoadedTileset {
    pub(crate) fn level(&self, z: i32) -> Option<&LevelInfo> {
        self.levels.iter().find(|level| level.z == z)
    }
}

#[derive(Debug, Clone)]
enum ManifestState {
    Loading,
    Ready(LoadedTileset),
    Failed,
}

#[derive(Debug, Clone)]
struct ManifestEntry {
    url: String,
    state: ManifestState,
}

#[derive(Resource, Default)]
pub(crate) struct LayerManifestCache {
    entries: HashMap<LayerId, ManifestEntry>,
}

impl LayerManifestCache {
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(crate) fn remove_layer(&mut self, layer: LayerId) {
        self.entries.remove(&layer);
    }

    pub(crate) fn get(&self, layer: LayerId, manifest_url: &str) -> Option<&LoadedTileset> {
        let entry = self.entries.get(&layer)?;
        if entry.url != manifest_url {
            return None;
        }
        match &entry.state {
            ManifestState::Ready(tileset) => Some(tileset),
            _ => None,
        }
    }

    pub(crate) fn status(&self, layer: LayerId, manifest_url: &str) -> LayerManifestStatus {
        let Some(entry) = self.entries.get(&layer) else {
            return LayerManifestStatus::Missing;
        };
        if entry.url != manifest_url {
            return LayerManifestStatus::Missing;
        }
        match entry.state {
            ManifestState::Loading => LayerManifestStatus::Loading,
            ManifestState::Ready(_) => LayerManifestStatus::Ready,
            ManifestState::Failed => LayerManifestStatus::Failed,
        }
    }
}

struct PendingManifestRequest {
    url: String,
    receiver: Receiver<Result<RawTilesetManifest, String>>,
}

#[derive(Resource, Default)]
pub(crate) struct PendingLayerManifests {
    receivers: HashMap<LayerId, PendingManifestRequest>,
}

impl PendingLayerManifests {
    pub(crate) fn clear(&mut self) {
        self.receivers.clear();
    }

    pub(crate) fn remove_layer(&mut self, layer: LayerId) {
        self.receivers.remove(&layer);
    }
}

pub(crate) fn ensure_manifest_request(
    layer_id: LayerId,
    manifest_url: &str,
    manifests: &mut LayerManifestCache,
    pending: &mut PendingLayerManifests,
) {
    if let Some(pending_request) = pending.receivers.get(&layer_id) {
        if pending_request.url == manifest_url {
            manifests.entries.insert(
                layer_id,
                ManifestEntry {
                    url: manifest_url.to_string(),
                    state: ManifestState::Loading,
                },
            );
            return;
        }
        pending.receivers.remove(&layer_id);
    }

    if let Some(entry) = manifests.entries.get(&layer_id) {
        if entry.url == manifest_url {
            return;
        }
    }

    let receiver = spawn_tileset_request(manifest_url.to_string());
    pending.receivers.insert(
        layer_id,
        PendingManifestRequest {
            url: manifest_url.to_string(),
            receiver,
        },
    );
    manifests.entries.insert(
        layer_id,
        ManifestEntry {
            url: manifest_url.to_string(),
            state: ManifestState::Loading,
        },
    );
}

pub(crate) fn poll_manifest_requests(
    manifests: &mut LayerManifestCache,
    pending: &mut PendingLayerManifests,
) {
    let layer_ids: Vec<LayerId> = pending.receivers.keys().copied().collect();
    for layer_id in layer_ids {
        let Some(request) = pending.receivers.get(&layer_id) else {
            continue;
        };
        let Ok(result) = request.receiver.try_recv() else {
            continue;
        };
        let Some(request) = pending.receivers.remove(&layer_id) else {
            continue;
        };
        match result.and_then(decode_tileset) {
            Ok(tileset) => {
                manifests.entries.insert(
                    layer_id,
                    ManifestEntry {
                        url: request.url,
                        state: ManifestState::Ready(tileset),
                    },
                );
            }
            Err(err) => {
                bevy::log::warn!("layer {:?} tileset load failed: {}", layer_id, err);
                manifests.entries.insert(
                    layer_id,
                    ManifestEntry {
                        url: request.url,
                        state: ManifestState::Failed,
                    },
                );
            }
        }
    }
}

fn decode_tileset(raw: RawTilesetManifest) -> Result<LoadedTileset, String> {
    let mut levels = Vec::with_capacity(raw.levels.len());
    for level in raw.levels {
        let width = level.width as usize;
        let height = level.height as usize;
        let expected_len = width.saturating_mul(height).div_ceil(8);
        let occupancy = base64::engine::general_purpose::STANDARD
            .decode(level.occupancy_b64.as_bytes())
            .map_err(|err| format!("decode occupancy for z={}: {}", level.z, err))?;
        if occupancy.len() < expected_len {
            return Err(format!(
                "invalid occupancy for z={}: expected at least {} bytes, got {}",
                level.z,
                expected_len,
                occupancy.len()
            ));
        }
        let inferred_count = occupancy
            .iter()
            .map(|byte| byte.count_ones() as usize)
            .sum::<usize>();
        levels.push(LevelInfo {
            z: level.z as i32,
            min_x: level.min_x,
            min_y: level.min_y,
            max_x: level.min_x + level.width as i32 - 1,
            max_y: level.min_y + level.height as i32 - 1,
            width: level.width,
            height: level.height,
            tile_count: level.tile_count.unwrap_or(inferred_count),
            occupancy,
        });
    }
    levels.sort_by_key(|level| level.z);
    let max_level = levels
        .iter()
        .filter(|level| level.z >= 0)
        .map(|level| level.z as u8)
        .max()
        .unwrap_or(0);
    Ok(LoadedTileset {
        tile_px: raw.tile_size_px.max(1),
        max_level,
        levels,
    })
}

pub fn map_version_id(map_version: &str) -> u64 {
    let mut hash = 14695981039346656037u64;
    for byte in map_version.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

pub(crate) fn layer_map_version<'a>(
    layer: &LayerSpec,
    map_version: Option<&'a str>,
) -> Option<(u64, Option<&'a str>)> {
    if layer_uses_map_version(layer) {
        let version = map_version?;
        Some((map_version_id(version), Some(version)))
    } else {
        Some((0, None))
    }
}

pub(crate) fn layer_tileset_url(layer: &LayerSpec, map_version: Option<&str>) -> String {
    layer
        .tileset_url
        .replace("{map_version}", map_version.unwrap_or(""))
}

pub(crate) fn layer_tile_url(
    layer: &LayerSpec,
    map_version: Option<&str>,
    level: i32,
    x: i32,
    y: i32,
) -> String {
    layer
        .tile_url_template
        .replace("{level}", &level.to_string())
        .replace("{z}", &level.to_string())
        .replace("{x}", &x.to_string())
        .replace("{y}", &y.to_string())
        .replace("{map_version}", map_version.unwrap_or(""))
}

pub(crate) fn layer_uses_map_version(layer: &LayerSpec) -> bool {
    layer.tile_url_template.contains("{map_version}") || layer.tileset_url.contains("{map_version}")
}

pub(crate) fn implicit_identity_tileset(
    layer: &LayerSpec,
    map_to_world: MapToWorld,
) -> Option<LoadedTileset> {
    if !matches!(layer.transform, LayerTransform::IdentityMapSpace) {
        return None;
    }
    let tile_space = TileSpace::new(layer.tile_px, layer.y_flip);
    let mut levels = Vec::new();
    for z in 0..=i32::from(layer.max_level) {
        let span = tile_space.tile_span_px(z)?;
        if span <= 0.0 {
            continue;
        }
        let width = ((map_to_world.image_size_x as f64) / span).ceil() as u32;
        let height = ((map_to_world.image_size_y as f64) / span).ceil() as u32;
        if width == 0 || height == 0 {
            continue;
        }
        let bits = (width as usize).saturating_mul(height as usize);
        let mut occupancy = vec![0_u8; bits.div_ceil(8)];
        for bit in 0..bits {
            occupancy[bit >> 3] |= 1_u8 << (bit & 7);
        }
        levels.push(LevelInfo {
            z,
            min_x: 0,
            min_y: 0,
            max_x: width as i32 - 1,
            max_y: height as i32 - 1,
            width,
            height,
            tile_count: bits,
            occupancy,
        });
    }
    if levels.is_empty() {
        return None;
    }
    Some(LoadedTileset {
        tile_px: layer.tile_px.max(1),
        max_level: layer.max_level,
        levels,
    })
}

fn spawn_tileset_request(url: String) -> Receiver<Result<RawTilesetManifest, String>> {
    let (sender, receiver) = async_channel::bounded(1);
    IoTaskPool::get()
        .spawn_local(async move {
            let result = fetch_json::<RawTilesetManifest>(&url).await;
            let _ = sender.send(result).await;
        })
        .detach();
    receiver
}

async fn fetch_json<T>(url: &str) -> Result<T, String>
where
    for<'de> T: Deserialize<'de> + Send + 'static,
{
    let resp = Request::get(url)
        .send()
        .await
        .map_err(|err| err.to_string())?;
    if !resp.ok() {
        return Err(format!("{}: {}", url, resp.status()));
    }
    resp.json::<T>().await.map_err(|err| err.to_string())
}
