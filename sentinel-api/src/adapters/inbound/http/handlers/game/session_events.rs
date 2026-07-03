//! Endpoints des "evenements de serveur" Game Portal : reglage du role a
//! pinguer par (guild, template), inscriptions des joueurs a une session, et
//! enregistrement des salons Discord crees par le bot.

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

// ── Reglages par template (role a pinguer) ──

#[derive(Serialize)]
pub struct TemplateSettingsDto {
    pub template_slug: String,
    pub discord_role_id: Option<String>,
}

/// GET /api/games/{guild_id}/template-settings
pub async fn list_template_settings(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<TemplateSettingsDto>>, ApiError> {
    let list = state.game_template_settings_repo.list(&guild_id).await?;
    Ok(Json(
        list.into_iter()
            .map(|s| TemplateSettingsDto {
                template_slug: s.template_slug,
                discord_role_id: s.discord_role_id,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct SetRoleDto {
    pub discord_role_id: Option<String>,
}

/// PUT /api/games/{guild_id}/template-settings/{slug}
pub async fn set_template_role(
    State(state): State<AppState>,
    Path((guild_id, slug)): Path<(String, String)>,
    Json(dto): Json<SetRoleDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .game_template_settings_repo
        .set_role(&guild_id, &slug, dto.discord_role_id.as_deref())
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── Inscriptions a une session ──

#[derive(Serialize)]
pub struct RegistrationDto {
    pub user_id: String,
    pub registered_at: String,
}

#[derive(Deserialize)]
pub struct RegisterDto {
    pub user_id: String,
}

/// POST /api/games/servers/{server_id}/registrations
pub async fn register_player(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(dto): Json<RegisterDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .game_session_reg_repo
        .register(server_id, &dto.user_id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /api/games/servers/{server_id}/registrations/{user_id}
pub async fn unregister_player(
    State(state): State<AppState>,
    Path((server_id, user_id)): Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .game_session_reg_repo
        .unregister(server_id, &user_id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /api/games/servers/{server_id}/registrations
pub async fn list_registrations(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<Vec<RegistrationDto>>, ApiError> {
    let list = state.game_session_reg_repo.list(server_id).await?;
    Ok(Json(
        list.into_iter()
            .map(|r| RegistrationDto {
                user_id: r.user_id,
                registered_at: r.registered_at.to_rfc3339(),
            })
            .collect(),
    ))
}

// ── Salons de session (enregistres par le bot) ──

#[derive(Deserialize)]
pub struct SessionChannelsDto {
    pub text_channel_id: Option<String>,
    pub voice_channel_id: Option<String>,
}

/// PATCH /api/games/servers/{server_id}/session-channels
pub async fn set_session_channels(
    State(state): State<AppState>,
    Path(server_id): Path<Uuid>,
    Json(dto): Json<SessionChannelsDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .game_server_repo
        .set_session_channels(
            server_id,
            dto.text_channel_id.as_deref(),
            dto.voice_channel_id.as_deref(),
        )
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
