use axum::extract::{rejection::QueryRejection, Extension, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use fishystuff_api::models::fish::{FishListResponse, FishMapResponse, FishTableResponse};

use crate::error::{with_timeout, AppError, AppResult};
use crate::routes::meta::map_request_id;
use crate::state::{RequestId, SharedState};
use crate::store::FishLang;

#[derive(Debug, Deserialize)]
pub struct FishQuery {
    pub lang: Option<String>,
    pub r#ref: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FishMapQuery {
    pub encyclopedia_key: Option<i32>,
    pub item_key: Option<i32>,
    pub r#ref: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FishTableQuery {
    pub r#ref: Option<String>,
}

pub async fn list_fish(
    State(state): State<SharedState>,
    headers: HeaderMap,
    query: Result<Query<FishQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<impl IntoResponse> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let lang = FishLang::from_param(query.lang.as_deref());
    let mut response = with_timeout(
        state.config.request_timeout_secs,
        state.store.list_fish(lang, query.r#ref),
    )
    .await
    .map_err(|err| map_request_id(err, &request_id))?;

    for entry in &mut response.fish {
        if let Some(icon_url) = entry.icon_url.as_deref() {
            entry.icon_url = Some(resolve_public_url(
                &headers,
                icon_url,
                state.config.images_public_base_url.as_deref(),
            ));
        }
    }

    let etag = format!("\"{}\"", response.revision);
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    response_headers.insert(
        header::ETAG,
        HeaderValue::from_str(&etag)
            .map_err(|err| AppError::internal(format!("invalid fish etag: {err}")))?,
    );

    if request_etag_matches(&headers, &etag) {
        return Ok((StatusCode::NOT_MODIFIED, response_headers).into_response());
    }

    if is_datastar_request(&headers) {
        response_headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/event-stream"),
        );
        return Ok((response_headers, datastar_fish_sse(response)?).into_response());
    }

    Ok((response_headers, Json(response)).into_response())
}

pub async fn fish_table(
    State(state): State<SharedState>,
    headers: HeaderMap,
    query: Result<Query<FishTableQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<Json<FishTableResponse>> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let fish = with_timeout(
        state.config.request_timeout_secs,
        state.store.fish_table(query.r#ref),
    )
    .await
    .map_err(|err| map_request_id(err, &request_id))?;
    let mut response = FishTableResponse { fish };
    for entry in &mut response.fish {
        if let Some(icon) = entry.icon.as_deref() {
            entry.icon = Some(resolve_public_url(
                &headers,
                icon,
                state.config.images_public_base_url.as_deref(),
            ));
        }
        if let Some(icon) = entry.encyclopedia_icon.as_deref() {
            entry.encyclopedia_icon = Some(resolve_public_url(
                &headers,
                icon,
                state.config.images_public_base_url.as_deref(),
            ));
        }
    }
    Ok(Json(response))
}

pub async fn fish_map(
    State(state): State<SharedState>,
    query: Result<Query<FishMapQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<Json<FishMapResponse>> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let encyclopedia_key = query.encyclopedia_key;
    let item_key = query.item_key;

    if encyclopedia_key.is_none() && item_key.is_none() {
        return Err(AppError::invalid_argument(
            "missing query param: encyclopedia_key or item_key",
        )
        .with_request_id(request_id.0));
    }

    let mapping = with_timeout(
        state.config.request_timeout_secs,
        state
            .store
            .fish_map(encyclopedia_key, item_key, query.r#ref),
    )
    .await
    .map_err(|err| map_request_id(err, &request_id))?;

    let mapping = mapping.ok_or_else(|| {
        AppError::not_found("fish mapping not found").with_request_id(request_id.0)
    })?;
    Ok(Json(mapping))
}

fn request_etag_matches(headers: &HeaderMap, current_etag: &str) -> bool {
    let Some(value) = headers.get(header::IF_NONE_MATCH) else {
        return false;
    };
    let Ok(value) = value.to_str() else {
        return false;
    };
    value
        .split(',')
        .map(str::trim)
        .any(|candidate| candidate == "*" || candidate == current_etag)
}

fn is_datastar_request(headers: &HeaderMap) -> bool {
    headers.contains_key("datastar-request")
}

fn datastar_fish_sse(response: FishListResponse) -> AppResult<String> {
    let signals = serde_json::to_string(&serde_json::json!({
        "revision": response.revision,
        "count": response.count,
        "fish": response.fish,
        "loading": false,
        "status_message": "",
        "api_error_message": "",
        "api_error_hint": "",
    }))
    .map_err(|err| AppError::internal(format!("serialize fish datastar payload: {err}")))?;
    Ok(format!(
        "event: datastar-merge-signals\ndata: signals {signals}\n\n"
    ))
}

fn resolve_public_url(headers: &HeaderMap, url: &str, configured_base: Option<&str>) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("data:")
    {
        return trimmed.to_string();
    }

    if let Some(base) = configured_base
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let base = base.trim_end_matches('/');
        if trimmed.starts_with('/') {
            return format!("{base}{trimmed}");
        }
        return format!("{base}/{}", trimmed.trim_start_matches('/'));
    }

    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("http");
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(header::HOST))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match host {
        Some(host) if trimmed.starts_with('/') => format!("{proto}://{host}{trimmed}"),
        Some(host) => format!("{proto}://{host}/{}", trimmed.trim_start_matches('/')),
        None => trimmed.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::extract::{Extension, Query, State};
    use axum::http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode};
    use axum::response::IntoResponse;
    use fishystuff_api::ids::MapVersionId;
    use fishystuff_api::models::effort::{EffortGridRequest, EffortGridResponse};
    use fishystuff_api::models::events::{EventsSnapshotMetaResponse, EventsSnapshotResponse};
    use fishystuff_api::models::fish::{
        FishEntry, FishListResponse, FishMapResponse, FishTableEntry,
    };
    use fishystuff_api::models::layers::LayersResponse;
    use fishystuff_api::models::meta::{MetaDefaults, MetaResponse};
    use fishystuff_api::models::region_groups::RegionGroupsResponse;
    use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};
    use fishystuff_api::models::zones::ZoneEntry;
    use hyper::body::to_bytes;

    use crate::config::{AppConfig, ZoneStatusConfig};
    use crate::error::AppResult;
    use crate::state::{AppState, RequestId};
    use crate::store::{FishLang, Store};

    use super::{list_fish, FishQuery};

    struct MockStore;

    #[async_trait]
    impl Store for MockStore {
        async fn get_meta(&self) -> AppResult<MetaResponse> {
            panic!("unused in test")
        }

        async fn get_layers(&self, _map_version_id: Option<String>) -> AppResult<LayersResponse> {
            panic!("unused in test")
        }

        async fn get_region_groups(
            &self,
            _map_version_id: Option<String>,
        ) -> AppResult<RegionGroupsResponse> {
            panic!("unused in test")
        }

        async fn list_fish(
            &self,
            _lang: FishLang,
            _ref_id: Option<String>,
        ) -> AppResult<FishListResponse> {
            Ok(FishListResponse {
                revision: "dolt:test-fish-rev".to_string(),
                count: 2,
                fish: vec![
                    FishEntry {
                        fish_id: 8474,
                        encyclopedia_key: Some(8474),
                        name: "Pirarucu".to_string(),
                        grade: Some("Prize".to_string()),
                        is_prize: Some(true),
                        icon_url: Some("/images/FishIcons/00008474.png".to_string()),
                        is_dried: false,
                        catch_methods: vec!["rod".to_string()],
                        vendor_price: Some(120_000_000),
                    },
                    FishEntry {
                        fish_id: 8201,
                        encyclopedia_key: Some(821001),
                        name: "Mudskipper".to_string(),
                        grade: Some("General".to_string()),
                        is_prize: Some(false),
                        icon_url: Some("/images/FishIcons/00008201.png".to_string()),
                        is_dried: true,
                        catch_methods: vec!["rod".to_string()],
                        vendor_price: Some(16_560),
                    },
                ],
            })
        }

        async fn list_zones(&self, _ref_id: Option<String>) -> AppResult<Vec<ZoneEntry>> {
            panic!("unused in test")
        }

        async fn fish_table(&self, _ref_id: Option<String>) -> AppResult<Vec<FishTableEntry>> {
            panic!("unused in test")
        }

        async fn fish_map(
            &self,
            _encyclopedia_key: Option<i32>,
            _item_key: Option<i32>,
            _ref_id: Option<String>,
        ) -> AppResult<Option<FishMapResponse>> {
            panic!("unused in test")
        }

        async fn zone_stats(
            &self,
            _request: ZoneStatsRequest,
            _status_cfg: ZoneStatusConfig,
        ) -> AppResult<ZoneStatsResponse> {
            panic!("unused in test")
        }

        async fn effort_grid(&self, _request: EffortGridRequest) -> AppResult<EffortGridResponse> {
            panic!("unused in test")
        }

        async fn events_snapshot_meta(&self) -> AppResult<EventsSnapshotMetaResponse> {
            panic!("unused in test")
        }

        async fn events_snapshot(
            &self,
            _requested_revision: Option<String>,
        ) -> AppResult<EventsSnapshotResponse> {
            panic!("unused in test")
        }

        async fn healthcheck(&self) -> AppResult<()> {
            panic!("unused in test")
        }
    }

    fn test_state() -> Arc<AppState> {
        let config = AppConfig {
            bind: "127.0.0.1:0".to_string(),
            database_url: "mysql://unused".to_string(),
            images_public_base_url: None,
            terrain_manifest_url: None,
            terrain_drape_manifest_url: None,
            terrain_height_tiles_url: None,
            defaults: MetaDefaults {
                tile_px: 32,
                sigma_tiles: 3.0,
                half_life_days: None,
                alpha0: 1.0,
                top_k: 30,
                map_version_id: Some(MapVersionId("v1".to_string())),
            },
            status_cfg: ZoneStatusConfig::default(),
            cache_zone_stats_max: 4,
            cache_effort_max: 4,
            cache_log: false,
            request_timeout_secs: 5,
        };
        AppState::for_tests(config, Arc::new(MockStore))
    }

    #[tokio::test]
    async fn list_fish_route_returns_revisioned_json_and_cache_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("api.example.test"));
        headers.insert(
            HeaderName::from_static("x-forwarded-proto"),
            HeaderValue::from_static("https"),
        );

        let response = list_fish(
            State(test_state()),
            headers,
            Ok(Query(FishQuery {
                lang: None,
                r#ref: None,
            })),
            Extension(RequestId("req-test".to_string())),
        )
        .await
        .expect("fish response")
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("public, max-age=300")
        );
        assert_eq!(
            response
                .headers()
                .get(header::ETAG)
                .and_then(|value| value.to_str().ok()),
            Some("\"dolt:test-fish-rev\"")
        );

        let body = to_bytes(response.into_body()).await.expect("body bytes");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(payload["revision"], "dolt:test-fish-rev");
        assert_eq!(payload["count"], 2);
        let fish = payload["fish"].as_array().expect("fish array");
        assert_eq!(fish.len(), 2);
        assert_eq!(fish[0]["fish_id"], 8474);
        assert_eq!(fish[0]["grade"], "Prize");
        assert_eq!(fish[0]["is_prize"], true);
        assert_eq!(fish[0]["is_dried"], false);
        assert_eq!(fish[0]["catch_methods"][0], "rod");
        assert_eq!(fish[0]["vendor_price"], 120000000);
        assert_eq!(
            fish[0]["icon_url"],
            "https://api.example.test/images/FishIcons/00008474.png"
        );
        assert_eq!(fish[1]["is_dried"], true);
    }

    #[tokio::test]
    async fn list_fish_route_returns_not_modified_for_matching_etag() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::IF_NONE_MATCH,
            HeaderValue::from_static("\"dolt:test-fish-rev\""),
        );

        let response = list_fish(
            State(test_state()),
            headers,
            Ok(Query(FishQuery {
                lang: None,
                r#ref: None,
            })),
            Extension(RequestId("req-test".to_string())),
        )
        .await
        .expect("fish response")
        .into_response();

        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        let body = to_bytes(response.into_body()).await.expect("body bytes");
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn list_fish_route_returns_datastar_signal_patch_for_datastar_requests() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("datastar-request"),
            HeaderValue::from_static("true"),
        );

        let response = list_fish(
            State(test_state()),
            headers,
            Ok(Query(FishQuery {
                lang: None,
                r#ref: None,
            })),
            Extension(RequestId("req-test".to_string())),
        )
        .await
        .expect("fish response")
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/event-stream")
        );
        let body = to_bytes(response.into_body()).await.expect("body bytes");
        let text = String::from_utf8(body.to_vec()).expect("utf8 body");
        assert!(text.contains("event: datastar-merge-signals"));
        assert!(text.contains("data: signals "));
        assert!(text.contains("\"loading\":false"));
        assert!(text.contains("\"count\":2"));
        assert!(text.contains("\"fish_id\":8474"));
    }
}
