//! Handlers lois (proposition, vote, cloture worker).

use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use sentinel_core::domain::entities::influence::vote::VoteChoice;
use sentinel_core::domain::errors::DomainError;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::handlers::influence::dto::LawStateDto;
use crate::application::game::worker_jobs::JobReport;
use crate::adapters::inbound::http::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ProposeLawDto {
    pub author_user_id: String,
    #[serde(default)]
    pub author_username: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub effect_param: Option<String>,
    #[serde(default)]
    pub effect_value: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LawVoteDto {
    pub law_id: String,
    pub user_id: String,
    #[serde(default)]
    pub username: String,
    pub choice: String,
}

#[derive(Debug, Deserialize)]
pub struct LawIdDto {
    pub law_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SetLawMessageDto {
    pub law_id: String,
    pub channel_id: String,
    pub message_id: String,
}

/// POST /api/influence/{guild_id}/laws
pub async fn propose_law(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<ProposeLawDto>,
) -> Result<Json<LawStateDto>, ApiError> {
    let st = state
        .influence_laws_uc
        .propose(
            &guild_id,
            &dto.author_user_id,
            &dto.author_username,
            &dto.title,
            &dto.body,
            dto.effect_param.as_deref(),
            dto.effect_value,
        )
        .await?;
    Ok(Json(st.into()))
}

/// POST /api/influence/{guild_id}/laws/vote
pub async fn law_vote(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<LawVoteDto>,
) -> Result<Json<LawStateDto>, ApiError> {
    let choice = VoteChoice::from_str_lossy(&dto.choice).ok_or_else(|| {
        ApiError(DomainError::ValidationError(format!(
            "Choix de vote invalide : {}",
            dto.choice
        )))
    })?;
    let st = state
        .influence_laws_uc
        .vote(&guild_id, &dto.law_id, &dto.user_id, &dto.username, choice)
        .await?;
    Ok(Json(st.into()))
}

/// POST /api/influence/{guild_id}/laws/state
pub async fn law_state(
    State(state): State<AppState>,
    ValidatedGuild { guild_id: _ }: ValidatedGuild,
    Json(dto): Json<LawIdDto>,
) -> Result<Json<LawStateDto>, ApiError> {
    let st = state.influence_laws_uc.get_state(&dto.law_id).await?;
    Ok(Json(st.into()))
}

/// POST /api/influence/{guild_id}/laws/message
pub async fn set_law_message(
    State(state): State<AppState>,
    ValidatedGuild { guild_id: _ }: ValidatedGuild,
    Json(dto): Json<SetLawMessageDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .influence_laws_uc
        .set_message(&dto.law_id, &dto.channel_id, &dto.message_id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/influence/internal/jobs/close-laws  (worker)
///
/// Cloture les lois arrivees a echeance et diffuse `influence_law_closed` pour
/// que le bot edite le message et annonce le resultat.
pub async fn job_close_laws(
    State(state): State<AppState>,
) -> Result<Json<JobReport>, ApiError> {
    let closed = state.influence_laws_uc.close_due().await?;
    let mut processed = 0usize;
    for st in &closed {
        if let Some(mid) = &st.law.message_id {
            state.broadcaster.broadcast(
                "influence_law_closed",
                serde_json::json!({
                    "guild_id": st.law.guild_id,
                    "law_id": st.law.id.to_string(),
                    "channel_id": st.law.channel_id,
                    "message_id": mid,
                }),
            );
        }
        processed += 1;
    }
    Ok(Json(JobReport {
        job: "close_laws",
        processed,
        errors: 0,
        details: serde_json::json!({}),
    }))
}
