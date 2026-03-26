use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::domain::errors::DomainError;

pub struct ApiError(pub DomainError);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            DomainError::RuleNotFound(_) | DomainError::InfractionNotFound(_) => {
                (StatusCode::NOT_FOUND, self.0.to_string())
            }
            DomainError::InvalidRule(_) => (StatusCode::BAD_REQUEST, self.0.to_string()),
            DomainError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Erreur interne".to_string(),
            ),
        };

        tracing::error!(error = %self.0, "Erreur API");

        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        ApiError(err)
    }
}
