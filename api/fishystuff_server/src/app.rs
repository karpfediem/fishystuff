use axum::extract::MatchedPath;
use axum::http::{header, HeaderName, HeaderValue, Method, Request};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use opentelemetry::global;
use opentelemetry::propagation::Extractor;
use opentelemetry::trace::{Status, TraceContextExt};
use std::time::Instant;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use uuid::Uuid;

use crate::routes;
use crate::state::{RequestId, SharedState};

pub fn build_router(state: SharedState) -> Router {
    let cors = build_cors_layer(&state.config.cors_allowed_origins);
    let api = Router::new()
        .route("/meta", get(routes::meta::get_meta))
        .route(
            "/region_groups",
            get(routes::region_groups::get_region_groups),
        )
        .route("/zones", get(routes::zones::list_zones))
        .route("/fish", get(routes::fish::list_fish))
        .route("/fish/", get(routes::fish::list_fish))
        .route(
            "/fish/community_zone_support",
            get(routes::fish::community_fish_zone_support),
        )
        .route(
            "/fish/community_zone_support/",
            get(routes::fish::community_fish_zone_support),
        )
        .route("/fish/:item_id/spots", get(routes::fish::fish_best_spots))
        .route("/fish/:item_id/spots/", get(routes::fish::fish_best_spots))
        .route(
            "/calculator",
            get(routes::calculator::get_calculator_catalog),
        )
        .route(
            "/calculator/datastar/init",
            get(routes::calculator::get_calculator_datastar_init)
                .post(routes::calculator::post_calculator_datastar_init),
        )
        .route(
            "/calculator/datastar/eval",
            post(routes::calculator::post_calculator_datastar_eval),
        )
        .route(
            "/calculator/datastar/zone-search",
            get(routes::calculator::get_calculator_datastar_zone_search),
        )
        .route(
            "/calculator/datastar/option-search",
            get(routes::calculator::get_calculator_datastar_option_search),
        )
        .route(
            "/zone_loot_summary",
            post(routes::calculator::post_zone_loot_summary),
        )
        .route(
            "/zone_profile_v2",
            post(routes::zone_profile_v2::zone_profile_v2),
        )
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
        .layer(middleware::from_fn(request_trace_middleware))
        .layer(middleware::from_fn(request_id_middleware))
        .layer(cors)
        .layer(CompressionLayer::new())
}

fn build_cors_layer(cors_allowed_origins: &[String]) -> CorsLayer {
    let datastar_request = HeaderName::from_static("datastar-request");
    let traceparent = HeaderName::from_static("traceparent");
    let tracestate = HeaderName::from_static("tracestate");
    let baggage = HeaderName::from_static("baggage");
    let x_trace_id = HeaderName::from_static("x-trace-id");
    let x_span_id = HeaderName::from_static("x-span-id");
    let x_request_id = HeaderName::from_static("x-request-id");
    let allowed_origins = cors_allowed_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect::<Vec<_>>();
    let allowed_origins = AllowOrigin::list(allowed_origins);

    CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::ACCEPT,
            header::CONTENT_TYPE,
            datastar_request,
            traceparent,
            tracestate,
            baggage,
        ])
        .expose_headers([x_request_id, x_trace_id, x_span_id])
}

struct AxumHeaderExtractor<'a>(&'a axum::http::HeaderMap);

impl Extractor for AxumHeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(HeaderName::as_str).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::util::ServiceExt;

    use fishystuff_api::models::calculator::CalculatorCatalogResponse;
    use fishystuff_api::models::effort::{EffortGridRequest, EffortGridResponse};
    use fishystuff_api::models::events::{EventsSnapshotMetaResponse, EventsSnapshotResponse};
    use fishystuff_api::models::fish::FishListResponse;
    use fishystuff_api::models::meta::MetaResponse;
    use fishystuff_api::models::region_groups::RegionGroupsResponse;
    use fishystuff_api::models::zone_profile_v2::{ZoneProfileV2Request, ZoneProfileV2Response};
    use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};
    use fishystuff_api::models::zones::ZoneEntry;

    use crate::config::{AppConfig, TelemetryConfig, ZoneStatusConfig};
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
        async fn calculator_catalog(
            &self,
            _lang: FishLang,
            _ref_id: Option<String>,
        ) -> crate::error::AppResult<CalculatorCatalogResponse> {
            Ok(CalculatorCatalogResponse::default())
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
        async fn zone_profile_v2(
            &self,
            _request: ZoneProfileV2Request,
            _status_cfg: ZoneStatusConfig,
        ) -> crate::error::AppResult<ZoneProfileV2Response> {
            Ok(ZoneProfileV2Response::default())
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
        async fn calculator_catalog(
            &self,
            _lang: FishLang,
            _ref_id: Option<String>,
        ) -> crate::error::AppResult<CalculatorCatalogResponse> {
            Ok(CalculatorCatalogResponse::default())
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
        async fn zone_profile_v2(
            &self,
            _request: ZoneProfileV2Request,
            _status_cfg: ZoneStatusConfig,
        ) -> crate::error::AppResult<ZoneProfileV2Response> {
            Ok(ZoneProfileV2Response::default())
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
            runtime_cdn_base_url: "http://127.0.0.1:4040".to_string(),
            defaults: Default::default(),
            status_cfg: ZoneStatusConfig::default(),
            cache_zone_stats_max: 16,
            cache_effort_max: 4,
            cache_log: false,
            request_timeout_secs: 5,
            telemetry: TelemetryConfig::default(),
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

async fn request_trace_middleware<B>(request: Request<B>, next: Next<B>) -> Response {
    let parent_context = global::get_text_map_propagator(|propagator| {
        propagator.extract(&AxumHeaderExtractor(request.headers()))
    });
    let method = request.method().clone();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or_else(|| request.uri().path())
        .to_string();
    let span = tracing::info_span!(
        "http.request",
        otel.kind = "server",
        http.request.method = %method,
        http.route = %route,
        request.id = tracing::field::Empty,
        trace.id = tracing::field::Empty,
        span.id = tracing::field::Empty,
        http.response.status_code = tracing::field::Empty,
        duration.ms = tracing::field::Empty,
        error.code = tracing::field::Empty,
        error.message = tracing::field::Empty,
        error.details = tracing::field::Empty,
    );
    let _ = span.set_parent(parent_context);
    sync_span_context_fields(&span);

    let started_at = Instant::now();
    let mut response = next.run(request).instrument(span.clone()).await;
    let elapsed_ms = started_at.elapsed().as_secs_f64() * 1000.0;
    if let Some(request_id) = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
    {
        span.record("request.id", tracing::field::display(request_id));
    }
    span.record(
        "http.response.status_code",
        tracing::field::display(response.status().as_u16()),
    );
    span.record("duration.ms", tracing::field::display(elapsed_ms));
    if response.status().is_server_error() {
        span.set_status(Status::error(format!(
            "HTTP {}",
            response.status().as_u16()
        )));
    }
    let (trace_id, span_id) = sync_span_context_fields(&span);
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let status_code = response.status().as_u16();

    if response.status().is_client_error() || response.status().is_server_error() {
        tracing::warn!(
            http.request.method = %method,
            http.route = %route,
            http.response.status_code = status_code,
            request.id = request_id,
            trace.id = trace_id.as_str(),
            span.id = span_id.as_str(),
            duration.ms = elapsed_ms,
            "request completed"
        );
    } else {
        tracing::info!(
            http.request.method = %method,
            http.route = %route,
            http.response.status_code = status_code,
            request.id = request_id,
            trace.id = trace_id.as_str(),
            span.id = span_id.as_str(),
            duration.ms = elapsed_ms,
            "request completed"
        );
    }

    if !trace_id.is_empty() {
        if let Ok(trace_id) = HeaderValue::from_str(&trace_id) {
            response
                .headers_mut()
                .insert(HeaderName::from_static("x-trace-id"), trace_id);
        }
    }
    if !span_id.is_empty() {
        if let Ok(span_id) = HeaderValue::from_str(&span_id) {
            response
                .headers_mut()
                .insert(HeaderName::from_static("x-span-id"), span_id);
        }
    }

    response
}

fn sync_span_context_fields(span: &tracing::Span) -> (String, String) {
    let span_context = span.context().span().span_context().clone();
    if !span_context.is_valid() {
        return (String::new(), String::new());
    }

    let trace_id = span_context.trace_id().to_string();
    let span_id = span_context.span_id().to_string();
    span.record("trace.id", tracing::field::display(&trace_id));
    span.record("span.id", tracing::field::display(&span_id));
    (trace_id, span_id)
}
