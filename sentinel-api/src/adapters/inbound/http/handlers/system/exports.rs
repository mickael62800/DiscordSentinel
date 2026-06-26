//! Phase 6A — Handlers de la file d'attente des jobs d'export.
//!
//! Architecture identique a ai_jobs (Phase 4 A) : POST renvoie 202 immediat,
//! l'export-worker depile, execute la query, serialise le resultat et le
//! stocke inline dans `result` (TEXT). Les clients poll via GET pour recuperer.
//!
//! Gates RBAC : `POST` requiert `Moderator+` pour eviter qu'un viewer puisse
//! dumper toute la moderation d'un guild. Le GET est ouvert a tout role
//! (on assume que si quelqu'un a le job_id, il l'a demande lui-meme).

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::errors_helpers::sqlx_internal;
use crate::adapters::inbound::http::middleware::rbac::check_role_for_guild;
use sentinel_core::domain::enums::system::role::Role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::domain::entities::system::discord_ids::GuildId;

#[derive(Debug, Deserialize)]
pub struct CreateExportJobDto {
    pub guild_id: GuildId,
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

    if !sentinel_core::domain::entities::system::job_whitelists::is_valid_export_job_type(&dto.job_type) {
        return Err(ApiError(DomainError::ValidationError(format!(
            "job_type invalide : '{}'",
            dto.job_type
        ))));
    }
    if !sentinel_core::domain::entities::system::job_whitelists::is_valid_export_format(&dto.format) {
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
    .bind(dto.guild_id.as_str())
    .bind(&dto.requested_by)
    .bind(&dto.job_type)
    .bind(&dto.format)
    .bind(&dto.filters)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(sqlx_internal("export_jobs insert"))?;

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
    let uuid = validation::parse_uuid("job_id", &id).map_err(ApiError)?;

    let job = sqlx::query_as::<_, ExportJobStatusDto>(
        "SELECT id, guild_id, requested_by, job_type, format, status, result, result_rows, \
                error_message, retries, created_at, started_at, completed_at \
         FROM export_jobs WHERE id = $1",
    )
    .bind(uuid)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(sqlx_internal("export_jobs select"))?
    .ok_or_else(|| ApiError(DomainError::NotFound(format!("export_job {id}"))))?;

    Ok(Json(job))
}

#[cfg(test)]
#[path = "tests/exports.rs"]
mod tests;
