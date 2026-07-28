pub mod wallet;
pub mod wheel;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use nexus_core::domain::errors::DomainError;

/// Enveloppe d'erreur API : mappe DomainError -> statut HTTP + JSON.
pub struct ApiError(pub DomainError);

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        Self(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self.0 {
            DomainError::Validation(m) => (StatusCode::BAD_REQUEST, m.clone()),
            DomainError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            DomainError::Infrastructure(m) => {
                tracing::error!(error = %m, "erreur infrastructure nexus-api");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "erreur interne".to_string(),
                )
            }
        };
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}
