use axum::http::{header, HeaderName, HeaderValue, Method, Request};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::routes;
use crate::state::{RequestId, SharedState};

pub fn build_router(state: SharedState) -> Router {
    let cors = build_cors_layer(&state.config.cors_allowed_origins);
    let api = Router::new()
        .route("/meta", get(routes::meta::get_meta))
        .route("/layers", get(routes::layers::get_layers))
        .route(
            "/region_groups",
            get(routes::region_groups::get_region_groups),
        )
        .route("/zones", get(routes::zones::list_zones))
        .route("/fish", get(routes::fish::list_fish))
        .route("/fish/", get(routes::fish::list_fish))
        .route("/zone_stats", post(routes::zone_stats::zone_stats))
        .route("/effort_grid", post(routes::effort::effort_grid))
        .route(
            "/events_snapshot_meta",
            get(routes::events::events_snapshot_meta),
        )
        .route("/events_snapshot", get(routes::events::events_snapshot))
        .route("/openapi.json", get(routes::meta::openapi_json));

    Router::new()
        .route("/healthz", get(routes::meta::healthz))
        .route("/readyz", get(routes::meta::readyz))
        .nest("/api/v1", api)
        .with_state(state)
        .layer(middleware::from_fn(request_id_middleware))
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}

fn build_cors_layer(cors_allowed_origins: &[String]) -> CorsLayer {
    let datastar_request = HeaderName::from_static("datastar-request");
    let allowed_origins = cors_allowed_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect::<Vec<_>>();
    let allowed_origins = AllowOrigin::list(allowed_origins);

    CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::ACCEPT, header::CONTENT_TYPE, datastar_request])
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::util::ServiceExt;

    use fishystuff_api::models::effort::{EffortGridRequest, EffortGridResponse};
    use fishystuff_api::models::events::{EventsSnapshotMetaResponse, EventsSnapshotResponse};
    use fishystuff_api::models::fish::FishListResponse;
    use fishystuff_api::models::layers::LayersResponse;
    use fishystuff_api::models::meta::MetaResponse;
    use fishystuff_api::models::region_groups::RegionGroupsResponse;
    use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};
    use fishystuff_api::models::zones::ZoneEntry;

    use crate::config::{AppConfig, ZoneStatusConfig};
    use crate::state::AppState;
    use crate::store::{FishLang, Store};

    use super::build_router;

    struct MockStore;
    struct HealthcheckStore {
        health_ok: bool,
    }

    #[async_trait::async_trait]
    impl Store for MockStore {
        async fn get_meta(&self) -> crate::error::AppResult<MetaResponse> {
            Ok(MetaResponse::default())
        }
        async fn get_layers(
            &self,
            _map_version_id: Option<String>,
        ) -> crate::error::AppResult<LayersResponse> {
            Ok(LayersResponse::default())
        }
        async fn get_region_groups(
            &self,
            _map_version_id: Option<String>,
        ) -> crate::error::AppResult<RegionGroupsResponse> {
            Ok(RegionGroupsResponse::default())
        }
        async fn list_fish(
            &self,
            _lang: FishLang,
            _ref_id: Option<String>,
        ) -> crate::error::AppResult<FishListResponse> {
            Ok(FishListResponse::default())
        }
        async fn list_zones(
            &self,
            _ref_id: Option<String>,
        ) -> crate::error::AppResult<Vec<ZoneEntry>> {
            Ok(Vec::new())
        }
        async fn zone_stats(
            &self,
            _request: ZoneStatsRequest,
            _status_cfg: ZoneStatusConfig,
        ) -> crate::error::AppResult<ZoneStatsResponse> {
            Ok(ZoneStatsResponse::default())
        }
        async fn effort_grid(
            &self,
            _request: EffortGridRequest,
        ) -> crate::error::AppResult<EffortGridResponse> {
            Ok(EffortGridResponse::default())
        }
        async fn events_snapshot_meta(
            &self,
        ) -> crate::error::AppResult<EventsSnapshotMetaResponse> {
            Ok(EventsSnapshotMetaResponse::default())
        }
        async fn events_snapshot(
            &self,
            _requested_revision: Option<String>,
        ) -> crate::error::AppResult<EventsSnapshotResponse> {
            Ok(EventsSnapshotResponse::default())
        }
        async fn healthcheck(&self) -> crate::error::AppResult<()> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Store for HealthcheckStore {
        async fn get_meta(&self) -> crate::error::AppResult<MetaResponse> {
            Ok(MetaResponse::default())
        }
        async fn get_layers(
            &self,
            _map_version_id: Option<String>,
        ) -> crate::error::AppResult<LayersResponse> {
            Ok(LayersResponse::default())
        }
        async fn get_region_groups(
            &self,
            _map_version_id: Option<String>,
        ) -> crate::error::AppResult<RegionGroupsResponse> {
            Ok(RegionGroupsResponse::default())
        }
        async fn list_fish(
            &self,
            _lang: FishLang,
            _ref_id: Option<String>,
        ) -> crate::error::AppResult<FishListResponse> {
            Ok(FishListResponse::default())
        }
        async fn list_zones(
            &self,
            _ref_id: Option<String>,
        ) -> crate::error::AppResult<Vec<ZoneEntry>> {
            Ok(Vec::new())
        }
        async fn zone_stats(
            &self,
            _request: ZoneStatsRequest,
            _status_cfg: ZoneStatusConfig,
        ) -> crate::error::AppResult<ZoneStatsResponse> {
            Ok(ZoneStatsResponse::default())
        }
        async fn effort_grid(
            &self,
            _request: EffortGridRequest,
        ) -> crate::error::AppResult<EffortGridResponse> {
            Ok(EffortGridResponse::default())
        }
        async fn events_snapshot_meta(
            &self,
        ) -> crate::error::AppResult<EventsSnapshotMetaResponse> {
            Ok(EventsSnapshotMetaResponse::default())
        }
        async fn events_snapshot(
            &self,
            _requested_revision: Option<String>,
        ) -> crate::error::AppResult<EventsSnapshotResponse> {
            Ok(EventsSnapshotResponse::default())
        }
        async fn healthcheck(&self) -> crate::error::AppResult<()> {
            if self.health_ok {
                Ok(())
            } else {
                Err(crate::error::AppError::unavailable("db not ready"))
            }
        }
    }

    fn test_config(origins: Vec<&str>) -> AppConfig {
        AppConfig {
            bind: "127.0.0.1:0".to_string(),
            database_url: "mysql://unused".to_string(),
            cors_allowed_origins: origins.into_iter().map(str::to_string).collect(),
            terrain_manifest_url: None,
            terrain_drape_manifest_url: None,
            terrain_height_tiles_url: None,
            defaults: Default::default(),
            status_cfg: ZoneStatusConfig::default(),
            cache_zone_stats_max: 16,
            cache_effort_max: 4,
            cache_log: false,
            request_timeout_secs: 5,
        }
    }

    #[tokio::test]
    async fn cors_allows_configured_origin() {
        let router = build_router(AppState::for_tests(
            test_config(vec!["https://fishystuff.fish", "http://127.0.0.1:1990"]),
            Arc::new(MockStore),
        ));

        let response = router
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/healthz")
                    .header("origin", "http://127.0.0.1:1990")
                    .header("access-control-request-method", "GET")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["access-control-allow-origin"],
            "http://127.0.0.1:1990"
        );
    }

    #[tokio::test]
    async fn cors_rejects_unconfigured_origin() {
        let router = build_router(AppState::for_tests(
            test_config(vec!["https://fishystuff.fish"]),
            Arc::new(MockStore),
        ));

        let response = router
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/healthz")
                    .header("origin", "http://127.0.0.1:1990")
                    .header("access-control-request-method", "GET")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response
            .headers()
            .get("access-control-allow-origin")
            .is_none());
    }

    #[tokio::test]
    async fn healthz_is_pure_liveness() {
        let router = build_router(AppState::for_tests(
            test_config(vec!["https://fishystuff.fish"]),
            Arc::new(HealthcheckStore { health_ok: false }),
        ));

        let response = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/healthz")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn readyz_reports_store_health() {
        let router = build_router(AppState::for_tests(
            test_config(vec!["https://fishystuff.fish"]),
            Arc::new(HealthcheckStore { health_ok: false }),
        ));

        let response = router
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/readyz")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}

async fn request_id_middleware<B>(mut request: Request<B>, next: Next<B>) -> Response {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    let mut response = next.run(request).await;
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(
            header::HeaderName::from_static("x-request-id"),
            header_value,
        );
    }

    response
}
