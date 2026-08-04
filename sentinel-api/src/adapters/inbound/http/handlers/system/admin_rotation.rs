//! Handlers HTTP de l'administrateur tournant (etat + historique).
//! Persistance uniquement : l'orchestration Discord est cote bot.

use axum::Json;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::entities::system::admin_rotation::RotationState;

/// Cle RBAC partagee avec le bouton dashboard (rotation.dashboard). Les
/// appels internes (bot, sans X-Discord-Token) passent ; les utilisateurs
/// web doivent avoir le role minimal configure.
const RBAC_KEY: &str = "rotation.dashboard";

fn parse_dt(s: &Option<String>) -> Option<chrono::DateTime<chrono::Utc>> {
    s.as_deref()
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
        .map(|d| d.with_timezone(&chrono::Utc))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RotationStateDto {
    pub guild_id: String,
    pub state: String,
    pub current_admin_id: Option<String>,
    pub current_admin_since: Option<String>,
    pub period_start: Option<String>,
    pub next_rotation_at: Option<String>,
    pub candidate_id: Option<String>,
    pub candidate_offered_at: Option<String>,
    #[serde(default)]
    pub asked_this_round: Vec<String>,
}

impl From<RotationState> for RotationStateDto {
    fn from(s: RotationState) -> Self {
        Self {
            guild_id: s.guild_id,
            state: s.state,
            current_admin_id: s.current_admin_id,
            current_admin_since: s.current_admin_since.map(|d| d.to_rfc3339()),
            period_start: s.period_start.map(|d| d.to_rfc3339()),
            next_rotation_at: s.next_rotation_at.map(|d| d.to_rfc3339()),
            candidate_id: s.candidate_id,
            candidate_offered_at: s.candidate_offered_at.map(|d| d.to_rfc3339()),
            asked_this_round: s.asked_this_round,
        }
    }
}

impl RotationStateDto {
    fn into_domain(self) -> RotationState {
        RotationState {
            current_admin_since: parse_dt(&self.current_admin_since),
            period_start: parse_dt(&self.period_start),
            next_rotation_at: parse_dt(&self.next_rotation_at),
            candidate_offered_at: parse_dt(&self.candidate_offered_at),
            guild_id: self.guild_id,
            state: self.state,
            current_admin_id: self.current_admin_id,
            candidate_id: self.candidate_id,
            asked_this_round: self.asked_this_round,
        }
    }
}

/// GET /api/rotation/{guild_id}
pub async fn get_state(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<RotationStateDto>, ApiError> {
    let s = state.rotation_uc.get_state(&guild_id).await?;
    Ok(Json(s.into()))
}

/// PUT /api/rotation/{guild_id}
pub async fn save_state(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(mut body): Json<RotationStateDto>,
) -> Result<Json<RotationStateDto>, ApiError> {
    body.guild_id = guild_id;
    let domain = body.into_domain();
    state.rotation_uc.save_state(domain.clone()).await?;
    Ok(Json(domain.into()))
}

#[derive(Debug, Deserialize)]
pub struct ServedBody {
    pub user_id: String,
}

/// POST /api/rotation/{guild_id}/served
pub async fn record_served(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(body): Json<ServedBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .rotation_uc
        .record_served(&guild_id, &body.user_id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Serialize)]
pub struct ServedEntryDto {
    pub user_id: String,
    pub served_at: String,
}

/// GET /api/rotation/{guild_id}/history
pub async fn history(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<ServedEntryDto>>, ApiError> {
    let entries = state.rotation_uc.served_entries(&guild_id).await?;
    Ok(Json(
        entries
            .into_iter()
            .map(|e| ServedEntryDto {
                user_id: e.user_id,
                served_at: e.served_at.to_rfc3339(),
            })
            .collect(),
    ))
}
