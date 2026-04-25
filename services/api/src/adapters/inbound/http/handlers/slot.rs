use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::normalize_limit;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use crate::adapters::inbound::http::handlers::coude::taunts::TauntEventDto;
use crate::domain::entities::{SlotSpin, SlotTopWinner};
use crate::ports::inbound::manage_slot::{SpinCommand, SpinResult};

// ── DTOs ──

#[derive(Debug, Deserialize)]
pub struct SpinDto {
    pub user_id: String,
    pub username: String,
    pub mise: i64,
}

#[derive(Debug, Deserialize)]
pub struct DailyDto {
    pub user_id: String,
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct SpinResponseDto {
    pub spin_id: String,
    pub symbols: Vec<String>,
    pub mise: i64,
    pub payout: i64,
    pub multiplier: f64,
    pub is_jackpot: bool,
    pub is_free: bool,
    pub jackpot_pool_after: i64,
    pub balance_after: i64,
    /// Liste des taunts declenches (faillite, jackpot eco). Le bot peut
    /// les rejouer pour annoncer le bankrupt ou le jackpot economique.
    pub triggered_taunts: Vec<TauntEventDto>,
}

impl From<SpinResult> for SpinResponseDto {
    fn from(r: SpinResult) -> Self {
        Self {
            spin_id: r.spin.id.to_string(),
            symbols: r.spin.symbols,
            mise: r.spin.mise,
            payout: r.spin.payout,
            multiplier: r.spin.multiplier,
            is_jackpot: r.spin.is_jackpot,
            is_free: r.spin.is_free,
            jackpot_pool_after: r.jackpot_pool_after,
            balance_after: r.balance_after,
            triggered_taunts: r.triggered_taunts.into_iter().map(TauntEventDto::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SlotSpinDto {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub mise: i64,
    pub symbols: Vec<String>,
    pub payout: i64,
    pub multiplier: f64,
    pub is_jackpot: bool,
    pub is_free: bool,
    pub created_at: String,
}

impl From<SlotSpin> for SlotSpinDto {
    fn from(s: SlotSpin) -> Self {
        Self {
            id: s.id.to_string(),
            user_id: s.user_id,
            username: s.username,
            mise: s.mise,
            symbols: s.symbols,
            payout: s.payout,
            multiplier: s.multiplier,
            is_jackpot: s.is_jackpot,
            is_free: s.is_free,
            created_at: s.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SlotTopWinnerDto {
    pub user_id: String,
    pub username: String,
    pub total_payout: i64,
    pub jackpot_count: u32,
    pub spin_count: u32,
}

impl From<SlotTopWinner> for SlotTopWinnerDto {
    fn from(t: SlotTopWinner) -> Self {
        Self {
            user_id: t.user_id,
            username: t.username,
            total_payout: t.total_payout,
            jackpot_count: t.jackpot_count,
            spin_count: t.spin_count,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct JackpotPoolDto {
    pub current_pool: i64,
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    pub days: Option<i64>,
    pub limit: Option<i64>,
}

// ── Handlers ──

/// POST /api/slot/{guild_id}/spin
pub async fn spin(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<SpinDto>,
) -> Result<Json<SpinResponseDto>, ApiError> {
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &dto.user_id).map_err(ApiError)?;

    let cmd = SpinCommand {
        guild_id,
        user_id: dto.user_id,
        username: dto.username,
        mise: dto.mise,
        is_daily: false,
    };
    let result = state.slot_uc.spin(cmd).await?;

    state.broadcaster.broadcast(
        "slot_spin",
        serde_json::json!({
            "guild_id": &result.spin.guild_id,
            "user_id": &result.spin.user_id,
            "username": &result.spin.username,
            "symbols": &result.spin.symbols,
            "payout": result.spin.payout,
            "is_jackpot": result.spin.is_jackpot,
        }),
    );

    Ok(Json(SpinResponseDto::from(result)))
}

/// POST /api/slot/{guild_id}/daily
pub async fn daily(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<DailyDto>,
) -> Result<Json<SpinResponseDto>, ApiError> {
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &dto.user_id).map_err(ApiError)?;

    let cmd = SpinCommand {
        guild_id,
        user_id: dto.user_id,
        username: dto.username,
        mise: 0, // ignore : la mise daily vient de la config
        is_daily: true,
    };
    let result = state.slot_uc.claim_daily_bonus(cmd).await?;

    state.broadcaster.broadcast(
        "slot_daily",
        serde_json::json!({
            "guild_id": &result.spin.guild_id,
            "user_id": &result.spin.user_id,
            "username": &result.spin.username,
            "payout": result.spin.payout,
        }),
    );

    Ok(Json(SpinResponseDto::from(result)))
}

/// GET /api/slot/{guild_id}/jackpot
pub async fn get_jackpot(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<JackpotPoolDto>, ApiError> {
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    let pool = state.slot_uc.get_jackpot_pool(&guild_id).await?;
    Ok(Json(JackpotPoolDto { current_pool: pool }))
}

/// GET /api/slot/{guild_id}/recent?limit=20
pub async fn recent_spins(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<LimitQuery>,
) -> Result<Json<Vec<SlotSpinDto>>, ApiError> {
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    let limit = normalize_limit(params.limit, 20, 200);
    let spins = state.slot_uc.recent_spins(&guild_id, limit).await?;
    Ok(Json(spins.into_iter().map(SlotSpinDto::from).collect()))
}

/// GET /api/slot/{guild_id}/leaderboard?days=7&limit=10
pub async fn leaderboard(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<LeaderboardQuery>,
) -> Result<Json<Vec<SlotTopWinnerDto>>, ApiError> {
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    let days = params.days.unwrap_or(7).clamp(1, 365);
    let limit = normalize_limit(params.limit, 10, 100);
    let winners = state.slot_uc.top_winners(&guild_id, days, limit).await?;
    Ok(Json(winners.into_iter().map(SlotTopWinnerDto::from).collect()))
}

#[cfg(test)]
#[path = "tests/slot.rs"]
mod tests;
