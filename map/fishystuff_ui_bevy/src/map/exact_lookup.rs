use std::collections::HashMap;

use async_channel::Receiver;
use bevy::prelude::Resource;
use fishystuff_api::Rgb;
use fishystuff_core::masks::ZoneLookupRows;

use crate::map::layers::{LayerId, LayerSpec};
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
    let Some(url) = layer.exact_lookup_url() else {
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
    let url = layer.exact_lookup_url()?;
    let lookup = lookups.get(layer.id, &url)?;
    let rgb = lookup.rgb_u32(map_px_x, map_px_y)?;
    let [r, g, b] = [
        ((rgb >> 16) & 0xff) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
    ];
    Some(Rgb::new(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::layers::{LayerKind, LayerSpec, LodPolicy, PickMode};
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
        let lookup = ZoneLookupRows::from_zone_mask(&mask).expect("lookup");
        let url = layer.exact_lookup_url().expect("lookup url");
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
}
