//! Phase 5 — Endpoints pour la table `coude_steal_attempts`.
//!
//! Le bot Discord persiste chaque /voler ici (au lieu de lancer un
//! `tokio::spawn(sleep 60s)` qui mourrait avec le process). Le worker
//! `expire_steals` (sentinel-worker, domaine coude) scanne les pending
//! expires et publie un event Redis pour declencher la resolution AFK
//! cote bot.
//!
//! Implementation pragmatique : SQL direct dans le handler (pas de
//! port/adapter/use case). Acceptable parce que :
//!   - Pas de logique metier ici, juste persistance simple.
//!   - Pattern aligne avec `bot_persistence.rs` qui fait pareil pour
//!     les heartbeats / lifecycle logs.

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;

#[derive(Deserialize)]
pub struct CreateStealAttemptDto {
    pub guild_id: String,
    pub thief_id: String,
    pub target_id: String,
    pub message_id: String,
    pub channel_id: String,
    /// Duree de la fenetre de defense en secondes. Le bot envoie 60.
    pub window_secs: i64,
}

#[derive(Serialize)]
pub struct StealAttemptDto {
    pub id: Uuid,
    pub expires_at: DateTime<Utc>,
}

/// POST /api/coude/steals — bot cree une tentative quand /voler est lance.
pub async fn create_steal_attempt(
    State(state): State<AppState>,
    Json(dto): Json<CreateStealAttemptDto>,
) -> Result<Json<StealAttemptDto>, ApiError> {
    let id = Uuid::new_v4();
    let expires_at = Utc::now() + chrono::Duration::seconds(dto.window_secs.max(1));

    sqlx::query(
        "INSERT INTO coude_steal_attempts \
         (id, guild_id, thief_id, target_id, message_id, channel_id, expires_at, status) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending')",
    )
    .bind(id)
    .bind(&dto.guild_id)
    .bind(&dto.thief_id)
    .bind(&dto.target_id)
    .bind(&dto.message_id)
    .bind(&dto.channel_id)
    .bind(expires_at)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("create steal_attempt: {e}"))))?;

    Ok(Json(StealAttemptDto { id, expires_at }))
}

/// PATCH /api/coude/steals/{id}/defend — la victime a clique le bouton.
/// Marque pending -> defended (atomique, idempotent).
pub async fn mark_defended(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let res = sqlx::query(
        "UPDATE coude_steal_attempts SET status = 'defended' \
         WHERE id = $1 AND status = 'pending'",
    )
    .bind(id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("mark defended: {e}"))))?;

    if res.rows_affected() == 0 {
        // Soit deja defended/expired, soit id inconnu. Retourne 200
        // quand meme — l'idempotence cote bot evite de re-resoudre.
        return Ok(StatusCode::NO_CONTENT);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// PATCH /api/coude/steals/{id}/resolved — le bot a fini la resolution.
/// Marque le row resolved (etat final).
pub async fn mark_resolved(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    sqlx::query(
        "UPDATE coude_steal_attempts SET status = 'resolved', resolved_at = NOW() \
         WHERE id = $1 AND status IN ('pending','defended','expired')",
    )
    .bind(id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("mark resolved: {e}"))))?;
    Ok(StatusCode::NO_CONTENT)
}
