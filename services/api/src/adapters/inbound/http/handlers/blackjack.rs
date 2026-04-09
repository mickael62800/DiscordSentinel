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

// ══════════════════════════════════════════════════════════
//  Multiplayer — Tables
// ══════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct CreateTableDto {
    pub guild_id: String,
    pub channel_id: String,
    pub owner_id: String,
    pub owner_name: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TableDto {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TablePlayerDto {
    pub user_id: String,
    pub user_name: String,
    pub joined_at: String,
}

#[derive(Debug, Deserialize)]
pub struct JoinTableDto {
    pub user_id: String,
    pub user_name: String,
}

/// POST /api/blackjack/tables — creer une table multijoueur avec sabot de 6 decks
pub async fn create_table(
    State(state): State<AppState>,
    Json(dto): Json<CreateTableDto>,
) -> Result<Json<TableDto>, ApiError> {
    // Creer un sabot de 6 decks melanges (312 cartes)
    use crate::domain::entities::create_deck;
    let mut shoe: Vec<crate::domain::entities::Card> = Vec::with_capacity(312);
    for _ in 0..6 {
        shoe.extend(create_deck());
    }
    // Melanger le sabot entier
    use rand::seq::SliceRandom;
    shoe.shuffle(&mut rand::thread_rng());

    let shoe_json = serde_json::to_value(&shoe)
        .map_err(|e| ApiError::from(crate::domain::errors::DomainError::Internal(e.to_string())))?;

    let table = sqlx::query_as::<_, TableDto>(
        r#"INSERT INTO blackjack_tables (guild_id, channel_id, owner_id, owner_name, deck)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id::text, guild_id, channel_id, owner_id, owner_name, status, created_at::text"#,
    )
    .bind(&dto.guild_id)
    .bind(&dto.channel_id)
    .bind(&dto.owner_id)
    .bind(&dto.owner_name)
    .bind(&shoe_json)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(crate::domain::errors::DomainError::Internal(e.to_string())))?;

    // Le owner est automatiquement joueur
    sqlx::query(
        "INSERT INTO blackjack_table_players (table_id, user_id, user_name) VALUES ($1::uuid, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(&table.id)
    .bind(&dto.owner_id)
    .bind(&dto.owner_name)
    .execute(&state.pg_pool)
    .await
    .ok();

    Ok(Json(table))
}

/// POST /api/blackjack/tables/{table_id}/join — rejoindre une table
pub async fn join_table(
    State(state): State<AppState>,
    Path(table_id): Path<String>,
    Json(dto): Json<JoinTableDto>,
) -> Result<axum::http::StatusCode, ApiError> {
    // Verifier que la table est ouverte + recuperer le guild_id
    let table_info = sqlx::query_as::<_, (String, String)>(
        "SELECT status, guild_id FROM blackjack_tables WHERE id = $1::uuid",
    )
    .bind(&table_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(crate::domain::errors::DomainError::Internal(e.to_string())))?;

    match &table_info {
        Some((s, _)) if s == "open" => {}
        Some(_) => return Err(crate::domain::errors::DomainError::Conflict("Table fermee".into()).into()),
        None => return Err(crate::domain::errors::DomainError::NotFound("Table introuvable".into()).into()),
    }

    let guild_id = table_info.unwrap().1;

    // Verifier la limite de joueurs
    let current_count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM blackjack_table_players WHERE table_id = $1::uuid",
    )
    .bind(&table_id)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(crate::domain::errors::DomainError::Internal(e.to_string())))?
    .0;

    let max_players: i64 = {
        let config = state.bot_config_repo.get_config(&guild_id, "blackjack-bot").await.unwrap_or_default();
        config.iter()
            .find(|c| c.config_key == "max_players_per_table")
            .and_then(|c| c.config_value.parse().ok())
            .unwrap_or(7)
    };

    if current_count >= max_players {
        return Err(crate::domain::errors::DomainError::ValidationError(
            format!("Table pleine ({}/{} joueurs)", current_count, max_players),
        ).into());
    }

    sqlx::query(
        "INSERT INTO blackjack_table_players (table_id, user_id, user_name) VALUES ($1::uuid, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(&table_id)
    .bind(&dto.user_id)
    .bind(&dto.user_name)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(crate::domain::errors::DomainError::Internal(e.to_string())))?;

    // Mettre a jour last_activity
    sqlx::query("UPDATE blackjack_tables SET last_activity = NOW() WHERE id = $1::uuid")
        .bind(&table_id)
        .execute(&state.pg_pool)
        .await
        .ok();

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// GET /api/blackjack/tables/{table_id}/players — lister les joueurs d'une table
pub async fn list_table_players(
    State(state): State<AppState>,
    Path(table_id): Path<String>,
) -> Result<Json<Vec<TablePlayerDto>>, ApiError> {
    let players = sqlx::query_as::<_, TablePlayerDto>(
        "SELECT user_id, user_name, joined_at::text FROM blackjack_table_players WHERE table_id = $1::uuid ORDER BY joined_at",
    )
    .bind(&table_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(crate::domain::errors::DomainError::Internal(e.to_string())))?;

    Ok(Json(players))
}

/// GET /api/blackjack/tables/by-channel/{channel_id} — trouver la table par channel
pub async fn get_table_by_channel(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
) -> Result<Json<Option<TableDto>>, ApiError> {
    let table = sqlx::query_as::<_, TableDto>(
        r#"SELECT id::text, guild_id, channel_id, owner_id, owner_name, status, created_at::text
           FROM blackjack_tables WHERE channel_id = $1 AND status = 'open'"#,
    )
    .bind(&channel_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(crate::domain::errors::DomainError::Internal(e.to_string())))?;

    Ok(Json(table))
}

/// DELETE /api/blackjack/tables/{table_id} — fermer une table
pub async fn close_table(
    State(state): State<AppState>,
    Path(table_id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    sqlx::query(
        "UPDATE blackjack_tables SET status = 'closed' WHERE id = $1::uuid AND status = 'open'",
    )
    .bind(&table_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(crate::domain::errors::DomainError::Internal(e.to_string())))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// GET /api/blackjack/tables/{table_id}/games — parties d'une table (resume)
pub async fn list_table_games(
    State(state): State<AppState>,
    Path(table_id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, i64)>(
        r#"SELECT id::text, user_id, username, status, bet, payout
           FROM blackjack_games WHERE table_id = $1::uuid ORDER BY created_at DESC"#,
    )
    .bind(&table_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(crate::domain::errors::DomainError::Internal(e.to_string())))?;

    let result: Vec<serde_json::Value> = rows.iter().map(|(id, uid, name, status, bet, payout)| {
        serde_json::json!({
            "id": id, "user_id": uid, "username": name,
            "status": status, "bet": bet, "payout": payout,
        })
    }).collect();

    Ok(Json(result))
}
