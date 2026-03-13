use axum::extract::{rejection::QueryRejection, Extension, Query, State};
use axum::Json;
use serde::Deserialize;

use fishystuff_api::models::zones::ZonesResponse;

use crate::error::{with_timeout, AppResult};
use crate::routes::meta::map_request_id;
use crate::state::{RequestId, SharedState};

#[derive(Debug, Deserialize)]
pub struct ZonesQuery {
    pub r#ref: Option<String>,
}

pub async fn list_zones(
    State(state): State<SharedState>,
    query: Result<Query<ZonesQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<Json<ZonesResponse>> {
    let Query(query) = query.map_err(|err| {
        crate::error::AppError::invalid_argument(err.to_string())
            .with_request_id(request_id.0.clone())
    })?;

    let zones = with_timeout(
        state.config.request_timeout_secs,
        state.store.list_zones(query.r#ref),
    )
    .await
    .map_err(|err| map_request_id(err, &request_id))?;

    Ok(Json(ZonesResponse { zones }))
}
