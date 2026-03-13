use axum::extract::{rejection::QueryRejection, Extension, Query, State};
use axum::Json;
use serde::Deserialize;

use fishystuff_api::ids::MapVersionId;
use fishystuff_api::models::region_groups::RegionGroupsResponse;

use crate::error::{with_timeout, AppResult};
use crate::routes::meta::map_request_id;
use crate::state::{RequestId, SharedState};

#[derive(Debug, Deserialize)]
pub struct RegionGroupsQuery {
    pub map_version: Option<String>,
}

pub async fn get_region_groups(
    State(state): State<SharedState>,
    query: Result<Query<RegionGroupsQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<Json<RegionGroupsResponse>> {
    let Query(query) = query.map_err(|err| {
        crate::error::AppError::invalid_argument(err.to_string())
            .with_request_id(request_id.0.clone())
    })?;

    let map_version = query
        .map_version
        .and_then(|raw| {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .or_else(|| {
            state
                .config
                .defaults
                .map_version_id
                .as_ref()
                .map(|id| id.0.clone())
        });

    let mut response = with_timeout(
        state.config.request_timeout_secs,
        state.store.get_region_groups(map_version.clone()),
    )
    .await
    .map_err(|err| map_request_id(err, &request_id))?;

    if response.map_version_id.is_none() {
        response.map_version_id = map_version.map(MapVersionId);
    }

    Ok(Json(response))
}
