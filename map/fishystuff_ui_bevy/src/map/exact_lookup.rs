use std::collections::HashMap;

use async_channel::Receiver;
use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::Resource;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use fishystuff_api::Rgb;
use fishystuff_core::masks::ZoneLookupRows;

use crate::map::layers::{FieldColorMode, LayerId, LayerSpec};
use crate::map::raster::TileKey;
use crate::map::spaces::layer_transform::LayerTransform;
use crate::runtime_io;

#[derive(Debug, Clone)]
enum ExactLookupState {
    Loading,
    Ready(ZoneLookupRows),
    Failed,
}

#[derive(Debug, Clone)]
struct ExactLookupEntry {
    url: String,
    state: ExactLookupState,
}

struct PendingExactLookupRequest {
    url: String,
    receiver: Receiver<Result<Vec<u8>, String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactLookupStatus {
    Missing,
    Loading,
    Ready,
    Failed,
}

#[derive(Resource, Default)]
pub struct ExactLookupCache {
    entries: HashMap<LayerId, ExactLookupEntry>,
}

#[derive(Resource, Default)]
pub struct PendingExactLookups {
    receivers: HashMap<LayerId, PendingExactLookupRequest>,
}

impl ExactLookupCache {
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn remove_layer(&mut self, layer: LayerId) {
        self.entries.remove(&layer);
    }

    pub fn get(&self, layer: LayerId, url: &str) -> Option<&ZoneLookupRows> {
        let entry = self.entries.get(&layer)?;
        if entry.url != url {
            return None;
        }
        match &entry.state {
            ExactLookupState::Ready(lookup) => Some(lookup),
            ExactLookupState::Loading | ExactLookupState::Failed => None,
        }
    }

    pub fn status(&self, layer: LayerId, url: &str) -> ExactLookupStatus {
        let Some(entry) = self.entries.get(&layer) else {
            return ExactLookupStatus::Missing;
        };
        if entry.url != url {
            return ExactLookupStatus::Missing;
        }
        match entry.state {
            ExactLookupState::Loading => ExactLookupStatus::Loading,
            ExactLookupState::Ready(_) => ExactLookupStatus::Ready,
            ExactLookupState::Failed => ExactLookupStatus::Failed,
        }
    }

    pub fn layer_ids(&self) -> Vec<LayerId> {
        self.entries.keys().copied().collect()
    }
}

impl PendingExactLookups {
    pub fn clear(&mut self) {
        self.receivers.clear();
    }

    pub fn remove_layer(&mut self, layer: LayerId) {
        self.receivers.remove(&layer);
    }

    pub fn layer_ids(&self) -> Vec<LayerId> {
        self.receivers.keys().copied().collect()
    }
}

pub fn ensure_exact_lookup_request(
    layer: &LayerSpec,
    lookups: &mut ExactLookupCache,
    pending: &mut PendingExactLookups,
) {
    let Some(url) = layer.field_url() else {
        lookups.remove_layer(layer.id);
        pending.remove_layer(layer.id);
        return;
    };

    if let Some(request) = pending.receivers.get(&layer.id) {
        if request.url == url {
            lookups.entries.insert(
                layer.id,
                ExactLookupEntry {
                    url,
                    state: ExactLookupState::Loading,
                },
            );
            return;
        }
        pending.receivers.remove(&layer.id);
    }

    if let Some(entry) = lookups.entries.get(&layer.id) {
        if entry.url == url {
            return;
        }
    }

    let receiver = runtime_io::spawn_bytes_request(url.clone());
    pending.receivers.insert(
        layer.id,
        PendingExactLookupRequest {
            url: url.clone(),
            receiver,
        },
    );
    lookups.entries.insert(
        layer.id,
        ExactLookupEntry {
            url,
            state: ExactLookupState::Loading,
        },
    );
}

pub fn poll_exact_lookup_requests(
    lookups: &mut ExactLookupCache,
    pending: &mut PendingExactLookups,
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
        match result.and_then(|bytes| {
            ZoneLookupRows::from_bytes(&bytes)
                .map_err(|err| format!("decode {}: {}", request.url, err))
        }) {
            Ok(lookup) => {
                lookups.entries.insert(
                    layer_id,
                    ExactLookupEntry {
                        url: request.url,
                        state: ExactLookupState::Ready(lookup),
                    },
                );
            }
            Err(err) => {
                bevy::log::warn!("layer {:?} exact lookup load failed: {}", layer_id, err);
                lookups.entries.insert(
                    layer_id,
                    ExactLookupEntry {
                        url: request.url,
                        state: ExactLookupState::Failed,
                    },
                );
            }
        }
    }
}

pub fn sample_exact_lookup_rgb(
    layer: &LayerSpec,
    lookups: &ExactLookupCache,
    map_px_x: i32,
    map_px_y: i32,
) -> Option<Rgb> {
    if layer.field_color_mode() != Some(FieldColorMode::RgbU24) {
        return None;
    }
    sample_field_layer_rgb(layer, lookups, map_px_x, map_px_y)
}

pub fn sample_field_layer_rgb(
    layer: &LayerSpec,
    lookups: &ExactLookupCache,
    map_px_x: i32,
    map_px_y: i32,
) -> Option<Rgb> {
    let color_mode = layer.field_color_mode()?;
    let id = sample_field_layer_id_u32(layer, lookups, map_px_x, map_px_y)?;
    let [r, g, b] = rgb_bytes_for_field_id(id, color_mode);
    Some(Rgb::new(r, g, b))
}

pub fn sample_field_layer_id_u32(
    layer: &LayerSpec,
    lookups: &ExactLookupCache,
    map_px_x: i32,
    map_px_y: i32,
) -> Option<u32> {
    let url = layer.field_url()?;
    let lookup = lookups.get(layer.id, &url)?;
    lookup.cell_id_u32(map_px_x, map_px_y)
}

pub fn render_exact_lookup_tile_image(
    layer: &LayerSpec,
    lookups: &ExactLookupCache,
    key: TileKey,
) -> Option<Image> {
    if !matches!(layer.transform, LayerTransform::IdentityMapSpace) {
        return None;
    }
    if layer.y_flip || key.z < 0 {
        return None;
    }
    let url = layer.field_url()?;
    if layer.field_color_mode() != Some(FieldColorMode::RgbU24) {
        return None;
    }
    let lookup = lookups.get(layer.id, &url)?;
    let scale = 1_u32.checked_shl(key.z as u32)?;
    let source_span = layer.tile_px.checked_mul(scale)?;
    if source_span == 0 {
        return None;
    }
    let source_origin_x = key.tx.checked_mul(source_span as i32)?;
    let source_origin_y = key.ty.checked_mul(source_span as i32)?;
    let visible_source_width =
        (i32::from(lookup.width()) - source_origin_x).clamp(0, source_span as i32) as u32;
    let visible_source_height =
        (i32::from(lookup.height()) - source_origin_y).clamp(0, source_span as i32) as u32;
    if visible_source_width == 0 || visible_source_height == 0 {
        return None;
    }

    let output_width = visible_source_width.div_ceil(scale) as u16;
    let output_height = visible_source_height.div_ceil(scale) as u16;
    let chunk = lookup.render_rgba_resampled_chunk(
        source_origin_x,
        source_origin_y,
        visible_source_width,
        visible_source_height,
        output_width,
        output_height,
        |rgb| {
            [
                ((rgb >> 16) & 0xff) as u8,
                ((rgb >> 8) & 0xff) as u8,
                (rgb & 0xff) as u8,
                255,
            ]
        },
    );

    Some(Image::new(
        Extent3d {
            width: u32::from(chunk.width()),
            height: u32::from(chunk.height()),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        chunk.into_data(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ))
}

fn rgb_bytes_for_field_id(id: u32, color_mode: FieldColorMode) -> [u8; 3] {
    match color_mode {
        FieldColorMode::RgbU24 => [
            ((id >> 16) & 0xff) as u8,
            ((id >> 8) & 0xff) as u8,
            (id & 0xff) as u8,
        ],
        FieldColorMode::DebugHash => {
            let hash = hash_u32(id);
            [
                ((hash >> 16) & 0xff) as u8,
                ((hash >> 8) & 0xff) as u8,
                (hash & 0xff) as u8,
            ]
        }
    }
}

fn hash_u32(mut value: u32) -> u32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::layers::{LayerKind, LayerSpec, LodPolicy, PickMode};
    use crate::map::raster::TileKey;
    use crate::map::spaces::layer_transform::LayerTransform;
    use fishystuff_core::masks::ZoneMask;

    fn test_layer() -> LayerSpec {
        LayerSpec {
            id: LayerId::from_raw(7),
            key: "zone_mask".to_string(),
            name: "Zone Mask".to_string(),
            visible_default: true,
            opacity_default: 1.0,
            z_base: 0.0,
            kind: LayerKind::TiledRaster,
            tileset_url: "/images/tiles/mask/v1/tileset.json".to_string(),
            tile_url_template: "/images/tiles/mask/v1/{level}/{x}_{y}.png".to_string(),
            tileset_version: "v1".to_string(),
            field_source: None,
            vector_source: None,
            transform: LayerTransform::IdentityMapSpace,
            tile_px: 512,
            max_level: 0,
            y_flip: false,
            lod_policy: LodPolicy {
                target_tiles: 64,
                hysteresis_hi: 80.0,
                hysteresis_lo: 40.0,
                margin_tiles: 0,
                enable_refine: false,
                refine_debounce_ms: 0,
                max_detail_tiles: 128,
                max_resident_tiles: 256,
                pinned_coarse_levels: 0,
                coarse_pin_min_level: None,
                warm_margin_tiles: 1,
                protected_margin_tiles: 0,
                detail_eviction_weight: 4.0,
                max_detail_requests_while_camera_moving: 1,
                motion_suppresses_refine: true,
            },
            request_weight: 1.0,
            pick_mode: PickMode::ExactTilePixel,
            display_order: 0,
        }
    }

    #[test]
    fn sample_exact_lookup_rgb_uses_ready_lookup_asset() {
        let layer = test_layer();
        let mask = ZoneMask::from_rgb(
            2,
            2,
            vec![
                1, 2, 3, 4, 5, 6, //
                7, 8, 9, 10, 11, 12,
            ],
        )
        .expect("mask");
        let lookup = mask.to_lookup_rows().expect("lookup");
        let url = layer.field_url().expect("lookup url");
        let mut cache = ExactLookupCache::default();
        cache.entries.insert(
            layer.id,
            ExactLookupEntry {
                url,
                state: ExactLookupState::Ready(lookup),
            },
        );

        assert_eq!(
            sample_exact_lookup_rgb(&layer, &cache, 0, 0),
            Some(Rgb::new(1, 2, 3))
        );
        assert_eq!(
            sample_exact_lookup_rgb(&layer, &cache, 1, 1),
            Some(Rgb::new(10, 11, 12))
        );
        assert_eq!(sample_exact_lookup_rgb(&layer, &cache, 2, 0), None);
    }

    #[test]
    fn render_exact_lookup_tile_image_uses_exact_lookup_pixels() {
        let layer = test_layer();
        let mask = ZoneMask::from_rgb(
            2,
            2,
            vec![
                1, 2, 3, 4, 5, 6, //
                7, 8, 9, 10, 11, 12,
            ],
        )
        .expect("mask");
        let lookup = mask.to_lookup_rows().expect("lookup");
        let url = layer.field_url().expect("lookup url");
        let mut cache = ExactLookupCache::default();
        cache.entries.insert(
            layer.id,
            ExactLookupEntry {
                url,
                state: ExactLookupState::Ready(lookup),
            },
        );

        let image = render_exact_lookup_tile_image(
            &layer,
            &cache,
            TileKey {
                layer: layer.id,
                map_version: 0,
                z: 0,
                tx: 0,
                ty: 0,
            },
        )
        .expect("image");

        assert_eq!(image.texture_descriptor.size.width, 2);
        assert_eq!(image.texture_descriptor.size.height, 2);
        let data = image.data.as_ref().expect("image data");
        assert_eq!(
            data,
            &[
                1, 2, 3, 255, 4, 5, 6, 255, //
                7, 8, 9, 255, 10, 11, 12, 255,
            ]
        );
    }
}
