use anyhow::Error as AnyError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use opentelemetry::trace::TraceContextExt;
use std::future::Future;
use std::time::Duration;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use fishystuff_api::error::{ApiError, ApiErrorCode, ApiErrorEnvelope};

pub type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Debug, Clone)]
pub struct AppError(pub ApiError);

impl AppError {
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self(ApiError::invalid_argument(message))
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self(ApiError::not_found(message))
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self(ApiError::internal(message))
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self(ApiError::unavailable(message))
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.0 = self.0.with_request_id(request_id);
        self
    }
}

impl From<ApiError> for AppError {
    fn from(value: ApiError) -> Self {
        Self(value)
    }
}

impl From<AnyError> for AppError {
    fn from(value: AnyError) -> Self {
        Self(ApiError::internal(value.to_string()))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let api_error = self.0;
        let status = match api_error.code {
            ApiErrorCode::InvalidArgument => StatusCode::BAD_REQUEST,
            ApiErrorCode::NotFound => StatusCode::NOT_FOUND,
            ApiErrorCode::Conflict => StatusCode::CONFLICT,
            ApiErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ApiErrorCode::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorCode::Timeout => StatusCode::REQUEST_TIMEOUT,
            ApiErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let error_code = api_error_code_name(api_error.code);
        let details = api_error
            .details
            .as_ref()
            .map(|value| value.to_string())
            .unwrap_or_default();
        let span = Span::current();
        let span_context = span.context().span().span_context().clone();
        let (trace_id, span_id) = if span_context.is_valid() {
            let trace_id = span_context.trace_id().to_string();
            let span_id = span_context.span_id().to_string();
            span.record("trace.id", tracing::field::display(&trace_id));
            span.record("span.id", tracing::field::display(&span_id));
            (trace_id, span_id)
        } else {
            (String::new(), String::new())
        };
        span.record(
            "http.response.status_code",
            tracing::field::display(status.as_u16()),
        );
        span.record("error.code", tracing::field::display(error_code));
        span.record("error.message", tracing::field::display(&api_error.message));
        if !details.is_empty() {
            span.record("error.details", tracing::field::display(&details));
        }
        if let Some(request_id) = api_error
            .request_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            span.record("request.id", tracing::field::display(request_id));
        }
        tracing::warn!(
            http.response.status_code = status.as_u16(),
            error.code = error_code,
            error.message = %api_error.message,
            error.details = %details,
            request.id = api_error.request_id.as_deref().unwrap_or(""),
            trace.id = %trace_id,
            span.id = %span_id,
            "request failed"
        );

        (status, Json(ApiErrorEnvelope { error: api_error })).into_response()
    }
}

fn api_error_code_name(code: ApiErrorCode) -> &'static str {
    match code {
        ApiErrorCode::InvalidArgument => "INVALID_ARGUMENT",
        ApiErrorCode::NotFound => "NOT_FOUND",
        ApiErrorCode::Internal => "INTERNAL",
        ApiErrorCode::Unavailable => "UNAVAILABLE",
        ApiErrorCode::Timeout => "TIMEOUT",
        ApiErrorCode::Conflict => "CONFLICT",
        ApiErrorCode::Unauthorized => "UNAUTHORIZED",
        ApiErrorCode::Forbidden => "FORBIDDEN",
    }
}

pub async fn with_timeout<T, F>(seconds: u64, fut: F) -> AppResult<T>
where
    F: Future<Output = AppResult<T>>,
{
    tokio::time::timeout(Duration::from_secs(seconds), fut)
        .await
        .map_err(|_| AppError::from(ApiError::timeout("request timed out")))?
}
