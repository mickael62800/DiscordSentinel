//! Handlers du jeu solo : start, hit, stand, double, get_active.
//!
//! Tous délèguent à `state.blackjack_svc` et broadcastent un événement
//! `blackjack_result` via le broadcaster quand la partie se termine.

use axum::extract::{Path, State};
use axum::Json;

use super::dto::{game_is_over, to_dto, BlackjackGameDto, StartGameDto};
use super::parse_uuid;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::BlackjackGame;

/// Diffuse un événement `blackjack_result` pour la partie terminée.
fn broadcast_result(state: &AppState, game: &BlackjackGame, doubled: bool) {
    let mut payload = serde_json::json!({
        "guild_id": game.guild_id,
        "user_id": game.user_id,
        "username": game.username,
        "status": game.status,
        "payout": game.payout,
        "bet": game.bet,
    });
    if doubled {
        payload["doubled"] = serde_json::Value::Bool(true);
    }
    state.broadcaster.broadcast("blackjack_result", payload);
}

/// POST /api/blackjack/start
pub async fn start_game(
    State(state): State<AppState>,
    Json(dto): Json<StartGameDto>,
) -> Result<Json<BlackjackGameDto>, ApiError> {
    // Lire la config depuis bot_guild_config
    let config = state
        .bot_config_repo
        .get_config(&dto.guild_id, "blackjack-bot")
        .await
        .unwrap_or_default();
    let min_bet = config
        .iter()
        .find(|c| c.config_key == "min_bet")
        .and_then(|c| c.config_value.parse().ok())
        .unwrap_or(10);
    let max_bet = config
        .iter()
        .find(|c| c.config_key == "max_bet")
        .and_then(|c| c.config_value.parse().ok())
        .unwrap_or(1000);
    let starting_coins = config
        .iter()
        .find(|c| c.config_key == "starting_coins")
        .and_then(|c| c.config_value.parse().ok())
        .unwrap_or(200);
    let blackjack_payout: f64 = config
        .iter()
        .find(|c| c.config_key == "blackjack_payout")
        .and_then(|c| c.config_value.parse().ok())
        .unwrap_or(1.5);

    let game = state
        .blackjack_svc
        .start_game(
            dto.guild_id,
            dto.user_id,
            dto.username,
            dto.bet,
            min_bet,
            max_bet,
            starting_coins,
            blackjack_payout,
        )
        .await?;

    if game_is_over(&game.status) {
        broadcast_result(&state, &game, false);
    }

    Ok(Json(to_dto(&game)))
}

/// POST /api/blackjack/{game_id}/hit
pub async fn hit(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<Json<BlackjackGameDto>, ApiError> {
    let id = parse_uuid(&game_id)?;
    let game = state.blackjack_svc.hit(id).await?;

    if game_is_over(&game.status) {
        broadcast_result(&state, &game, false);
    }

    Ok(Json(to_dto(&game)))
}

/// POST /api/blackjack/{game_id}/stand
pub async fn stand(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<Json<BlackjackGameDto>, ApiError> {
    let id = parse_uuid(&game_id)?;
    let game = state.blackjack_svc.stand(id).await?;

    // stand termine toujours la partie → on broadcast systématiquement.
    broadcast_result(&state, &game, false);

    Ok(Json(to_dto(&game)))
}

/// POST /api/blackjack/{game_id}/double
pub async fn double_down(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<Json<BlackjackGameDto>, ApiError> {
    let id = parse_uuid(&game_id)?;
    let game = state.blackjack_svc.double_down(id).await?;

    if game_is_over(&game.status) {
        broadcast_result(&state, &game, true);
    }

    Ok(Json(to_dto(&game)))
}

/// GET /api/blackjack/{guild_id}/{user_id}/active
pub async fn get_active(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Option<BlackjackGameDto>>, ApiError> {
    let game = state.blackjack_svc.get_active(&guild_id, &user_id).await?;
    Ok(Json(game.as_ref().map(to_dto)))
}

#[derive(serde::Deserialize)]
pub struct ListGamesQuery {
    pub status: Option<String>,
}

/// GET /api/blackjack/{guild_id}/games?status=in_progress
pub async fn list_games(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    axum::extract::Query(q): axum::extract::Query<ListGamesQuery>,
) -> Result<Json<Vec<BlackjackGameDto>>, ApiError> {
    let games = state.blackjack_svc.list_games(&guild_id, q.status.as_deref()).await?;
    Ok(Json(games.iter().map(to_dto).collect()))
}

/// DELETE /api/blackjack/games/{game_id}
pub async fn cancel_game(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let id = parse_uuid(&game_id)?;
    state.blackjack_svc.cancel_game(id).await?;
    state.broadcaster.broadcast(
        "blackjack_cancelled",
        serde_json::json!({ "game_id": game_id }),
    );
    Ok(axum::http::StatusCode::NO_CONTENT)
}
