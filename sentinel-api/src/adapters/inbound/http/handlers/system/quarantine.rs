//! Phase 5F — Endpoints minimalistes pour `security_quarantine_pending`.
//! SQL direct (pas de port/adapter) — meme principe que steal_attempts.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::errors::DomainError;

#[derive(Deserialize)]
pub struct CreateQuarantineDto {
    pub guild_id: String,
    pub user_id: String,
    /// Duree avant kick automatique (secondes).
    pub timeout_secs: i64,
}

/// POST /api/security/quarantine — bot enregistre la mise en quarantaine
/// d'un user. UPSERT pour idempotence (re-quarantaine reset le timer).
pub async fn create_quarantine(
    State(state): State<AppState>,
    Json(dto): Json<CreateQuarantineDto>,
) -> Result<StatusCode, ApiError> {
    let expires_at: DateTime<Utc> =
        Utc::now() + chrono::Duration::seconds(dto.timeout_secs.max(1));
    sqlx::query(
        "INSERT INTO security_quarantine_pending (guild_id, user_id, expires_at) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (guild_id, user_id) DO UPDATE SET expires_at = EXCLUDED.expires_at",
    )
    .bind(&dto.guild_id)
    .bind(&dto.user_id)
    .bind(expires_at)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("upsert quarantine: {e}"))))?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/security/quarantine/{guild_id}/{user_id} — bot retire un
/// user de la quarantaine apres validation captcha (ou suppression par
/// admin). Idempotent.
pub async fn delete_quarantine(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    sqlx::query(
        "DELETE FROM security_quarantine_pending WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("delete quarantine: {e}"))))?;
    Ok(StatusCode::NO_CONTENT)
}
