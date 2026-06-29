use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::handlers::coude::taunts::TauntEventDto;
use crate::adapters::inbound::http::helpers::normalize_limit;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use crate::ports::inbound::casino::manage_wheel::WheelSpinCommand;
use crate::ports::inbound::casino::manage_wheel::WheelSpinResult;
use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use sentinel_core::domain::entities::casino::wheel::WheelSpin;
use sentinel_core::domain::entities::casino::wheel::WheelTopWinner;
use sentinel_core::domain::entities::system::discord_ids::UserId;
use serde::Deserialize;
use serde::Serialize;
// ── DTOs ──

#[derive(Debug, Deserialize)]
pub struct WheelSpinDto {
    pub user_id: UserId,
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct WheelSpinResponseDto {
    pub spin_id: String,
    pub case_key: String,
    pub case_label: String,
    pub payout: i64,
    pub balance_after: i64,
    pub is_memorable: bool,
    pub triggered_taunts: Vec<TauntEventDto>,
}

impl From<WheelSpinResult> for WheelSpinResponseDto {
    fn from(r: WheelSpinResult) -> Self {
        Self {
            spin_id: r.spin.id.to_string(),
            case_key: r.case.key.to_string(),
            case_label: r.case.label.to_string(),
            payout: r.case.payout,
            balance_after: r.balance_after,
            is_memorable: r.is_memorable,
            triggered_taunts: r
                .triggered_taunts
                .into_iter()
                .map(TauntEventDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WheelSpinLogDto {
    pub id: String,
    pub user_id: UserId,
    pub username: String,
    pub case_key: String,
    pub case_label: String,
    pub payout: i64,
    pub created_at: String,
}

impl From<WheelSpin> for WheelSpinLogDto {
    fn from(s: WheelSpin) -> Self {
        Self {
            id: s.id.to_string(),
            user_id: s.user_id,
            username: s.username,
            case_key: s.case_key,
            case_label: s.case_label,
            payout: s.payout,
            created_at: s.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WheelTopWinnerDto {
    pub user_id: UserId,
    pub username: String,
    pub total_payout: i64,
    pub spin_count: u32,
}

impl From<WheelTopWinner> for WheelTopWinnerDto {
    fn from(t: WheelTopWinner) -> Self {
        Self {
            user_id: t.user_id,
            username: t.username,
            total_payout: t.total_payout,
            spin_count: t.spin_count,
        }
    }
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

pub async fn spin(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<WheelSpinDto>,
) -> Result<Json<WheelSpinResponseDto>, ApiError> {
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &dto.user_id).map_err(ApiError)?;

    let cmd = WheelSpinCommand {
        guild_id: guild_id.into(),
        user_id: dto.user_id,
        username: dto.username,
    };
    let result = state.wheel_uc.spin(cmd).await?;

    state.broadcaster.broadcast(
        "wheel_spin",
        serde_json::json!({
            "guild_id": &result.spin.guild_id,
            "user_id": &result.spin.user_id,
            "username": &result.spin.username,
            "case_key": &result.case.key,
            "payout": result.case.payout,
            "is_memorable": result.is_memorable,
        }),
    );

    Ok(Json(WheelSpinResponseDto::from(result)))
}

pub async fn recent(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<LimitQuery>,
) -> Result<Json<Vec<WheelSpinLogDto>>, ApiError> {
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    let limit = normalize_limit(params.limit, 20, 200);
    let spins = state.wheel_uc.recent_spins(&guild_id, limit).await?;
    Ok(Json(spins.into_iter().map(WheelSpinLogDto::from).collect()))
}

pub async fn leaderboard(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<LeaderboardQuery>,
) -> Result<Json<Vec<WheelTopWinnerDto>>, ApiError> {
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    let days = params.days.unwrap_or(7).clamp(1, 365);
    let limit = normalize_limit(params.limit, 10, 100);
    let winners = state.wheel_uc.top_winners(&guild_id, days, limit).await?;
    Ok(Json(
        winners.into_iter().map(WheelTopWinnerDto::from).collect(),
    ))
}

#[cfg(test)]
#[path = "tests/wheel.rs"]
mod tests;
