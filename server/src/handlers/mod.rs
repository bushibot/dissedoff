pub mod auth;
pub mod channels;
pub mod gateway;
pub mod server_info;
pub mod voice;

use axum::http::StatusCode;
use axum::Json;
use dissedoff_protocol::ApiError;

pub fn api_error(status: StatusCode, code: &str, message: impl Into<String>) -> (StatusCode, Json<ApiError>) {
    (
        status,
        Json(ApiError {
            error_code: code.to_string(),
            message: message.into(),
        }),
    )
}

pub fn auth_error_response(err: crate::auth::AuthError) -> (StatusCode, Json<ApiError>) {
    use crate::auth::AuthError::*;
    match err {
        MissingToken => api_error(StatusCode::UNAUTHORIZED, "missing_token", "Authorization: Bearer <token> header required"),
        InvalidToken => api_error(StatusCode::UNAUTHORIZED, "invalid_token", "Session token is invalid or expired"),
        Internal => api_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", "Internal error"),
    }
}
