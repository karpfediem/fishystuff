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
        .route("/fish_table", get(routes::fish::fish_table))
        .route("/fish_map", get(routes::fish::fish_map))
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
        .nest("/api/v1", api)
        .with_state(state)
        .layer(middleware::from_fn(request_id_middleware))
        .layer(build_cors_layer())
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}

fn build_cors_layer() -> CorsLayer {
    let datastar_request = HeaderName::from_static("datastar-request");
    let allowed_origins = AllowOrigin::list([
        HeaderValue::from_static("http://localhost:1990"),
        HeaderValue::from_static("http://127.0.0.1:1990"),
        HeaderValue::from_static("https://fishystuff.fish"),
        HeaderValue::from_static("https://www.fishystuff.fish"),
    ]);

    CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::ACCEPT, header::CONTENT_TYPE, datastar_request])
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
