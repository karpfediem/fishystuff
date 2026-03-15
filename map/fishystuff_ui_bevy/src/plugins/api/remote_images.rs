use std::collections::HashMap;

use crate::runtime_io;
use async_channel::Receiver;
use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::IoTaskPool;

#[derive(Resource, Default)]
pub struct RemoteImageCache {
    handles: HashMap<String, Handle<Image>>,
    pending: HashMap<String, Receiver<Result<DecodedRemoteImage, String>>>,
    failed: HashMap<String, String>,
}

#[derive(Resource, Default)]
pub struct RemoteImageEpoch(pub u64);

pub enum RemoteImageStatus {
    Ready(Handle<Image>),
    Pending,
    Failed(String),
}

#[derive(Debug)]
struct DecodedRemoteImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

pub fn remote_image_handle(url: &str, cache: &mut RemoteImageCache) -> RemoteImageStatus {
    let normalized = url.trim();
    if normalized.is_empty() {
        return RemoteImageStatus::Failed("empty url".to_string());
    }

    if let Some(handle) = cache.handles.get(normalized) {
        return RemoteImageStatus::Ready(handle.clone());
    }
    if let Some(error) = cache.failed.get(normalized) {
        return RemoteImageStatus::Failed(error.clone());
    }
    if !cache.pending.contains_key(normalized) {
        let (sender, receiver) = async_channel::bounded(1);
        let request_url = normalized.to_string();
        IoTaskPool::get()
            .spawn_local(async move {
                let result = fetch_remote_image(&request_url).await;
                let _ = sender.send(result).await;
            })
            .detach();
        cache.pending.insert(normalized.to_string(), receiver);
    }
    RemoteImageStatus::Pending
}

pub fn poll_remote_image_requests(
    mut cache: ResMut<RemoteImageCache>,
    mut epoch: ResMut<RemoteImageEpoch>,
    mut images: ResMut<Assets<Image>>,
) {
    if cache.pending.is_empty() {
        return;
    }

    let pending = cache
        .pending
        .iter()
        .map(|(url, receiver)| (url.clone(), receiver.clone()))
        .collect::<Vec<_>>();

    let mut changed = false;
    for (url, receiver) in pending {
        let Ok(result) = receiver.try_recv() else {
            continue;
        };
        cache.pending.remove(&url);
        match result {
            Ok(decoded) => {
                cache.failed.remove(&url);
                let image = Image::new(
                    Extent3d {
                        width: decoded.width,
                        height: decoded.height,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    decoded.rgba,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::RENDER_WORLD,
                );
                cache.handles.insert(url, images.add(image));
            }
            Err(error) => {
                cache.failed.insert(url, error);
            }
        }
        changed = true;
    }

    if changed {
        epoch.0 = epoch.0.wrapping_add(1);
    }
}

async fn fetch_remote_image(url: &str) -> Result<DecodedRemoteImage, String> {
    let bytes = runtime_io::load_bytes_async(url).await?;
    let image = image::load_from_memory(bytes.as_slice())
        .map_err(|err| format!("decode {url}: {err}"))?
        .to_rgba8();
    Ok(DecodedRemoteImage {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}
