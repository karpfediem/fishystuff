use anyhow::Error as AnyError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::future::Future;
use std::time::Duration;

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
        let status = match self.0.code {
            ApiErrorCode::InvalidArgument => StatusCode::BAD_REQUEST,
            ApiErrorCode::NotFound => StatusCode::NOT_FOUND,
            ApiErrorCode::Conflict => StatusCode::CONFLICT,
            ApiErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ApiErrorCode::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            ApiErrorCode::Timeout => StatusCode::REQUEST_TIMEOUT,
            ApiErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(ApiErrorEnvelope { error: self.0 })).into_response()
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
