//! Phase 4 A — Handlers de la file d'attente des jobs IA.
//!
//! Approche queue async : les bots POSTent un job (retour 202 immediat avec
//! `job_id`) au lieu d'attendre la reponse synchrone des endpoints `/analyze`.
//! L'ai-worker depile et appelle les services d'inference. Les bots peuvent
//! soit poll `GET /api/ai/jobs/:id`, soit ecouter Redis `ai_result:{job_id}`.
//!
//! Pragmatique : sqlx direct (comme bot_persistence.rs) — la couche est triviale
//! et pas couverte par une use case business.

use crate::adapters::inbound::http::errors_helpers::sqlx_internal;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use sentinel_core::domain::errors::DomainError;

#[derive(Debug, Deserialize)]
pub struct CreateAiJobDto {
    pub guild_id: String,
    /// "analyze_text" ou "analyze_image"
    pub job_type: String,
    pub input_payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct AiJobCreatedDto {
    pub job_id: String,
    pub status: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AiJobStatusDto {
    pub id: Uuid,
    pub guild_id: String,
    pub job_type: String,
    pub status: String,
    pub input_payload: serde_json::Value,
    pub result_payload: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub retries: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// POST /api/ai/jobs — enqueue un job IA. Retourne 202 immediatement.
pub async fn create_ai_job(
    State(state): State<AppState>,
    Json(dto): Json<CreateAiJobDto>,
) -> Result<(StatusCode, Json<AiJobCreatedDto>), ApiError> {
    if !sentinel_core::domain::entities::system::job_whitelists::is_valid_ai_job_type(&dto.job_type)
    {
        return Err(ApiError::from(DomainError::ValidationError(format!(
            "job_type invalide : '{}', attendu 'analyze_text' ou 'analyze_image'",
            dto.job_type
        ))));
    }
    if dto.guild_id.is_empty() {
        return Err(ApiError::from(DomainError::ValidationError(
            "guild_id requis".into(),
        )));
    }

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO ai_jobs (guild_id, job_type, input_payload) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(&dto.guild_id)
    .bind(&dto.job_type)
    .bind(&dto.input_payload)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(sqlx_internal("ai_jobs insert"))?;

    Ok((
        StatusCode::ACCEPTED,
        Json(AiJobCreatedDto {
            job_id: id.to_string(),
            status: "pending".to_string(),
        }),
    ))
}

/// GET /api/ai/jobs/{id} — recupere le statut courant d'un job IA.
pub async fn get_ai_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<AiJobStatusDto>, ApiError> {
    let uuid = validation::parse_uuid("job_id", &id).map_err(ApiError)?;

    let job = sqlx::query_as::<_, AiJobStatusDto>(
        "SELECT id, guild_id, job_type, status, input_payload, result_payload, \
                error_message, retries, created_at, started_at, completed_at \
         FROM ai_jobs WHERE id = $1",
    )
    .bind(uuid)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(sqlx_internal("ai_jobs select"))?
    .ok_or_else(|| ApiError::from(DomainError::NotFound(format!("ai_job {id}"))))?;

    Ok(Json(job))
}

#[cfg(test)]
#[path = "tests/ai_jobs.rs"]
mod tests;
