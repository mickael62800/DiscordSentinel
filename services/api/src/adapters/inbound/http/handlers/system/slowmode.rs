//! Phase 5H — Endpoints pour `security_slowmode_active`.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;

#[derive(Deserialize)]
pub struct CreateSlowmodeDto {
    pub guild_id: String,
    /// JSON array : [{channel_id, rate}, ...]
    pub previous_states: serde_json::Value,
    pub duration_secs: i64,
}

pub async fn create_slowmode(
    State(state): State<AppState>,
    Json(dto): Json<CreateSlowmodeDto>,
) -> Result<StatusCode, ApiError> {
    let expires_at: DateTime<Utc> =
        Utc::now() + chrono::Duration::seconds(dto.duration_secs.max(1));
    sqlx::query(
        "INSERT INTO security_slowmode_active (guild_id, previous_states, expires_at) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (guild_id) DO UPDATE SET \
             previous_states = EXCLUDED.previous_states, \
             expires_at = EXCLUDED.expires_at",
    )
    .bind(&dto.guild_id)
    .bind(&dto.previous_states)
    .bind(expires_at)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("upsert slowmode: {e}"))))?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_slowmode(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    sqlx::query("DELETE FROM security_slowmode_active WHERE guild_id = $1")
        .bind(&guild_id)
        .execute(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("delete slowmode: {e}"))))?;
    Ok(StatusCode::NO_CONTENT)
}
