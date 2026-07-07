//! Handlers HTTP pour le tournoi hebdomadaire (migration 139).
//!
//! Adaptateur ENTRANT mince : parse + map. L'assemblage du classement, le
//! calcul des rangs et l'estimation du prize pool vivent dans
//! `ManageTournamentsUseCase` ; le SQL d'agregation dans `TournamentRepository`.
//!
//!   - GET /api/coude/{guild_id}/tournaments/current
//!   - GET /api/coude/{guild_id}/tournaments/history

use axum::extract::State;
use axum::Json;
use chrono::DateTime;
use chrono::Utc;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::state::AppState;

use sentinel_core::domain::entities::coude::tournament::CurrentTournament;
use sentinel_core::domain::entities::coude::tournament::PastTournament;
use sentinel_core::domain::entities::coude::tournament::TournamentStanding;
use sentinel_core::domain::entities::system::discord_ids::GuildId;
use sentinel_core::domain::entities::system::discord_ids::UserId;

#[derive(Debug, Serialize)]
pub struct StandingDto {
    pub user_id: UserId,
    pub username: String,
    pub net_gain: i64,
    pub rank: i32,
}

impl From<TournamentStanding> for StandingDto {
    fn from(s: TournamentStanding) -> Self {
        Self {
            user_id: s.user_id.into(),
            username: s.username,
            net_gain: s.net_gain,
            rank: s.rank,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CurrentTournamentDto {
    pub guild_id: GuildId,
    pub week_start: DateTime<Utc>,
    pub week_end: DateTime<Utc>,
    pub prize_pool_estimated: i64,
    pub standings: Vec<StandingDto>,
}

impl From<CurrentTournament> for CurrentTournamentDto {
    fn from(t: CurrentTournament) -> Self {
        Self {
            guild_id: t.guild_id.into(),
            week_start: t.week_start,
            week_end: t.week_end,
            prize_pool_estimated: t.prize_pool_estimated,
            standings: t.standings.into_iter().map(StandingDto::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PastTournamentDto {
    pub id: String,
    pub guild_id: GuildId,
    pub week_start: DateTime<Utc>,
    pub week_end: DateTime<Utc>,
    pub winner_user_id: Option<String>,
    pub winner_username: Option<String>,
    pub winner_net_gain: i64,
    pub prize_amount: i64,
    pub status: String,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl From<PastTournament> for PastTournamentDto {
    fn from(t: PastTournament) -> Self {
        Self {
            id: t.id,
            guild_id: t.guild_id.into(),
            week_start: t.week_start,
            week_end: t.week_end,
            winner_user_id: t.winner_user_id,
            winner_username: t.winner_username,
            winner_net_gain: t.winner_net_gain,
            prize_amount: t.prize_amount,
            status: t.status,
            resolved_at: t.resolved_at,
        }
    }
}

/// GET /api/coude/{guild_id}/tournaments/current
pub async fn get_current_tournament(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<CurrentTournamentDto>, ApiError> {
    let tournament = state.tournaments_uc.current_tournament(&guild_id).await?;
    Ok(Json(tournament.into()))
}

/// GET /api/coude/{guild_id}/tournaments/history
pub async fn get_tournament_history(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<PastTournamentDto>>, ApiError> {
    let history = state.tournaments_uc.tournament_history(&guild_id).await?;
    Ok(Json(
        history.into_iter().map(PastTournamentDto::from).collect(),
    ))
}

#[cfg(test)]
#[path = "tests/tournaments.rs"]
mod tests;
