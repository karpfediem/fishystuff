use std::collections::HashMap;

use async_channel::Receiver;
use bevy::prelude::Resource;
use fishystuff_core::field_metadata::{FieldHoverMetadataAsset, FieldHoverMetadataEntry};

use crate::map::layers::{LayerId, LayerSpec};
use crate::runtime_io;

#[derive(Debug, Clone)]
enum FieldMetadataState {
    Loading,
    Ready(FieldHoverMetadataAsset),
    Failed,
}

#[derive(Debug, Clone)]
struct FieldMetadataEntryState {
    url: String,
    state: FieldMetadataState,
}

struct PendingFieldMetadataRequest {
    url: String,
    receiver: Receiver<Result<FieldHoverMetadataAsset, String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldMetadataStatus {
    Missing,
    Loading,
    Ready,
    Failed,
}

#[derive(Resource, Default)]
pub struct FieldMetadataCache {
    entries: HashMap<LayerId, FieldMetadataEntryState>,
}

#[derive(Resource, Default)]
pub struct PendingFieldMetadata {
    receivers: HashMap<LayerId, PendingFieldMetadataRequest>,
}

impl FieldMetadataCache {
    pub fn insert_ready(&mut self, layer: LayerId, url: String, metadata: FieldHoverMetadataAsset) {
        self.entries.insert(
            layer,
            FieldMetadataEntryState {
                url,
                state: FieldMetadataState::Ready(metadata),
            },
        );
    }

    pub fn remove_layer(&mut self, layer: LayerId) {
        self.entries.remove(&layer);
    }

    pub fn get(&self, layer: LayerId, url: &str) -> Option<&FieldHoverMetadataAsset> {
        let entry = self.entries.get(&layer)?;
        if entry.url != url {
            return None;
        }
        match &entry.state {
            FieldMetadataState::Ready(metadata) => Some(metadata),
            FieldMetadataState::Loading | FieldMetadataState::Failed => None,
        }
    }

    pub fn entry(
        &self,
        layer: LayerId,
        url: &str,
        field_id: u32,
    ) -> Option<&FieldHoverMetadataEntry> {
        self.get(layer, url)?.entry(field_id)
    }

    pub fn status(&self, layer: LayerId, url: &str) -> FieldMetadataStatus {
        let Some(entry) = self.entries.get(&layer) else {
            return FieldMetadataStatus::Missing;
        };
        if entry.url != url {
            return FieldMetadataStatus::Missing;
        }
        match entry.state {
            FieldMetadataState::Loading => FieldMetadataStatus::Loading,
            FieldMetadataState::Ready(_) => FieldMetadataStatus::Ready,
            FieldMetadataState::Failed => FieldMetadataStatus::Failed,
        }
    }

    pub fn layer_ids(&self) -> Vec<LayerId> {
        self.entries.keys().copied().collect()
    }
}

impl PendingFieldMetadata {
    pub fn remove_layer(&mut self, layer: LayerId) {
        self.receivers.remove(&layer);
    }

    pub fn layer_ids(&self) -> Vec<LayerId> {
        self.receivers.keys().copied().collect()
    }
}

pub fn ensure_field_metadata_request(
    layer: &LayerSpec,
    metadata: &mut FieldMetadataCache,
    pending: &mut PendingFieldMetadata,
) {
    let Some(url) = layer.field_metadata_url() else {
        metadata.remove_layer(layer.id);
        pending.remove_layer(layer.id);
        return;
    };

    if let Some(request) = pending.receivers.get(&layer.id) {
        if request.url == url {
            metadata.entries.insert(
                layer.id,
                FieldMetadataEntryState {
                    url,
                    state: FieldMetadataState::Loading,
                },
            );
            return;
        }
        pending.receivers.remove(&layer.id);
    }

    if let Some(entry) = metadata.entries.get(&layer.id) {
        if entry.url == url {
            return;
        }
    }

    let receiver = runtime_io::spawn_json_request(url.clone());
    pending.receivers.insert(
        layer.id,
        PendingFieldMetadataRequest {
            url: url.clone(),
            receiver,
        },
    );
    metadata.entries.insert(
        layer.id,
        FieldMetadataEntryState {
            url,
            state: FieldMetadataState::Loading,
        },
    );
}

pub fn poll_field_metadata_requests(
    metadata: &mut FieldMetadataCache,
    pending: &mut PendingFieldMetadata,
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
        match result {
            Ok(asset) => {
                metadata.entries.insert(
                    layer_id,
                    FieldMetadataEntryState {
                        url: request.url,
                        state: FieldMetadataState::Ready(asset),
                    },
                );
            }
            Err(err) => {
                bevy::log::warn!("layer {:?} field metadata load failed: {}", layer_id, err);
                metadata.entries.insert(
                    layer_id,
                    FieldMetadataEntryState {
                        url: request.url,
                        state: FieldMetadataState::Failed,
                    },
                );
            }
        }
    }
}
