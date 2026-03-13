use axum::Json;
use axum::extract::{Extension, State, rejection::JsonRejection};
use axum::http::HeaderMap;

use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};

use crate::error::{AppError, AppResult, with_timeout};
use crate::routes::meta::map_request_id;
use crate::routes::public_assets::absolutize_zone_stats_icons;
use crate::state::{RequestId, SharedState};

pub async fn zone_stats(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Extension(request_id): Extension<RequestId>,
    payload: Result<Json<ZoneStatsRequest>, JsonRejection>,
) -> AppResult<Json<ZoneStatsResponse>> {
    let Json(request) = payload.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let cache_key = serde_json::to_string(&request)
        .map_err(|err| AppError::internal(err.to_string()).with_request_id(request_id.0.clone()))?;
    if let Ok(mut cache) = state.cache.zone_stats.lock() {
        if let Some(cached) = cache.get(&cache_key) {
            let mut parsed: ZoneStatsResponse = serde_json::from_str(&cached).map_err(|err| {
                AppError::internal(format!("zone_stats cache decode failed: {err}"))
                    .with_request_id(request_id.0.clone())
            })?;
            absolutize_zone_stats_icons(
                &headers,
                &mut parsed,
                state.config.images_public_base_url.as_deref(),
            );
            return Ok(Json(parsed));
        }
    }

    let raw_response = with_timeout(
        state.config.request_timeout_secs,
        state
            .store
            .zone_stats(request, state.config.status_cfg.clone()),
    )
    .await
    .map_err(|err| map_request_id(err, &request_id))?;

    if let Ok(encoded) = serde_json::to_string(&raw_response) {
        if let Ok(mut cache) = state.cache.zone_stats.lock() {
            cache.insert(cache_key, encoded);
        }
    }

    let mut response = raw_response;
    absolutize_zone_stats_icons(
        &headers,
        &mut response,
        state.config.images_public_base_url.as_deref(),
    );
    Ok(Json(response))
}
