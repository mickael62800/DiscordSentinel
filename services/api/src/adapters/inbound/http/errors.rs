use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::domain::errors::DomainError;

pub struct ApiError(pub DomainError);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            // 404
            DomainError::RuleNotFound(_)
            | DomainError::InfractionNotFound(_)
            | DomainError::TicketNotFound(_)
            | DomainError::NotFound(_) => (StatusCode::NOT_FOUND, self.0.to_string()),

            // 400
            DomainError::InvalidRule(_) => (StatusCode::BAD_REQUEST, self.0.to_string()),

            // 422
            DomainError::ValidationError(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.0.to_string())
            }

            // 403
            DomainError::Forbidden(_) => (StatusCode::FORBIDDEN, self.0.to_string()),

            // 409
            DomainError::Conflict(_) => (StatusCode::CONFLICT, self.0.to_string()),

            // 429
            DomainError::RateLimited(_) => (StatusCode::TOO_MANY_REQUESTS, self.0.to_string()),

            // 504
            DomainError::Timeout(_) => (StatusCode::GATEWAY_TIMEOUT, self.0.to_string()),

            // 500
            DomainError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Erreur interne".to_string(),
            ),
        };

        tracing::error!(status = %status, error = %self.0, "Erreur API");

        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        ApiError(err)
    }
}

#[cfg(test)]
#[path = "tests/errors.rs"]
mod tests;
