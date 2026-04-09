use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::{BlackjackGame, Card};

// ── Request DTOs ──

#[derive(Debug, Deserialize)]
pub struct StartGameDto {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub bet: i64,
}

// ── Response DTOs ──

#[derive(Debug, Serialize)]
pub struct CardDto {
    pub rank: String,
    pub suit: String,
    pub filename: String,
}

impl From<&Card> for CardDto {
    fn from(c: &Card) -> Self {
        Self {
            rank: c.rank.clone(),
            suit: c.suit.clone(),
            filename: c.filename(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BlackjackGameDto {
    pub id: String,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub bet: i64,
    pub player_hand: Vec<CardDto>,
    pub dealer_hand: Vec<CardDto>,
    pub status: String,
    pub player_score: i32,
    pub dealer_score: i32,
    pub doubled: bool,
    pub payout: i64,
    pub created_at: String,
    pub finished_at: Option<String>,
}

fn game_is_over(status: &str) -> bool {
    matches!(
        status,
        "player_bust" | "player_win" | "dealer_win" | "dealer_bust" | "push" | "player_blackjack"
    )
}

fn to_dto(game: &BlackjackGame) -> BlackjackGameDto {
    let over = game_is_over(&game.status);

    let dealer_hand: Vec<CardDto> = if over {
        // Partie terminée : révéler toutes les cartes du dealer
        game.dealer_hand.iter().map(CardDto::from).collect()
    } else {
        // Partie en cours : cacher la 2e carte du dealer
        let mut cards: Vec<CardDto> = Vec::new();
        if let Some(first) = game.dealer_hand.first() {
            cards.push(CardDto::from(first));
        }
        if game.dealer_hand.len() > 1 {
            cards.push(CardDto {
                rank: "hidden".to_string(),
                suit: "hidden".to_string(),
                filename: "card_back.jpg".to_string(),
            });
        }
        cards
    };

    let dealer_score = if over {
        game.dealer_score
    } else {
        // Score visible = seulement la première carte
        game.dealer_hand.first().map(|c| c.value()).unwrap_or(0)
    };

    BlackjackGameDto {
        id: game.id.to_string(),
        guild_id: game.guild_id.clone(),
        user_id: game.user_id.clone(),
        username: game.username.clone(),
        bet: game.bet,
        player_hand: game.player_hand.iter().map(CardDto::from).collect(),
        dealer_hand,
        status: game.status.clone(),
        player_score: game.player_score,
        dealer_score,
        doubled: game.doubled,
        payout: game.payout,
        created_at: game.created_at.to_rfc3339(),
        finished_at: game.finished_at.map(|d| d.to_rfc3339()),
    }
}

// ── Handlers ──

/// POST /api/blackjack/start
pub async fn start_game(
    State(state): State<AppState>,
    Json(dto): Json<StartGameDto>,
) -> Result<Json<BlackjackGameDto>, ApiError> {
    // Lire la config depuis bot_guild_config
    let config = state.bot_config_repo.get_config(&dto.guild_id, "blackjack-bot").await.unwrap_or_default();
    let min_bet = config.iter().find(|c| c.config_key == "min_bet").and_then(|c| c.config_value.parse().ok()).unwrap_or(10);
    let max_bet = config.iter().find(|c| c.config_key == "max_bet").and_then(|c| c.config_value.parse().ok()).unwrap_or(1000);
    let starting_coins = config.iter().find(|c| c.config_key == "starting_coins").and_then(|c| c.config_value.parse().ok()).unwrap_or(200);
    let blackjack_payout: f64 = config.iter().find(|c| c.config_key == "blackjack_payout").and_then(|c| c.config_value.parse().ok()).unwrap_or(1.5);

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
        state.broadcaster.broadcast(
            "blackjack_result",
            serde_json::json!({
                "guild_id": game.guild_id,
                "user_id": game.user_id,
                "username": game.username,
                "status": game.status,
                "payout": game.payout,
                "bet": game.bet,
            }),
        );
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
        state.broadcaster.broadcast(
            "blackjack_result",
            serde_json::json!({
                "guild_id": game.guild_id,
                "user_id": game.user_id,
                "username": game.username,
                "status": game.status,
                "payout": game.payout,
                "bet": game.bet,
            }),
        );
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

    state.broadcaster.broadcast(
        "blackjack_result",
        serde_json::json!({
            "guild_id": game.guild_id,
            "user_id": game.user_id,
            "username": game.username,
            "status": game.status,
            "payout": game.payout,
            "bet": game.bet,
        }),
    );

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
        state.broadcaster.broadcast(
            "blackjack_result",
            serde_json::json!({
                "guild_id": game.guild_id,
                "user_id": game.user_id,
                "username": game.username,
                "status": game.status,
                "payout": game.payout,
                "bet": game.bet,
                "doubled": true,
            }),
        );
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

fn parse_uuid(s: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(s).map_err(|_| {
        ApiError::from(crate::domain::errors::DomainError::ValidationError(
            "ID de partie invalide.".into(),
        ))
    })
}
