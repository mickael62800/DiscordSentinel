//! Phase 5G — Endpoints pour `security_lockdown_active`.
//! SQL direct (meme principe que steal_attempts / quarantine).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::errors::DomainError;

#[derive(Deserialize)]
pub struct CreateLockdownDto {
    pub guild_id: String,
    /// JSON array decrivant les overwrites originaux par salon.
    /// Cf domains/security/expire_lockdown.rs cote worker pour le format.
    pub saved_states: serde_json::Value,
    pub duration_secs: i64,
}

/// POST /api/security/lockdown — bot enregistre un lockdown actif.
/// UPSERT pour idempotence (re-activation reset le timer + states).
pub async fn create_lockdown(
    State(state): State<AppState>,
    Json(dto): Json<CreateLockdownDto>,
) -> Result<StatusCode, ApiError> {
    let expires_at: DateTime<Utc> =
        Utc::now() + chrono::Duration::seconds(dto.duration_secs.max(1));
    sqlx::query(
        "INSERT INTO security_lockdown_active (guild_id, saved_states, expires_at) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (guild_id) DO UPDATE SET \
             saved_states = EXCLUDED.saved_states, \
             expires_at = EXCLUDED.expires_at",
    )
    .bind(&dto.guild_id)
    .bind(&dto.saved_states)
    .bind(expires_at)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("upsert lockdown: {e}"))))?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/security/lockdown/{guild_id} — bot retire un lockdown
/// (deactivation manuelle ou via worker).
pub async fn delete_lockdown(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    sqlx::query("DELETE FROM security_lockdown_active WHERE guild_id = $1")
        .bind(&guild_id)
        .execute(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("delete lockdown: {e}"))))?;
    Ok(StatusCode::NO_CONTENT)
}
