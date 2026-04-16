use axum::extract::{rejection::QueryRejection, Extension, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use fishystuff_api::models::fish::{FishBestSpotsResponse, FishListResponse};

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
    query: Result<Query<FishQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<(HeaderMap, Json<FishListResponse>)> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let lang = FishLang::from_param(query.lang.as_deref());
    let response = load_fish_response(&state, lang, query.r#ref, &request_id).await?;

    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((response_headers, Json(response)))
}

pub async fn fish_best_spots(
    State(state): State<SharedState>,
    Path(item_id): Path<i32>,
    query: Result<Query<FishQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<(HeaderMap, Json<FishBestSpotsResponse>)> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let lang = FishLang::from_param(query.lang.as_deref());
    let response =
        load_fish_best_spots_response(&state, lang, query.r#ref, item_id, &request_id).await?;

    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((response_headers, Json(response)))
}

async fn load_fish_response(
    state: &SharedState,
    lang: FishLang,
    ref_id: Option<String>,
    request_id: &RequestId,
) -> AppResult<FishListResponse> {
    with_timeout(
        state.config.request_timeout_secs,
        state.store.list_fish(lang, ref_id),
    )
    .await
    .map_err(|err| map_request_id(err, request_id))
}

async fn load_fish_best_spots_response(
    state: &SharedState,
    lang: FishLang,
    ref_id: Option<String>,
    item_id: i32,
    request_id: &RequestId,
) -> AppResult<FishBestSpotsResponse> {
    with_timeout(
        state.config.request_timeout_secs,
        state.store.fish_best_spots(lang, ref_id, item_id),
    )
    .await
    .map_err(|err| map_request_id(err, request_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::http::StatusCode;
    use fishystuff_api::ids::MapVersionId;
    use fishystuff_api::models::calculator::CalculatorCatalogResponse;
    use fishystuff_api::models::effort::{EffortGridRequest, EffortGridResponse};
    use fishystuff_api::models::events::{EventsSnapshotMetaResponse, EventsSnapshotResponse};
    use fishystuff_api::models::fish::{
        FishBestSpotEntry, FishBestSpotsResponse, FishEntry, FishListResponse,
    };
    use fishystuff_api::models::meta::{MetaDefaults, MetaResponse};
    use fishystuff_api::models::region_groups::RegionGroupsResponse;
    use fishystuff_api::models::zone_profile_v2::{ZoneProfileV2Request, ZoneProfileV2Response};
    use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};
    use fishystuff_api::models::zones::ZoneEntry;
    use hyper::body::to_bytes;

    use crate::config::{AppConfig, ZoneStatusConfig};
    use crate::state::AppState;
    use crate::store::Store;

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

        async fn fish_best_spots(
            &self,
            _lang: FishLang,
            _ref_id: Option<String>,
            item_id: i32,
        ) -> AppResult<FishBestSpotsResponse> {
            Ok(FishBestSpotsResponse {
                revision: "dolt:test-fish-rev".to_string(),
                item_id,
                count: 2,
                spots: vec![
                    FishBestSpotEntry {
                        zone_rgb: "240,74,74".to_string(),
                        zone_name: "Velia Beach".to_string(),
                        db_groups: vec!["Prize".to_string()],
                        community_groups: vec!["Prize".to_string()],
                        has_ranking_presence: true,
                        ranking_observation_count: Some(8),
                        ..FishBestSpotEntry::default()
                    },
                    FishBestSpotEntry {
                        zone_rgb: "10,20,30".to_string(),
                        zone_name: "Ancado".to_string(),
                        has_ranking_presence: true,
                        ranking_observation_count: Some(2),
                        ..FishBestSpotEntry::default()
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
            runtime_cdn_base_url: "http://127.0.0.1:4040".to_string(),
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
        let response = list_fish(
            State(test_state()),
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
        assert_eq!(fish[1]["is_dried"], true);
    }

    #[tokio::test]
    async fn fish_best_spots_route_returns_revisioned_json_and_no_store_headers() {
        let response = fish_best_spots(
            State(test_state()),
            Path(8474),
            Ok(Query(FishQuery {
                lang: None,
                r#ref: None,
            })),
            Extension(RequestId("req-test".to_string())),
        )
        .await
        .expect("fish best spots response")
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
        assert_eq!(payload["item_id"], 8474);
        assert_eq!(payload["count"], 2);
        let spots = payload["spots"].as_array().expect("spots array");
        assert_eq!(spots[0]["zone_name"], "Velia Beach");
        assert_eq!(spots[0]["db_groups"][0], "Prize");
        assert_eq!(spots[1]["has_ranking_presence"], true);
    }
}
