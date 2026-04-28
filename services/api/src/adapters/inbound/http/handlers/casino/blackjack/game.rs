//! Handlers du jeu solo : start, hit, stand, double, get_active.
//!
//! Tous délèguent à `state.blackjack_svc` et broadcastent un événement
//! `blackjack_result` via le broadcaster quand la partie se termine.

use axum::extract::Path;
use axum::extract::State;
use axum::Json;

use super::dto::game_is_over;
use super::dto::to_dto;
use super::dto::BlackjackGameDto;
use super::dto::StartGameDto;
use super::parse_uuid;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::casino::blackjack::BlackjackGame;

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
    // Lire la config depuis bot_guild_config puis deleguer le parsing +
    // les invariants metier a `BlackjackConfig::from_kv_pairs` (domaine).
    let rows = state
        .bot_config_repo
        .get_config(&dto.guild_id, "blackjack-bot")
        .await
        .unwrap_or_default();
    let pairs: Vec<(String, String)> = rows
        .into_iter()
        .map(|c| (c.config_key, c.config_value))
        .collect();
    let cfg = crate::domain::entities::casino::blackjack::BlackjackConfig::from_kv_pairs(&pairs);

    let result = state
        .blackjack_svc
        .start_game(
            dto.guild_id,
            dto.user_id,
            dto.username,
            dto.bet,
            cfg.min_bet,
            cfg.max_bet,
            cfg.starting_coins,
            cfg.blackjack_payout,
        )
        .await?;
    let game = result.game;

    // Anti-AFK : si le joueur est dans une table multi, bumpe son
    // `last_activity` pour eviter une fermeture par le cleanup worker.
    state
        .blackjack_table_repo
        .touch_activity_by_player(&game.guild_id, &game.user_id)
        .await
        .ok();

    if game_is_over(&game.status) {
        broadcast_result(&state, &game, false);
    }

    // Migration #4 : les `taunt_events` eventuels ne sont PAS renvoyes sur
    // l'API HTTP (usage admin desktop qui n'a pas de pipeline de dispatch).
    // Le bot passe par gRPC qui inclut les taunts dans `BlackjackGameResult`.
    Ok(Json(to_dto(&game)))
}

/// POST /api/blackjack/{game_id}/hit
pub async fn hit(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<Json<BlackjackGameDto>, ApiError> {
    let id = parse_uuid(&game_id)?;
    let game = state.blackjack_svc.hit(id).await?.game;

    // Anti-AFK : bumpe last_activity de la table multi du joueur (si toute).
    state
        .blackjack_table_repo
        .touch_activity_by_player(&game.guild_id, &game.user_id)
        .await
        .ok();

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
    let game = state.blackjack_svc.stand(id).await?.game;

    // Anti-AFK.
    state
        .blackjack_table_repo
        .touch_activity_by_player(&game.guild_id, &game.user_id)
        .await
        .ok();

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
    let game = state.blackjack_svc.double_down(id).await?.game;

    // Anti-AFK.
    state
        .blackjack_table_repo
        .touch_activity_by_player(&game.guild_id, &game.user_id)
        .await
        .ok();

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

/// GET /api/blackjack/admin/{guild_id}/games?status=playing
pub async fn list_games(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    axum::extract::Query(q): axum::extract::Query<ListGamesQuery>,
) -> Result<Json<Vec<BlackjackGameDto>>, ApiError> {
    let games = state.blackjack_svc.list_games(&guild_id, q.status.as_deref()).await?;
    Ok(Json(games.iter().map(to_dto).collect()))
}

/// DELETE /api/blackjack/admin/{guild_id}/purge
/// Vide totalement toutes les tables blackjack pour une guild donnee.
/// Double-check cote frontend obligatoire.
pub async fn purge_all(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use crate::domain::errors::DomainError;

    let games = sqlx::query("DELETE FROM blackjack_games WHERE guild_id = $1")
        .bind(&guild_id)
        .execute(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("purge blackjack_games: {e}"))))?;

    // blackjack_table_players est en CASCADE sur blackjack_tables.
    let tables = sqlx::query("DELETE FROM blackjack_tables WHERE guild_id = $1")
        .bind(&guild_id)
        .execute(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("purge blackjack_tables: {e}"))))?;

    Ok(Json(serde_json::json!({
        "deleted_games": games.rows_affected(),
        "deleted_tables": tables.rows_affected(),
    })))
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
