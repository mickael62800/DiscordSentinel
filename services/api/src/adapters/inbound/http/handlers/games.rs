use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;

// ── DTOs ──

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct GameDto {
    pub id: String,
    pub guild_id: String,
    pub game_name: String,
    pub created_by: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateGameDto {
    pub guild_id: String,
    pub game_name: String,
    pub created_by: String,
}

#[derive(Debug, Deserialize)]
pub struct SubscribeDto {
    pub user_id: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SubscriberDto {
    pub user_id: String,
}

// ── Games CRUD ──

/// GET /api/games/{guild_id}
pub async fn list_games(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<GameDto>>, ApiError> {
    let games = sqlx::query_as::<_, GameDto>(
        r#"SELECT id::text, guild_id, game_name, created_by, created_at::text
           FROM games WHERE guild_id = $1 ORDER BY game_name"#,
    )
    .bind(&guild_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(games))
}

/// POST /api/games
pub async fn create_game(
    State(state): State<AppState>,
    Json(dto): Json<CreateGameDto>,
) -> Result<Json<GameDto>, ApiError> {
    let name = dto.game_name.trim().to_string();
    if name.is_empty() {
        return Err(DomainError::ValidationError("Le nom du jeu ne peut pas etre vide".into()).into());
    }
    if name.len() > 100 {
        return Err(DomainError::ValidationError("Le nom du jeu ne peut pas depasser 100 caracteres".into()).into());
    }

    let game = sqlx::query_as::<_, GameDto>(
        r#"INSERT INTO games (guild_id, game_name, created_by)
           VALUES ($1, $2, $3)
           RETURNING id::text, guild_id, game_name, created_by, created_at::text"#,
    )
    .bind(&dto.guild_id)
    .bind(&name)
    .bind(&dto.created_by)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("idx_games_guild_name") {
            ApiError::from(DomainError::Conflict("Un jeu avec ce nom existe deja".into()))
        } else {
            ApiError::from(DomainError::Internal(e.to_string()))
        }
    })?;

    Ok(Json(game))
}

/// DELETE /api/games/{guild_id}/{game_id}
pub async fn delete_game(
    State(state): State<AppState>,
    Path((guild_id, game_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query("DELETE FROM games WHERE guild_id = $1 AND id = $2::uuid")
        .bind(&guild_id)
        .bind(&game_id)
        .execute(&state.pg_pool)
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Jeu introuvable".into()).into());
    }

    Ok(StatusCode::NO_CONTENT)
}

// ── Subscriptions ──

/// POST /api/games/{guild_id}/{game_id}/subscribe
pub async fn subscribe(
    State(state): State<AppState>,
    Path((guild_id, game_id)): Path<(String, String)>,
    Json(dto): Json<SubscribeDto>,
) -> Result<StatusCode, ApiError> {
    sqlx::query(
        r#"INSERT INTO game_subscriptions (guild_id, game_id, user_id)
           VALUES ($1, $2::uuid, $3)
           ON CONFLICT (game_id, user_id) DO NOTHING"#,
    )
    .bind(&guild_id)
    .bind(&game_id)
    .bind(&dto.user_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/games/{guild_id}/{game_id}/subscribe/{user_id}
pub async fn unsubscribe(
    State(state): State<AppState>,
    Path((guild_id, game_id, user_id)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    sqlx::query(
        "DELETE FROM game_subscriptions WHERE guild_id = $1 AND game_id = $2::uuid AND user_id = $3",
    )
    .bind(&guild_id)
    .bind(&game_id)
    .bind(&user_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/games/{guild_id}/{game_id}/subscribers
pub async fn get_subscribers(
    State(state): State<AppState>,
    Path((_guild_id, game_id)): Path<(String, String)>,
) -> Result<Json<Vec<SubscriberDto>>, ApiError> {
    let subs = sqlx::query_as::<_, SubscriberDto>(
        "SELECT user_id FROM game_subscriptions WHERE game_id = $1::uuid",
    )
    .bind(&game_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(subs))
}

/// GET /api/games/{guild_id}/by-name/{game_name}
pub async fn get_game_by_name(
    State(state): State<AppState>,
    Path((guild_id, game_name)): Path<(String, String)>,
) -> Result<Json<Option<GameDto>>, ApiError> {
    let game = sqlx::query_as::<_, GameDto>(
        r#"SELECT id::text, guild_id, game_name, created_by, created_at::text
           FROM games WHERE guild_id = $1 AND LOWER(game_name) = LOWER($2)"#,
    )
    .bind(&guild_id)
    .bind(&game_name)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(game))
}

/// GET /api/games/{guild_id}/user/{user_id}
pub async fn get_user_games(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Vec<GameDto>>, ApiError> {
    let games = sqlx::query_as::<_, GameDto>(
        r#"SELECT g.id::text, g.guild_id, g.game_name, g.created_by, g.created_at::text
           FROM games g
           INNER JOIN game_subscriptions gs ON gs.game_id = g.id
           WHERE g.guild_id = $1 AND gs.user_id = $2
           ORDER BY g.game_name"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(games))
}
