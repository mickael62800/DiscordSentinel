//! Handlers HTTP pour /tout-ou-rien (cf. COUPE_AMELIORATIONS 6.1).
//!
//! - POST /api/coude/{g}/tout-ou-rien/record : log une tentative
//! - GET  /api/coude/{g}/tout-ou-rien/memorial : top N pertes

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::{ToutOuRienLogEntry, ToutOuRienLogOutcome, ToutOuRienUserStats};
use crate::domain::errors::DomainError;

#[derive(Debug, Deserialize)]
pub struct RecordToutOuRienDto {
    pub user_id: String,
    pub username: String,
    pub mise: i64,
    /// "won" | "lost"
    pub outcome: String,
    pub delta: i64,
}

#[derive(Debug, Serialize)]
pub struct MemorialEntryDto {
    pub user_id: String,
    pub username: String,
    pub mise: i64,
    pub outcome: String,
    pub delta: i64,
    pub created_at: DateTime<Utc>,
}

impl From<ToutOuRienLogEntry> for MemorialEntryDto {
    fn from(e: ToutOuRienLogEntry) -> Self {
        Self {
            user_id: e.user_id,
            username: e.username,
            mise: e.mise,
            outcome: e.outcome.as_db_str().into(),
            delta: e.delta,
            created_at: e.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct MemorialQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    10
}

/// POST /api/coude/{guild_id}/tout-ou-rien/record
pub async fn record_tout_ou_rien(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<RecordToutOuRienDto>,
) -> Result<StatusCode, ApiError> {
    let outcome = ToutOuRienLogOutcome::from_db_str(&dto.outcome).ok_or_else(|| {
        ApiError::from(DomainError::ValidationError(format!(
            "Outcome invalide : {} (attendu won|lost)",
            dto.outcome
        )))
    })?;
    state
        .coude_tout_ou_rien_repo
        .record(
            &guild_id,
            &dto.user_id,
            &dto.username,
            dto.mise,
            outcome,
            dto.delta,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/coude/{guild_id}/tout-ou-rien/memorial?limit=N
pub async fn get_memorial(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(q): Query<MemorialQuery>,
) -> Result<Json<Vec<MemorialEntryDto>>, ApiError> {
    let entries = state
        .coude_tout_ou_rien_repo
        .memorial(&guild_id, q.limit)
        .await?;
    Ok(Json(entries.into_iter().map(Into::into).collect()))
}

#[derive(Debug, Serialize)]
pub struct ToutOuRienUserStatsDto {
    pub attempts: i64,
    pub wins: i64,
    pub losses: i64,
    pub biggest_win: i64,
    pub biggest_loss: i64,
}

impl From<ToutOuRienUserStats> for ToutOuRienUserStatsDto {
    fn from(s: ToutOuRienUserStats) -> Self {
        Self {
            attempts: s.attempts,
            wins: s.wins,
            losses: s.losses,
            biggest_win: s.biggest_win,
            biggest_loss: s.biggest_loss,
        }
    }
}

/// GET /api/coude/{guild_id}/tout-ou-rien/by-user/{user_id}
pub async fn get_user_stats(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<ToutOuRienUserStatsDto>, ApiError> {
    let stats = state
        .coude_tout_ou_rien_repo
        .user_stats(&guild_id, &user_id)
        .await?;
    Ok(Json(stats.into()))
}
