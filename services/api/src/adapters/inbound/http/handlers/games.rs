use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::{require_role, Role, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;

// ── DTOs ──

#[derive(Debug, Serialize)]
pub struct GameDto {
    pub id: String,
    pub guild_id: String,
    pub game_name: String,
    pub created_by: String,
    pub created_at: String,
}

impl From<crate::ports::outbound::Game> for GameDto {
    fn from(g: crate::ports::outbound::Game) -> Self {
        Self { id: g.id, guild_id: g.guild_id, game_name: g.game_name, created_by: g.created_by, created_at: g.created_at }
    }
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

#[derive(Debug, Serialize)]
pub struct SubscriberDto {
    pub user_id: String,
}

// ── Games CRUD (via GameRepository) ──

pub async fn list_games(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<GameDto>>, ApiError> {
    let games = state.game_repo.list(&guild_id).await?;
    Ok(Json(games.into_iter().map(Into::into).collect()))
}

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
    let game = state.game_repo.create(&dto.guild_id, &name, &dto.created_by).await?;
    Ok(Json(game.into()))
}

pub async fn delete_game(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, game_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    if let Some(Extension(ctx)) = rbac {
        require_role(&ctx, Role::Admin)
            .map_err(|_| ApiError(DomainError::Forbidden("admin+ requis pour supprimer une game".into())))?;
    }
    if !state.game_repo.delete(&guild_id, &game_id).await? {
        return Err(DomainError::NotFound("Jeu introuvable".into()).into());
    }
    Ok(StatusCode::NO_CONTENT)
}

// ── Subscriptions ──

pub async fn subscribe(
    State(state): State<AppState>,
    Path((guild_id, game_id)): Path<(String, String)>,
    Json(dto): Json<SubscribeDto>,
) -> Result<StatusCode, ApiError> {
    state.game_repo.subscribe(&guild_id, &game_id, &dto.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unsubscribe(
    State(state): State<AppState>,
    Path((guild_id, game_id, user_id)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    state.game_repo.unsubscribe(&guild_id, &game_id, &user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_subscribers(
    State(state): State<AppState>,
    Path((_guild_id, game_id)): Path<(String, String)>,
) -> Result<Json<Vec<SubscriberDto>>, ApiError> {
    let subs = state.game_repo.get_subscribers(&game_id).await?;
    Ok(Json(subs.into_iter().map(|u| SubscriberDto { user_id: u }).collect()))
}

pub async fn get_game_by_name(
    State(state): State<AppState>,
    Path((guild_id, game_name)): Path<(String, String)>,
) -> Result<Json<Option<GameDto>>, ApiError> {
    let game = state.game_repo.find_by_name(&guild_id, &game_name).await?;
    Ok(Json(game.map(Into::into)))
}

pub async fn get_user_games(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Vec<GameDto>>, ApiError> {
    let games = state.game_repo.get_user_games(&guild_id, &user_id).await?;
    Ok(Json(games.into_iter().map(Into::into).collect()))
}
