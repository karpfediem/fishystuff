use super::*;

fn synthetic_loaded_manifest(config: &Terrain3dConfig) -> LoadedTerrainManifest {
    LoadedTerrainManifest {
        manifest: TerrainManifest {
            revision: "runtime-height-tiles".to_string(),
            map_width: config.map_width,
            map_height: config.map_height,
            chunk_map_px: config.chunk_map_px,
            grid_size: config.verts_per_chunk_edge as u16,
            max_level: 0,
            bbox_y_min: config.bbox_y_min,
            bbox_y_max: config.bbox_y_max,
            encoding: TerrainHeightEncoding::U16Norm,
            root: String::new(),
            chunk_path: String::new(),
            levels: Vec::new(),
        },
        levels: HashMap::new(),
    }
}

pub(super) fn invalidate_on_config_change(
    config: Res<Terrain3dConfig>,
    mut runtime: ResMut<TerrainRuntime>,
    mut commands: Commands,
) {
    if !config.is_changed() {
        return;
    }
    let manifest_changed = runtime
        .active_manifest_url
        .as_ref()
        .map(|url| url != &config.terrain_manifest_url)
        .unwrap_or(true);
    let drape_changed = runtime
        .active_drape_manifest_url
        .as_ref()
        .map(|url| url != &config.drape_manifest_url)
        .unwrap_or(config.use_chunk_aligned_drape);
    if manifest_changed || drape_changed {
        runtime.clear_all_entities(&mut commands);
        runtime.manifest = None;
        runtime.pending_manifest = None;
        runtime.manifest_failed = false;
        runtime.active_manifest_url = None;
        runtime.pending_chunks.clear();
    }
}

pub(super) fn ensure_terrain_manifest_request(
    mut mode: ResMut<ViewModeState>,
    config: Res<Terrain3dConfig>,
    mut runtime: ResMut<TerrainRuntime>,
) {
    if mode.mode != ViewMode::Terrain3D {
        return;
    }
    if !mode.terrain_initialized {
        mode.terrain_initialized = true;
    }
    if config.terrain_manifest_url.trim().is_empty() {
        runtime.pending_manifest = None;
        runtime.manifest = Some(synthetic_loaded_manifest(&config));
        runtime.manifest_failed = false;
        runtime.active_manifest_url = None;
        runtime.pending_chunks.clear();
        runtime.pending_drape_manifest = None;
        runtime.drape_manifest = None;
        runtime.active_drape_manifest_url = None;
        runtime.chunk_drape_entries.clear();
        runtime.pending_chunk_drape_textures.clear();
        runtime.queued_chunk_drapes.clear();
        runtime.chunk_drape_queue.clear();
        return;
    }
    let should_restart = runtime
        .active_manifest_url
        .as_ref()
        .map(|url| url != &config.terrain_manifest_url)
        .unwrap_or(true);
    if should_restart {
        runtime.pending_manifest = Some(PendingTerrainManifestRequest {
            url: config.terrain_manifest_url.clone(),
            receiver: spawn_json_request::<TerrainManifest>(config.terrain_manifest_url.clone()),
        });
        runtime.manifest = None;
        runtime.manifest_failed = false;
        runtime.active_manifest_url = Some(config.terrain_manifest_url.clone());
        runtime.pending_chunks.clear();
    }
    if config.use_chunk_aligned_drape {
        if config.drape_manifest_url.trim().is_empty() {
            runtime.pending_drape_manifest = None;
            runtime.drape_manifest = None;
            runtime.active_drape_manifest_url = None;
            runtime.chunk_drape_entries.clear();
            runtime.pending_chunk_drape_textures.clear();
            runtime.queued_chunk_drapes.clear();
            runtime.chunk_drape_queue.clear();
            return;
        }
        let drape_restart = runtime
            .active_drape_manifest_url
            .as_ref()
            .map(|url| url != &config.drape_manifest_url)
            .unwrap_or(true);
        if drape_restart {
            runtime.pending_drape_manifest = Some(PendingTerrainDrapeManifestRequest {
                url: config.drape_manifest_url.clone(),
                receiver: spawn_json_request::<TerrainDrapeManifest>(
                    config.drape_manifest_url.clone(),
                ),
            });
            runtime.drape_manifest = None;
            runtime.active_drape_manifest_url = Some(config.drape_manifest_url.clone());
            runtime.chunk_drape_entries.clear();
            runtime.pending_chunk_drape_textures.clear();
            runtime.queued_chunk_drapes.clear();
            runtime.chunk_drape_queue.clear();
        }
    } else {
        runtime.pending_drape_manifest = None;
        runtime.drape_manifest = None;
        runtime.active_drape_manifest_url = None;
        runtime.chunk_drape_entries.clear();
        runtime.pending_chunk_drape_textures.clear();
        runtime.queued_chunk_drapes.clear();
        runtime.chunk_drape_queue.clear();
    }
}

pub(super) fn poll_terrain_manifest_ready(
    mode: Res<ViewModeState>,
    _config: Res<Terrain3dConfig>,
    mut runtime: ResMut<TerrainRuntime>,
) {
    if mode.mode != ViewMode::Terrain3D {
        return;
    }
    if let Some(pending) = runtime.pending_manifest.as_ref() {
        if let Ok(result) = pending.receiver.try_recv() {
            let Some(pending) = runtime.pending_manifest.take() else {
                return;
            };
            match result {
                Ok(manifest) => match decode_loaded_terrain_manifest(manifest, &pending.url) {
                    Ok(loaded) => {
                        bevy::log::info!(
                            "terrain manifest ready: rev={} levels={} chunk_map_px={} grid={} max_level={}",
                            loaded.manifest.revision,
                            loaded.manifest.levels.len(),
                            loaded.manifest.chunk_map_px,
                            loaded.manifest.grid_size,
                            loaded.manifest.max_level
                        );
                        runtime.manifest = Some(loaded);
                        runtime.manifest_failed = false;
                        runtime.active_manifest_url = Some(pending.url);
                    }
                    Err(err) => {
                        runtime.manifest = None;
                        runtime.manifest_failed = true;
                        bevy::log::warn!("terrain manifest decode failed: {}", err);
                    }
                },
                Err(err) => {
                    runtime.manifest = None;
                    runtime.manifest_failed = true;
                    bevy::log::warn!("terrain manifest load failed: {}", err);
                }
            }
        }
    }
    if let Some(pending) = runtime.pending_drape_manifest.as_ref() {
        if let Ok(result) = pending.receiver.try_recv() {
            let Some(pending) = runtime.pending_drape_manifest.take() else {
                return;
            };
            match result {
                Ok(manifest) => {
                    match decode_loaded_terrain_drape_manifest(manifest, &pending.url) {
                        Ok(loaded) => {
                            runtime.drape_manifest = Some(loaded);
                            runtime.active_drape_manifest_url = Some(pending.url);
                        }
                        Err(err) => {
                            runtime.drape_manifest = None;
                            bevy::log::warn!("terrain drape manifest decode failed: {}", err);
                        }
                    }
                }
                Err(err) => {
                    runtime.drape_manifest = None;
                    bevy::log::warn!("terrain drape manifest load failed: {}", err);
                }
            }
        }
    }

    let mut finished = Vec::new();
    for (key, pending) in runtime.pending_chunks.iter() {
        if let Ok(result) = pending.receiver.try_recv() {
            finished.push((*key, result));
        }
    }
    for (key, result) in finished {
        runtime.pending_chunks.remove(&key);
        let frame = runtime.frame;
        let entry = runtime.chunks.entry(key).or_default();
        entry.last_touched = frame;
        match result {
            Ok(bytes) => match decode_terrain_chunk(&bytes) {
                Ok(chunk) => {
                    entry.chunk = Some(chunk);
                    entry.state = TerrainChunkState::Building;
                    runtime.cache_hits = runtime.cache_hits.saturating_add(1);
                }
                Err(err) => {
                    entry.chunk = None;
                    entry.state = TerrainChunkState::Failed;
                    runtime.cache_misses = runtime.cache_misses.saturating_add(1);
                    bevy::log::warn!(
                        "terrain chunk decode failed level={} x={} y={}: {}",
                        key.level(),
                        key.cx(),
                        key.cy(),
                        err
                    );
                }
            },
            Err(err) => {
                entry.chunk = None;
                entry.state = TerrainChunkState::Failed;
                runtime.cache_misses = runtime.cache_misses.saturating_add(1);
                bevy::log::warn!(
                    "terrain chunk load failed level={} x={} y={}: {}",
                    key.level(),
                    key.cx(),
                    key.cy(),
                    err
                );
            }
        }
    }
}

fn decode_loaded_terrain_manifest(
    mut manifest: TerrainManifest,
    manifest_url: &str,
) -> Result<LoadedTerrainManifest, String> {
    manifest.root = rebase_manifest_root(manifest_url, &manifest.root);
    let mut levels = HashMap::new();
    for level in &manifest.levels {
        let decoded = level
            .decode()
            .map_err(|err| format!("decode terrain level {}: {err}", level.level))?;
        levels.insert(level.level, decoded);
    }
    Ok(LoadedTerrainManifest { manifest, levels })
}

fn decode_loaded_terrain_drape_manifest(
    mut manifest: TerrainDrapeManifest,
    manifest_url: &str,
) -> Result<LoadedTerrainDrapeManifest, String> {
    manifest.root = rebase_manifest_root(manifest_url, &manifest.root);
    let mut levels = HashMap::new();
    for level in &manifest.levels {
        let decoded = level
            .decode()
            .map_err(|err| format!("decode terrain drape level {}: {err}", level.level))?;
        levels.insert(level.level, decoded);
    }
    Ok(LoadedTerrainDrapeManifest { manifest, levels })
}

fn rebase_manifest_root(manifest_url: &str, root: &str) -> String {
    let normalized = root.trim();
    if normalized.is_empty()
        || normalized.starts_with("http://")
        || normalized.starts_with("https://")
        || !normalized.starts_with('/')
    {
        return normalized.to_string();
    }
    let Some(origin) = absolute_url_origin(manifest_url) else {
        return normalized.to_string();
    };
    format!("{origin}{normalized}")
}

fn absolute_url_origin(url: &str) -> Option<&str> {
    let scheme = url.find("://")?;
    let authority_start = scheme + 3;
    let authority_end = url[authority_start..]
        .find('/')
        .map(|offset| authority_start + offset)
        .unwrap_or(url.len());
    if authority_end <= authority_start {
        return None;
    }
    Some(&url[..authority_end])
}

pub(super) fn spawn_json_request<T>(url: String) -> Receiver<Result<T, String>>
where
    for<'de> T: Deserialize<'de> + Send + 'static,
{
    let (sender, receiver) = async_channel::bounded(1);
    IoTaskPool::get()
        .spawn_local(async move {
            let result = fetch_json::<T>(&url).await;
            let _ = sender.send(result).await;
        })
        .detach();
    receiver
}

pub(super) fn spawn_bytes_request(url: String) -> Receiver<Result<Vec<u8>, String>> {
    let (sender, receiver) = async_channel::bounded(1);
    IoTaskPool::get()
        .spawn_local(async move {
            let result = fetch_bytes(&url).await;
            let _ = sender.send(result).await;
        })
        .detach();
    receiver
}

async fn fetch_json<T>(url: &str) -> Result<T, String>
where
    for<'de> T: Deserialize<'de> + Send + 'static,
{
    let response = Request::get(url)
        .send()
        .await
        .map_err(|err| err.to_string())?;
    if !response.ok() {
        return Err(format!("{}: {}", url, response.status()));
    }
    response.json::<T>().await.map_err(|err| err.to_string())
}

async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    let response = Request::get(url)
        .send()
        .await
        .map_err(|err| err.to_string())?;
    if !response.ok() {
        return Err(format!("{}: {}", url, response.status()));
    }
    response.binary().await.map_err(|err| err.to_string())
}
