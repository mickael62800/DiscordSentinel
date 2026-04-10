//! Handlers sociaux : cooldowns, leaderboard, événements, daily chaos, saisons.
//! Délèguent à `state.coude_social_uc`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;

use super::dto::{
    CurrentSeasonDto, DailyChaosDto, DurationDto, EventDto, LeaderboardEntry,
    LeaderboardQueryParams,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::{LeaderboardCategory, NewDailyChaos};
use crate::domain::errors::DomainError;

// ── Cooldowns ──

/// GET /api/coude/{guild_id}/cooldown/{user_id}/{action}
pub async fn check_cooldown(
    State(state): State<AppState>,
    Path((guild_id, user_id, action)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let expires_at = state
        .coude_social_uc
        .check_cooldown(&guild_id, &user_id, &action)
        .await?
        .map(|dt| dt.to_rfc3339());
    Ok(Json(serde_json::json!({ "expires_at": expires_at })))
}

/// POST /api/coude/{guild_id}/cooldown/{user_id}/{action}
pub async fn set_cooldown(
    State(state): State<AppState>,
    Path((guild_id, user_id, action)): Path<(String, String, String)>,
    Json(dto): Json<DurationDto>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_social_uc
        .set_cooldown(&guild_id, &user_id, &action, dto.duration_secs)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Leaderboard ──

/// GET /api/coude/{guild_id}/leaderboard/{category}
pub async fn leaderboard(
    State(state): State<AppState>,
    Path((guild_id, category)): Path<(String, String)>,
    Query(params): Query<LeaderboardQueryParams>,
) -> Result<Json<Vec<LeaderboardEntry>>, ApiError> {
    let cat = LeaderboardCategory::parse(&category).ok_or_else(|| {
        ApiError::from(DomainError::ValidationError(format!(
            "Categorie invalide: {}. Valeurs acceptees: richest, thieves, cowards, chaos, level",
            category
        )))
    })?;
    let entries = state
        .coude_social_uc
        .leaderboard(&guild_id, cat, params.limit.unwrap_or(10))
        .await?;
    Ok(Json(entries.into_iter().map(LeaderboardEntry::from).collect()))
}

// ── Daily chaos ──

/// POST /api/coude/{guild_id}/daily-chaos
pub async fn log_daily_chaos(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<DailyChaosDto>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_social_uc
        .log_daily_chaos(NewDailyChaos {
            guild_id,
            loser_id: dto.loser_id,
            loser_name: dto.loser_name,
            winner_id: dto.winner_id,
            winner_name: dto.winner_name,
            amount: dto.amount,
        })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Événements serveur ──

/// GET /api/coude/{guild_id}/events
pub async fn get_active_events(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<EventDto>>, ApiError> {
    let events = state.coude_social_uc.list_active_events(&guild_id).await?;
    Ok(Json(events.into_iter().map(EventDto::from).collect()))
}

// ── Saison ──

/// GET /api/coude/{guild_id}/season/current
///
/// Retourne la saison active du serveur. Bootstrap automatique : si aucune
/// saison n'existe, une nouvelle est créée avec un numéro incrémenté.
/// Une saison dure 90 jours.
pub async fn get_current_season(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<CurrentSeasonDto>, ApiError> {
    let season = state.coude_social_uc.current_season(&guild_id).await?;
    Ok(Json(season.into()))
}
