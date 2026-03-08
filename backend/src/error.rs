use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Unified HTTP error type for Axum handlers.
/// Module-level snafu errors are converted into this at the handler boundary.
pub struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    pub fn bad_request(message: impl std::fmt::Display) -> Self {
        Self { status: StatusCode::BAD_REQUEST, message: message.to_string() }
    }

    pub fn not_found(message: impl std::fmt::Display) -> Self {
        Self { status: StatusCode::NOT_FOUND, message: message.to_string() }
    }

    pub fn internal(message: impl std::fmt::Display) -> Self {
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, message: message.to_string() }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}
