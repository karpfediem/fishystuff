use axum::extract::{Extension, State};
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

use fishystuff_api::models::meta::MetaResponse;

use crate::error::{with_timeout, AppError, AppResult};
use crate::state::{RequestId, SharedState};

pub async fn healthz(Extension(_request_id): Extension<RequestId>) -> AppResult<impl IntoResponse> {
    Ok(Json(json!({ "status": "ok" })))
}

pub async fn readyz(
    State(state): State<SharedState>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<impl IntoResponse> {
    with_timeout(state.config.request_timeout_secs, state.store.healthcheck())
        .await
        .map_err(|err| err.with_request_id(request_id.0))?;
    Ok(Json(json!({ "status": "ok" })))
}

pub async fn get_meta(
    State(state): State<SharedState>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<Json<MetaResponse>> {
    let mut meta = with_timeout(state.config.request_timeout_secs, state.store.get_meta())
        .await
        .map_err(|err| err.with_request_id(request_id.0))?;
    if let Some(url) = state.config.terrain_manifest_url.as_ref() {
        meta.terrain_manifest_url = Some(url.clone());
    }
    if let Some(url) = state.config.terrain_drape_manifest_url.as_ref() {
        meta.terrain_drape_manifest_url = Some(url.clone());
    }
    if let Some(url) = state.config.terrain_height_tiles_url.as_ref() {
        meta.terrain_height_tiles_url = Some(url.clone());
    }
    Ok(Json(meta))
}

pub async fn openapi_json() -> Json<serde_json::Value> {
    Json(json!({
      "openapi": "3.1.0",
      "info": {
        "title": "fishystuff API",
        "version": "v1"
      },
      "paths": {
        "/api/v1/meta": { "get": { "summary": "Get metadata" } },
        "/api/v1/layers": { "get": { "summary": "Get layer descriptors" } },
        "/api/v1/region_groups": { "get": { "summary": "Get region-group metadata" } },
        "/api/v1/zones": { "get": { "summary": "List zones" } },
        "/api/v1/fish": { "get": { "summary": "List fish metadata" } },
        "/api/v1/zone_profile_v2": { "post": { "summary": "Structured zone profile with separated ranking evidence and placeholders for border analysis and catch rates" } },
        "/api/v1/zone_stats": { "post": { "summary": "Zone evidence distribution" } },
        "/api/v1/effort_grid": { "post": { "summary": "Effort grid" } },
        "/api/v1/events_snapshot_meta": { "get": { "summary": "Ranking events snapshot metadata" } },
        "/api/v1/events_snapshot": { "get": { "summary": "Revisioned ranking events snapshot" } }
      }
    }))
}

pub fn map_request_id(err: AppError, request_id: &RequestId) -> AppError {
    err.with_request_id(request_id.0.clone())
}
