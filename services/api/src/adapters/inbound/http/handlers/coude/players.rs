//! Handlers joueurs : CRUD, progression (XP/level/stats), stats de combat,
//! coins et HP. Tous délèguent à `state.coude_players_uc`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;

use super::dto::{
    AddXpDto, AddXpResponse, AdjustCoinsDto, AmountDto, FullPlayerDto, GetOrCreatePlayerDto,
    PlayerDto, RandomPlayersQuery, RecordDrawDto, RecordLossDto, RecordWinDto, ResetStatsDto,
    SpendStatDto, UpdateClassDto, UpdateHpDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::CombatStat;
use crate::domain::errors::DomainError;

// ── Listing & utilitaires ──

/// GET /api/coude/{guild_id}/players — liste des joueurs
pub async fn list_players(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<PlayerDto>>, ApiError> {
    let players = state.coude_players_uc.list(&guild_id).await?;
    Ok(Json(players.iter().map(PlayerDto::from).collect()))
}

/// GET /api/coude/guilds — liste distincte des guild_id ayant au moins un joueur
pub async fn get_all_guild_ids(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, ApiError> {
    let guild_ids = state.coude_players_uc.list_guild_ids().await?;
    Ok(Json(guild_ids))
}

/// GET /api/coude/{guild_id}/players/random?count=2
pub async fn get_random_players(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<RandomPlayersQuery>,
) -> Result<Json<Vec<FullPlayerDto>>, ApiError> {
    let count = params.count.unwrap_or(2);
    let players = state
        .coude_players_uc
        .random_active(&guild_id, count)
        .await?;
    Ok(Json(players.into_iter().map(FullPlayerDto::from).collect()))
}

// ── CRUD ──

/// POST /api/coude/{guild_id}/players/get-or-create
pub async fn get_or_create_player(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<GetOrCreatePlayerDto>,
) -> Result<Json<FullPlayerDto>, ApiError> {
    let player = state
        .coude_players_uc
        .get_or_create(guild_id, dto.user_id, dto.username)
        .await?;
    Ok(Json(player.into()))
}

/// GET /api/coude/{guild_id}/players/{user_id}
pub async fn get_player(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<FullPlayerDto>, ApiError> {
    let player = state.coude_players_uc.get(&guild_id, &user_id).await?;
    Ok(Json(player.into()))
}

/// PATCH /api/coude/{guild_id}/players/{user_id}/class
pub async fn update_player_class(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<UpdateClassDto>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_players_uc
        .update_class(&guild_id, &user_id, &dto.class)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Progression ──

/// POST /api/coude/{guild_id}/players/{user_id}/xp
pub async fn add_xp(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AddXpDto>,
) -> Result<Json<AddXpResponse>, ApiError> {
    let progress = state
        .coude_players_uc
        .add_xp(&guild_id, &user_id, dto.amount)
        .await?;
    Ok(Json(progress.into()))
}

/// POST /api/coude/{guild_id}/players/{user_id}/spend-stat
pub async fn spend_stat_point(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<SpendStatDto>,
) -> Result<Json<FullPlayerDto>, ApiError> {
    let stat = CombatStat::parse(&dto.stat).ok_or_else(|| {
        ApiError::from(DomainError::ValidationError(
            "Stat invalide, doit etre 'atk' ou 'def'".into(),
        ))
    })?;
    let player = state
        .coude_players_uc
        .spend_stat_point(&guild_id, &user_id, stat)
        .await?;
    Ok(Json(player.into()))
}

/// POST /api/coude/{guild_id}/players/{user_id}/reset-stats
///
/// Reset atomique : remet ATK/DEF à 0, restitue les points dans `stat_points`
/// et déduit le coût (en coins).
pub async fn reset_stats(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<ResetStatsDto>,
) -> Result<Json<FullPlayerDto>, ApiError> {
    let player = state
        .coude_players_uc
        .reset_stats(&guild_id, &user_id, dto.cost)
        .await?;
    Ok(Json(player.into()))
}

// ── Stats recording ──

/// POST /api/coude/{guild_id}/players/{user_id}/record-win
pub async fn record_win(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<RecordWinDto>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_players_uc
        .record_win(&guild_id, &user_id, dto.earned, dto.stolen)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/record-loss
pub async fn record_loss(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<RecordLossDto>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_players_uc
        .record_loss(&guild_id, &user_id, dto.lost)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/record-draw
pub async fn record_draw(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<RecordDrawDto>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_players_uc
        .record_draw(&guild_id, &user_id, dto.lost)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/increment-cowardice
pub async fn increment_cowardice(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let count = state
        .coude_players_uc
        .increment_cowardice(&guild_id, &user_id)
        .await?;
    Ok(Json(serde_json::json!({ "cowardice_count": count })))
}

/// POST /api/coude/{guild_id}/players/{user_id}/increment-chaos
pub async fn increment_chaos(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_players_uc
        .increment_chaos(&guild_id, &user_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Coins ──

/// PATCH /api/coude/players/{guild_id}/{user_id}/coins — ajouter ou retirer des coins
pub async fn adjust_coins(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AdjustCoinsDto>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_players_uc
        .adjust_coins(&guild_id, &user_id, dto.amount)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/coins-earned
pub async fn record_coins_earned(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AmountDto>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_players_uc
        .record_coins_earned(&guild_id, &user_id, dto.amount)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/coins-lost
pub async fn record_coins_lost(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AmountDto>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_players_uc
        .record_coins_lost(&guild_id, &user_id, dto.amount)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── HP ──

/// POST /api/coude/{guild_id}/players/{user_id}/hp
pub async fn update_hp(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<UpdateHpDto>,
) -> Result<StatusCode, ApiError> {
    state
        .coude_players_uc
        .update_hp(&guild_id, &user_id, dto.hp_current, dto.hp_max)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/repos — soin complet (full heal)
pub async fn repos(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    state.coude_players_uc.full_heal(&guild_id, &user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
