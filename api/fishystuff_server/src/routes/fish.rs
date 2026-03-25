use axum::extract::{rejection::QueryRejection, Extension, Query, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use fishystuff_api::models::fish::FishListResponse;

use crate::error::{with_timeout, AppError, AppResult};
use crate::routes::meta::map_request_id;
use crate::state::{RequestId, SharedState};
use crate::store::FishLang;

#[derive(Debug, Deserialize)]
pub struct FishQuery {
    pub lang: Option<String>,
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
    let response = with_timeout(
        state.config.request_timeout_secs,
        state.store.list_fish(lang, query.r#ref),
    )
    .await
    .map_err(|err| map_request_id(err, &request_id))?;

    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));

    if is_datastar_request(&headers) {
        response_headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/event-stream"),
        );
        return Ok((response_headers, datastar_fish_sse(response)?).into_response());
    }

    Ok((response_headers, Json(response)).into_response())
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::extract::{Extension, Query, State};
    use axum::http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode};
    use axum::response::IntoResponse;
    use fishystuff_api::ids::MapVersionId;
    use fishystuff_api::models::calculator::CalculatorCatalogResponse;
    use fishystuff_api::models::effort::{EffortGridRequest, EffortGridResponse};
    use fishystuff_api::models::events::{EventsSnapshotMetaResponse, EventsSnapshotResponse};
    use fishystuff_api::models::fish::{FishEntry, FishListResponse};
    use fishystuff_api::models::meta::{MetaDefaults, MetaResponse};
    use fishystuff_api::models::region_groups::RegionGroupsResponse;
    use fishystuff_api::models::zone_profile_v2::{ZoneProfileV2Request, ZoneProfileV2Response};
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
                        item_id: 8474,
                        encyclopedia_key: Some(8474),
                        encyclopedia_id: Some(9474),
                        name: "Pirarucu".to_string(),
                        grade: Some("Prize".to_string()),
                        is_prize: Some(true),
                        is_dried: false,
                        catch_methods: vec!["rod".to_string()],
                        vendor_price: Some(120_000_000),
                    },
                    FishEntry {
                        item_id: 8201,
                        encyclopedia_key: Some(821001),
                        encyclopedia_id: Some(8501),
                        name: "Mudskipper".to_string(),
                        grade: Some("General".to_string()),
                        is_prize: Some(false),
                        is_dried: true,
                        catch_methods: vec!["rod".to_string()],
                        vendor_price: Some(16_560),
                    },
                ],
            })
        }

        async fn calculator_catalog(
            &self,
            _lang: FishLang,
            _ref_id: Option<String>,
        ) -> AppResult<CalculatorCatalogResponse> {
            panic!("unused in test")
        }

        async fn list_zones(&self, _ref_id: Option<String>) -> AppResult<Vec<ZoneEntry>> {
            panic!("unused in test")
        }

        async fn zone_stats(
            &self,
            _request: ZoneStatsRequest,
            _status_cfg: ZoneStatusConfig,
        ) -> AppResult<ZoneStatsResponse> {
            panic!("unused in test")
        }

        async fn zone_profile_v2(
            &self,
            _request: ZoneProfileV2Request,
            _status_cfg: ZoneStatusConfig,
        ) -> AppResult<ZoneProfileV2Response> {
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
            cors_allowed_origins: vec!["https://fishystuff.fish".to_string()],
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
    async fn list_fish_route_returns_revisioned_json_and_no_store_headers() {
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
            Some("no-store")
        );

        let body = to_bytes(response.into_body()).await.expect("body bytes");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(payload["revision"], "dolt:test-fish-rev");
        assert_eq!(payload["count"], 2);
        let fish = payload["fish"].as_array().expect("fish array");
        assert_eq!(fish.len(), 2);
        assert_eq!(fish[0]["item_id"], 8474);
        assert_eq!(fish[0]["encyclopedia_id"], 9474);
        assert_eq!(fish[0]["grade"], "Prize");
        assert_eq!(fish[0]["is_prize"], true);
        assert_eq!(fish[0]["is_dried"], false);
        assert_eq!(fish[0]["catch_methods"][0], "rod");
        assert_eq!(fish[0]["vendor_price"], 120000000);
        assert_eq!(fish[1]["is_dried"], true);
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
        assert!(text.contains("\"item_id\":8474"));
    }
}
