//! Phase 6A — Handlers de la file d'attente des jobs d'export.
//!
//! Architecture identique a ai_jobs (Phase 4 A) : POST renvoie 202 immediat,
//! l'export-worker depile, execute la query, serialise le resultat et le
//! stocke inline dans `result` (TEXT). Les clients poll via GET pour recuperer.
//!
//! Gates RBAC : `POST` requiert `Moderator+` pour eviter qu'un viewer puisse
//! dumper toute la moderation d'un guild. Le GET est ouvert a tout role
//! (on assume que si quelqu'un a le job_id, il l'a demande lui-meme).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::{check_role_for_guild, Role, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use crate::domain::errors::DomainError;

#[derive(Debug, Deserialize)]
pub struct CreateExportJobDto {
    pub guild_id: String,
    pub requested_by: String,
    /// "infractions" | "audit_logs" | "moderation_actions"
    pub job_type: String,
    /// "csv" | "json"
    pub format: String,
    #[serde(default)]
    pub filters: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ExportJobCreatedDto {
    pub job_id: String,
    pub status: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ExportJobStatusDto {
    pub id: Uuid,
    pub guild_id: String,
    pub requested_by: String,
    pub job_type: String,
    pub format: String,
    pub status: String,
    pub result: Option<String>,
    pub result_rows: Option<i32>,
    pub error_message: Option<String>,
    pub retries: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// POST /api/exports/jobs — enqueue un job d'export. Retourne 202.
///
/// Gated `Moderator+` via `require_role_for_guild` (body-based : le guild_id
/// n'est pas dans l'URL).
pub async fn create_export_job(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<CreateExportJobDto>,
) -> Result<(StatusCode, Json<ExportJobCreatedDto>), ApiError> {
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("requested_by", &dto.requested_by).map_err(ApiError)?;

    if !matches!(
        dto.job_type.as_str(),
        "infractions" | "audit_logs" | "moderation_actions"
    ) {
        return Err(ApiError(DomainError::ValidationError(format!(
            "job_type invalide : '{}'",
            dto.job_type
        ))));
    }
    if !matches!(dto.format.as_str(), "csv" | "json") {
        return Err(ApiError(DomainError::ValidationError(format!(
            "format invalide : '{}' (attendu csv|json)",
            dto.format
        ))));
    }

    // Phase 7 B — Gate RBAC : moderator+ pour lancer un export
    check_role_for_guild(
        &state,
        &rbac,
        &dto.guild_id,
        Role::Moderator,
        "moderator+ requis pour lancer un export",
    )
    .await?;

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO export_jobs (guild_id, requested_by, job_type, format, filters) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(&dto.guild_id)
    .bind(&dto.requested_by)
    .bind(&dto.job_type)
    .bind(&dto.format)
    .bind(&dto.filters)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("export_jobs insert: {e}"))))?;

    Ok((
        StatusCode::ACCEPTED,
        Json(ExportJobCreatedDto {
            job_id: id.to_string(),
            status: "pending".into(),
        }),
    ))
}

/// GET /api/exports/jobs/{id} — statut + resultat (si done).
pub async fn get_export_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ExportJobStatusDto>, ApiError> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError(DomainError::ValidationError(format!("job_id invalide : {id}"))))?;

    let job = sqlx::query_as::<_, ExportJobStatusDto>(
        "SELECT id, guild_id, requested_by, job_type, format, status, result, result_rows, \
                error_message, retries, created_at, started_at, completed_at \
         FROM export_jobs WHERE id = $1",
    )
    .bind(uuid)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("export_jobs select: {e}"))))?
    .ok_or_else(|| ApiError(DomainError::NotFound(format!("export_job {id}"))))?;

    Ok(Json(job))
}
