use axum::extract::{rejection::QueryRejection, Extension, Query, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use fishystuff_api::models::events::{EventsSnapshotMetaResponse, EventsSnapshotResponse};

use crate::error::{with_timeout, AppError, AppResult};
use crate::routes::meta::map_request_id;
use crate::state::{RequestId, SharedState};

#[derive(Debug, Deserialize, Default)]
pub struct EventsSnapshotQuery {
    pub revision: Option<String>,
}

pub async fn events_snapshot_meta(
    State(state): State<SharedState>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<impl IntoResponse> {
    let response = with_timeout(
        state.config.request_timeout_secs,
        state.store.events_snapshot_meta(),
    )
    .await
    .map_err(|err| map_request_id(err, &request_id))?;
    Ok((meta_headers(&response), Json(response)))
}

pub async fn events_snapshot(
    State(state): State<SharedState>,
    query: Result<Query<EventsSnapshotQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<impl IntoResponse> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let response = with_timeout(
        state.config.request_timeout_secs,
        state.store.events_snapshot(query.revision),
    )
    .await
    .map_err(|err| map_request_id(err, &request_id))?;

    Ok((snapshot_headers(&response), Json(response)))
}

fn meta_headers(response: &EventsSnapshotMetaResponse) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, max-age=0, must-revalidate"),
    );
    headers.insert(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    insert_revision_headers(&mut headers, response.revision.as_str());
    headers
}

fn snapshot_headers(response: &EventsSnapshotResponse) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    headers.insert(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    insert_revision_headers(&mut headers, response.revision.as_str());
    headers
}

fn insert_revision_headers(headers: &mut HeaderMap, revision: &str) {
    if let Ok(value) = HeaderValue::from_str(revision) {
        headers.insert(header::HeaderName::from_static("x-events-revision"), value);
    }
    let etag = format!("\"{revision}\"");
    if let Ok(value) = HeaderValue::from_str(etag.as_str()) {
        headers.insert(header::ETAG, value);
    }
}
