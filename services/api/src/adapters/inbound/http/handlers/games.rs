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
    pub emoji: Option<String>,
    pub category: Option<String>,
}

impl From<crate::ports::outbound::Game> for GameDto {
    fn from(g: crate::ports::outbound::Game) -> Self {
        Self {
            id: g.id,
            guild_id: g.guild_id,
            game_name: g.game_name,
            created_by: g.created_by,
            created_at: g.created_at,
            emoji: g.emoji,
            category: g.category,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateGameDto {
    pub guild_id: String,
    pub game_name: String,
    pub created_by: String,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubscribeDto {
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct SubscriberDto {
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct GamePanelDto {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: String,
    pub category: Option<String>,
}

impl From<crate::ports::outbound::GamePanel> for GamePanelDto {
    fn from(p: crate::ports::outbound::GamePanel) -> Self {
        Self {
            id: p.id,
            guild_id: p.guild_id,
            channel_id: p.channel_id,
            message_id: p.message_id,
            category: p.category,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SavePanelDto {
    pub channel_id: String,
    pub message_id: String,
    #[serde(default)]
    pub category: Option<String>,
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
    let emoji = dto.emoji.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let category = dto.category.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let game = state.game_repo.create(&dto.guild_id, &name, &dto.created_by, emoji, category).await?;
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

// ── Panels ──

pub async fn save_panel(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<SavePanelDto>,
) -> Result<Json<GamePanelDto>, ApiError> {
    let category = dto.category.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let panel = state
        .game_repo
        .save_panel(&guild_id, &dto.channel_id, &dto.message_id, category)
        .await?;
    Ok(Json(panel.into()))
}

pub async fn find_panel_by_message(
    State(state): State<AppState>,
    Path((guild_id, message_id)): Path<(String, String)>,
) -> Result<Json<Option<GamePanelDto>>, ApiError> {
    let panel = state.game_repo.find_panel_by_message(&guild_id, &message_id).await?;
    Ok(Json(panel.map(Into::into)))
}

pub async fn list_panels(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<GamePanelDto>>, ApiError> {
    let panels = state.game_repo.list_panels(&guild_id).await?;
    Ok(Json(panels.into_iter().map(Into::into).collect()))
}

pub async fn list_games_by_category(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    axum::extract::Query(q): axum::extract::Query<CategoryQuery>,
) -> Result<Json<Vec<GameDto>>, ApiError> {
    let cat = q.category.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let games = state.game_repo.list_by_category(&guild_id, cat).await?;
    Ok(Json(games.into_iter().map(Into::into).collect()))
}

#[derive(Debug, Deserialize)]
pub struct CategoryQuery {
    #[serde(default)]
    pub category: Option<String>,
}
