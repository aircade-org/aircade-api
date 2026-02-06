use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// Unified application error type that maps to JSON HTTP responses.
///
/// Matches the API specification error format: `{ "error": { "code": "...", "message": "..." } }`.
pub enum AppError {
    /// 400 Bad Request
    BadRequest(String),
    /// 401 Unauthorized
    Unauthorized(String),
    /// 403 Forbidden
    Forbidden(String),
    /// 404 Not Found
    NotFound(String),
    /// 409 Conflict
    Conflict(String),
    /// 413 Payload Too Large
    PayloadTooLarge(String),
    /// 422 Unprocessable Entity (generic, code defaults to `VALIDATION_ERROR`)
    UnprocessableEntity(String),
    /// 422 Unprocessable Entity with explicit error code
    Unprocessable(String, String),
    /// 500 Internal Server Error (wraps any error, logs details, returns generic message)
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST".to_string(), msg),
            Self::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED".to_string(), msg),
            Self::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN".to_string(), msg),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND".to_string(), msg),
            Self::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT".to_string(), msg),
            Self::PayloadTooLarge(msg) => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "PAYLOAD_TOO_LARGE".to_string(),
                msg,
            ),
            Self::UnprocessableEntity(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "VALIDATION_ERROR".to_string(),
                msg,
            ),
            Self::Unprocessable(code, msg) => (StatusCode::UNPROCESSABLE_ENTITY, code, msg),
            Self::Internal(err) => {
                tracing::error!("Internal server error: {err:#}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR".to_string(),
                    "An internal error occurred".to_string(),
                )
            }
        };

        (
            status,
            Json(json!({
                "error": {
                    "code": code,
                    "message": message,
                }
            })),
        )
            .into_response()
    }
}

/// Allow `?` to automatically convert any `anyhow::Error` into `AppError::Internal`.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::Internal(err.into())
    }
}
