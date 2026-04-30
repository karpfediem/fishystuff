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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnapshotCachePolicy {
    Latest,
    Revisioned,
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
    let requested_revision = query
        .revision
        .and_then(|revision| non_empty_query_value(revision.as_str()));
    let cache_policy = if requested_revision.is_some() {
        SnapshotCachePolicy::Revisioned
    } else {
        SnapshotCachePolicy::Latest
    };

    let response = with_timeout(
        state.config.request_timeout_secs,
        state.store.events_snapshot(requested_revision),
    )
    .await
    .map_err(|err| map_request_id(err, &request_id))?;

    Ok((snapshot_headers(&response, cache_policy), Json(response)))
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

fn snapshot_headers(
    response: &EventsSnapshotResponse,
    cache_policy: SnapshotCachePolicy,
) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let cache_control = match cache_policy {
        SnapshotCachePolicy::Latest => HeaderValue::from_static("no-store"),
        SnapshotCachePolicy::Revisioned => {
            HeaderValue::from_static("public, max-age=31536000, immutable")
        }
    };
    headers.insert(header::CACHE_CONTROL, cache_control);
    headers.insert(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    insert_revision_headers(&mut headers, response.revision.as_str());
    headers
}

fn non_empty_query_value(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revisioned_snapshot_headers_are_immutable() {
        let response = EventsSnapshotResponse {
            revision: "events-rev-1".to_owned(),
            ..EventsSnapshotResponse::default()
        };

        let headers = snapshot_headers(&response, SnapshotCachePolicy::Revisioned);

        assert_eq!(
            headers.get(header::CACHE_CONTROL).unwrap(),
            "public, max-age=31536000, immutable"
        );
        assert_eq!(headers.get(header::ETAG).unwrap(), "\"events-rev-1\"");
        assert_eq!(
            headers
                .get(header::HeaderName::from_static("x-events-revision"))
                .unwrap(),
            "events-rev-1"
        );
    }

    #[test]
    fn latest_snapshot_headers_are_not_immutable() {
        let response = EventsSnapshotResponse {
            revision: "events-rev-1".to_owned(),
            ..EventsSnapshotResponse::default()
        };

        let headers = snapshot_headers(&response, SnapshotCachePolicy::Latest);

        assert_eq!(headers.get(header::CACHE_CONTROL).unwrap(), "no-store");
        assert_eq!(headers.get(header::ETAG).unwrap(), "\"events-rev-1\"");
    }

    #[test]
    fn blank_revision_query_values_are_treated_as_latest() {
        assert_eq!(non_empty_query_value(""), None);
        assert_eq!(non_empty_query_value("   "), None);
        assert_eq!(non_empty_query_value(" rev-1 "), Some("rev-1".to_owned()));
    }
}
