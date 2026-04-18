use std::path::PathBuf;
use std::sync::OnceLock;

use async_channel::Receiver;
use bevy::tasks::IoTaskPool;
use serde::de::DeserializeOwned;
use serde::Serialize;

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
    serde_json::from_slice(&bytes).map_err(|err| {
        if let Some(body_preview) = preview_body_bytes(&bytes) {
            format!("parse {}: {} (body: {})", path, err, body_preview)
        } else {
            format!("parse {}: {}", path, err)
        }
    })
}

pub async fn post_json_async<Req, Resp>(path: &str, payload: &Req) -> Result<Resp, String>
where
    Req: Serialize,
    Resp: DeserializeOwned,
{
    #[cfg(target_arch = "wasm32")]
    {
        let response = gloo_net::http::Request::post(path)
            .json(payload)
            .map_err(|err| format!("encode {}: {}", path, err))?
            .send()
            .await
            .map_err(|err| format!("fetch {}: {}", path, err))?;
        if !response.ok() {
            return Err(http_error_message("POST", path, response).await);
        }
        return response
            .json::<Resp>()
            .await
            .map_err(|err| format!("parse {}: {}", path, err));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = path;
        let _ = payload;
        Err("post_json_async is only supported on wasm32".to_string())
    }
}

pub async fn load_bytes_async(path: &str) -> Result<Vec<u8>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let response = gloo_net::http::Request::get(path)
            .send()
            .await
            .map_err(|err| format!("fetch {}: {}", path, err))?;
        if !response.ok() {
            return Err(http_error_message("GET", path, response).await);
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

const ERROR_BODY_PREVIEW_LIMIT: usize = 240;

fn preview_body_bytes(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;
    let preview = summarize_body(text);
    if preview.is_empty() {
        None
    } else {
        Some(preview)
    }
}

fn summarize_body(body: &str) -> String {
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.len() <= ERROR_BODY_PREVIEW_LIMIT {
        return compact;
    }

    let mut end = ERROR_BODY_PREVIEW_LIMIT;
    while !compact.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &compact[..end])
}

#[cfg(target_arch = "wasm32")]
async fn http_error_message(
    method: &str,
    path: &str,
    response: gloo_net::http::Response,
) -> String {
    let status = response.status();
    let request_id = response.headers().get("x-request-id").unwrap_or_default();
    let trace_id = response.headers().get("x-trace-id").unwrap_or_default();
    let span_id = response.headers().get("x-span-id").unwrap_or_default();
    let body = summarize_body(&response.text().await.unwrap_or_default());

    let mut context = Vec::new();
    if !request_id.is_empty() {
        context.push(format!("request_id={request_id}"));
    }
    if !trace_id.is_empty() {
        context.push(format!("trace_id={trace_id}"));
    }
    if !span_id.is_empty() {
        context.push(format!("span_id={span_id}"));
    }

    let context = if context.is_empty() {
        String::new()
    } else {
        format!(" [{}]", context.join(" "))
    };
    if body.is_empty() {
        format!("{method} {path}: http {status}{context}")
    } else {
        format!("{method} {path}: http {status}{context}: {body}")
    }
}
