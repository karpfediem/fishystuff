use std::path::PathBuf;
use std::sync::OnceLock;

use async_channel::Receiver;
use bevy::tasks::IoTaskPool;
use serde::de::DeserializeOwned;

static BASE_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn set_base_dir(path: impl Into<PathBuf>) {
    let _ = BASE_DIR.set(path.into());
}

pub fn spawn_json_request<T>(path: String) -> Receiver<Result<T, String>>
where
    T: DeserializeOwned + Send + 'static,
{
    let (sender, receiver) = async_channel::bounded(1);
    #[cfg(target_arch = "wasm32")]
    IoTaskPool::get()
        .spawn_local(async move {
            let result = load_json_async::<T>(&path).await;
            let _ = sender.send(result).await;
        })
        .detach();

    #[cfg(not(target_arch = "wasm32"))]
    IoTaskPool::get()
        .spawn(async move {
            let result = load_json::<T>(&path);
            let _ = sender.send(result).await;
        })
        .detach();
    receiver
}

pub fn spawn_bytes_request(path: String) -> Receiver<Result<Vec<u8>, String>> {
    let (sender, receiver) = async_channel::bounded(1);
    #[cfg(target_arch = "wasm32")]
    IoTaskPool::get()
        .spawn_local(async move {
            let result = load_bytes_async(&path).await;
            let _ = sender.send(result).await;
        })
        .detach();

    #[cfg(not(target_arch = "wasm32"))]
    IoTaskPool::get()
        .spawn(async move {
            let result = load_bytes(&path);
            let _ = sender.send(result).await;
        })
        .detach();
    receiver
}

pub async fn load_json_async<T>(path: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let bytes = load_bytes_async(path).await?;
    serde_json::from_slice(&bytes).map_err(|err| format!("parse {}: {}", path, err))
}

pub async fn load_bytes_async(path: &str) -> Result<Vec<u8>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let response = gloo_net::http::Request::get(path)
            .send()
            .await
            .map_err(|err| format!("fetch {}: {}", path, err))?;
        if !response.ok() {
            return Err(format!("fetch {}: {}", path, response.status()));
        }
        response
            .binary()
            .await
            .map_err(|err| format!("read {}: {}", path, err))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        load_bytes(path)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_json<T>(path: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let bytes = load_bytes(path)?;
    serde_json::from_slice(&bytes).map_err(|err| format!("parse {}: {}", path, err))
}

#[cfg(target_arch = "wasm32")]
pub fn load_json<T>(_path: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    Err("synchronous JSON loads are not supported on wasm; use load_json_async".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_bytes(path: &str) -> Result<Vec<u8>, String> {
    let resolved = std::path::Path::new(path);
    let resolved = if resolved.is_absolute() {
        resolved.to_path_buf()
    } else {
        BASE_DIR
            .get()
            .cloned()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ".".into()))
            .join(resolved)
    };
    std::fs::read(&resolved).map_err(|err| format!("read {}: {}", resolved.display(), err))
}

#[cfg(target_arch = "wasm32")]
pub fn load_bytes(_path: &str) -> Result<Vec<u8>, String> {
    Err("synchronous byte loads are not supported on wasm; use load_bytes_async".to_string())
}
