pub mod bot_config;
pub mod casino;
pub mod coussin;
pub mod game;
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
            DomainError::Validation(m) | DomainError::ValidationError(m) => {
                (StatusCode::BAD_REQUEST, m.clone())
            }
            DomainError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            DomainError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            DomainError::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone()),
            DomainError::RateLimited(m) => (StatusCode::TOO_MANY_REQUESTS, m.clone()),
            DomainError::Timeout(m) => (StatusCode::GATEWAY_TIMEOUT, m.clone()),
            DomainError::NotImplemented(m) => (StatusCode::NOT_IMPLEMENTED, m.clone()),
            DomainError::Infrastructure(m) | DomainError::Internal(m) => {
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

/// Validation minimale d'un snowflake Discord (remplace le module
/// `validation` de sentinel-api) : chiffres uniquement, longueur bornee.
pub fn validate_discord_id(field: &str, value: &str) -> Result<(), DomainError> {
    let ok = !value.is_empty()
        && value.len() <= 20
        && value.len() >= 15
        && value.chars().all(|c| c.is_ascii_digit());
    if ok {
        Ok(())
    } else {
        Err(DomainError::ValidationError(format!(
            "{field} invalide : snowflake Discord attendu"
        )))
    }
}
