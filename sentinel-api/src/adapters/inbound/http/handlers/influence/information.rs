//! Handlers information (enquetes, intel, revelation, job worker).

use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::handlers::influence::dto::{
    InformationDto, InvestigationDto, RevealOutcomeDto,
};
use crate::application::game::worker_jobs::JobReport;
use crate::adapters::inbound::http::state::AppState;

#[derive(Debug, Deserialize)]
pub struct OpenInvestigationDto {
    pub initiator_user_id: String,
    #[serde(default)]
    pub initiator_username: String,
    pub target_user_id: String,
    #[serde(default)]
    pub target_username: String,
    pub subject: String,
}

#[derive(Debug, Deserialize)]
pub struct UserDto {
    pub user_id: String,
    #[serde(default)]
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct RevealDto {
    pub user_id: String,
    #[serde(default)]
    pub username: String,
    pub info_id: String,
}

/// POST /api/influence/{guild_id}/investigations
pub async fn open_investigation(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<OpenInvestigationDto>,
) -> Result<Json<InvestigationDto>, ApiError> {
    let inv = state
        .influence_information_uc
        .open_investigation(
            &guild_id,
            &dto.initiator_user_id,
            &dto.initiator_username,
            &dto.target_user_id,
            &dto.target_username,
            &dto.subject,
        )
        .await?;
    Ok(Json(inv.into()))
}

/// POST /api/influence/{guild_id}/intel
pub async fn list_intel(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<UserDto>,
) -> Result<Json<Vec<InformationDto>>, ApiError> {
    let list = state
        .influence_information_uc
        .list_intel(&guild_id, &dto.user_id, &dto.username)
        .await?;
    Ok(Json(list.into_iter().map(Into::into).collect()))
}

/// POST /api/influence/{guild_id}/reveal
pub async fn reveal(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<RevealDto>,
) -> Result<Json<RevealOutcomeDto>, ApiError> {
    let outcome = state
        .influence_information_uc
        .reveal(&guild_id, &dto.user_id, &dto.username, &dto.info_id)
        .await?;
    Ok(Json(outcome.into()))
}

/// POST /api/influence/internal/jobs/resolve-investigations  (worker)
pub async fn job_resolve_investigations(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    let resolved = state.influence_information_uc.resolve_due().await?;
    let processed = resolved.len();
    for r in &resolved {
        state.broadcaster.broadcast(
            "influence_investigation_done",
            serde_json::json!({
                "initiator_user_id": r.initiator_user_id,
                "target_username": r.target_username,
                "subject": r.subject,
                "success": r.success,
            }),
        );
    }
    Ok(Json(JobReport {
        job: "resolve_investigations",
        processed,
        errors: 0,
        details: serde_json::json!({}),
    }))
}
