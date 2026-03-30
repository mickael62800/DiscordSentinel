use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::DiscordRole;

#[derive(Debug, Serialize)]
pub struct DiscordRoleDto {
    pub id: String,
    pub guild_id: String,
    pub name: String,
    pub color: i32,
    pub position: i32,
    pub permissions: String,
    pub mentionable: bool,
    pub managed: bool,
    pub icon: Option<String>,
    pub member_count: i32,
    pub synced_at: String,
}

impl From<DiscordRole> for DiscordRoleDto {
    fn from(r: DiscordRole) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            name: r.name,
            color: r.color,
            position: r.position,
            permissions: r.permissions,
            mentionable: r.mentionable,
            managed: r.managed,
            icon: r.icon,
            member_count: r.member_count,
            synced_at: r.synced_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SyncRoleDto {
    pub id: String,
    pub name: String,
    pub color: i32,
    pub position: i32,
    pub permissions: String,
    pub mentionable: bool,
    pub managed: bool,
    pub icon: Option<String>,
    pub member_count: i32,
}

#[derive(Debug, Deserialize)]
pub struct SyncRolesRequest {
    pub roles: Vec<SyncRoleDto>,
}

/// GET /api/discord-roles/{guild_id} — Liste les roles Discord d'un serveur
pub async fn list_roles(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<DiscordRoleDto>>, ApiError> {
    let roles = state.discord_role_repo.find_by_guild(&guild_id).await?;
    Ok(map_to_dtos(roles))
}

/// POST /api/discord-roles/{guild_id}/sync — Synchronise les roles (appele par le bot)
pub async fn sync_roles(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(body): Json<SyncRolesRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let roles: Vec<DiscordRole> = body
        .roles
        .into_iter()
        .map(|r| DiscordRole {
            id: r.id,
            guild_id: guild_id.clone(),
            name: r.name,
            color: r.color,
            position: r.position,
            permissions: r.permissions,
            mentionable: r.mentionable,
            managed: r.managed,
            icon: r.icon,
            member_count: r.member_count,
            synced_at: chrono::Utc::now(),
        })
        .collect();

    let count = roles.len();
    state.discord_role_repo.sync_roles(&guild_id, roles).await?;

    Ok(Json(serde_json::json!({ "synced": count })))
}
