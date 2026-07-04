//! Handlers « ban en sursis ».

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use sentinel_core::domain::entities::moderation::sursis::{Sursis, SursisStatus};
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::inbound::moderation::manage_sursis::CreateSursisCommand;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::state::AppState;
use crate::application::game::worker_jobs::JobReport;

#[derive(Debug, Serialize)]
pub struct SursisDto {
    pub id: Uuid,
    pub user_id: String,
    pub username: String,
    pub reason: String,
    pub saved_roles: Vec<String>,
    pub channel_id: Option<String>,
    pub status: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl From<Sursis> for SursisDto {
    fn from(s: Sursis) -> Self {
        Self {
            id: s.id,
            user_id: s.user_id,
            username: s.username,
            reason: s.reason,
            saved_roles: s.saved_roles,
            channel_id: s.channel_id,
            status: s.status.as_str().to_string(),
            expires_at: s.expires_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateSursisDto {
    pub user_id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub moderator_id: String,
    #[serde(default)]
    pub moderator_name: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub saved_roles: Vec<String>,
    pub channel_id: Option<String>,
}

/// POST /api/moderation/{guild_id}/sursis
pub async fn create_sursis(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<CreateSursisDto>,
) -> Result<Json<SursisDto>, ApiError> {
    // Delai depuis la config (parametrable), defaut 7 jours.
    let days = state
        .bot_config_repo
        .get_config(&guild_id, "moderation-bot")
        .await
        .ok()
        .and_then(|cfg| {
            cfg.iter()
                .find(|c| c.config_key == "sursis_appeal_days")
                .and_then(|c| c.config_value.parse::<i64>().ok())
        })
        .unwrap_or(7);

    let sursis = state
        .sursis_uc
        .create(CreateSursisCommand {
            guild_id,
            user_id: dto.user_id,
            username: dto.username,
            moderator_id: dto.moderator_id,
            moderator_name: dto.moderator_name,
            reason: dto.reason,
            saved_roles: dto.saved_roles,
            channel_id: dto.channel_id,
            days,
        })
        .await?;
    Ok(Json(sursis.into()))
}

/// GET /api/moderation/sursis/{id}
pub async fn get_sursis(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SursisDto>, ApiError> {
    let s = state
        .sursis_uc
        .get(id)
        .await?
        .ok_or_else(|| ApiError(DomainError::NotFound("Sursis introuvable".into())))?;
    Ok(Json(s.into()))
}

#[derive(Debug, Deserialize)]
pub struct ResolveSursisDto {
    pub status: String, // gracie | banni
}

/// POST /api/moderation/sursis/{id}/resolve
pub async fn resolve_sursis(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<ResolveSursisDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let status = SursisStatus::from_str_lossy(&dto.status)
        .ok_or_else(|| ApiError(DomainError::ValidationError(format!("statut invalide : {}", dto.status))))?;
    state.sursis_uc.resolve(id, status).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/moderation/internal/jobs/sursis-expire  (worker)
///
/// Bannit definitivement les sursis echus : diffuse `sursis_ban` (le bot ban +
/// nettoie le salon) et marque le sursis 'banni'.
pub async fn job_sursis_expire(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    let due = state.sursis_uc.list_due().await?;
    let processed = due.len();
    for s in &due {
        state.broadcaster.broadcast(
            "sursis_ban",
            serde_json::json!({
                "guild_id": s.guild_id,
                "user_id": s.user_id,
                "username": s.username,
                "reason": s.reason,
                "channel_id": s.channel_id,
            }),
        );
        state.sursis_uc.resolve(s.id, SursisStatus::Banni).await?;
    }
    Ok(Json(JobReport {
        job: "sursis_expire",
        processed,
        errors: 0,
        details: serde_json::json!({}),
    }))
}
