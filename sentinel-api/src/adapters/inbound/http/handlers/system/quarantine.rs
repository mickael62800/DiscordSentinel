//! Phase 5F — Endpoints minimalistes pour `security_quarantine_pending`.
//! SQL direct (pas de port/adapter) — meme principe que steal_attempts.

use crate::adapters::inbound::http::errors_helpers::sqlx_internal;
use crate::adapters::inbound::http::extractors::ValidatedGuildUser;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

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
    let expires_at: DateTime<Utc> = Utc::now() + chrono::Duration::seconds(dto.timeout_secs.max(1));
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
    .map_err(sqlx_internal("upsert quarantine"))?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/security/quarantine/active — liste les quarantaines encore actives
/// (non expirees). Le bot l'appelle au demarrage pour rehydrater son tracker RAM
/// (sinon, apres un reboot, un user quarantine ne peut plus se verifier et sa
/// quarantaine ne peut plus etre levee cote bot).
pub async fn list_active_quarantines(
    State(state): State<AppState>,
) -> Result<Json<Vec<(String, String)>>, ApiError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT guild_id, user_id FROM security_quarantine_pending WHERE expires_at > NOW()",
    )
    .fetch_all(&state.pg_pool)
    .await
    .map_err(sqlx_internal("list active quarantines"))?;
    Ok(Json(rows))
}

/// DELETE /api/security/quarantine/{guild_id}/{user_id} — bot retire un
/// user de la quarantaine apres validation captcha (ou suppression par
/// admin). Idempotent.
pub async fn delete_quarantine(
    State(state): State<AppState>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<StatusCode, ApiError> {
    sqlx::query("DELETE FROM security_quarantine_pending WHERE guild_id = $1 AND user_id = $2")
        .bind(&guild_id)
        .bind(&user_id)
        .execute(&state.pg_pool)
        .await
        .map_err(sqlx_internal("delete quarantine"))?;
    Ok(StatusCode::NO_CONTENT)
}
