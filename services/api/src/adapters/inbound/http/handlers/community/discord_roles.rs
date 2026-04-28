use axum::extract::Path;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::middleware::rbac::check_role;
use crate::domain::enums::system::role::Role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::system::discord_role::parse_discord_permissions_bitfield;
use crate::domain::entities::system::discord_role::DiscordRole;
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
            // Phase 2 A.3 — `permissions` est BIGINT en base. On le serialise
            // en String pour le wire JSON (les bitfields Discord peuvent depasser
            // Number.MAX_SAFE_INTEGER cote JS).
            permissions: r.permissions.to_string(),
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

/// POST /api/discord-roles/{guild_id}/create — Creer un role Discord
pub async fn create_role(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(body): Json<CreateRoleRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = state.discord_api
        .create_role(&guild_id, &body.name, body.color, body.permissions.as_deref())
        .await?;
    Ok(Json(result))
}

/// PATCH /api/discord-roles/{guild_id}/{role_id} — Modifier un role Discord
pub async fn edit_role(
    State(state): State<AppState>,
    Path((guild_id, role_id)): Path<(String, String)>,
    Json(body): Json<EditRoleRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = state.discord_api
        .edit_role(&guild_id, &role_id, body.name.as_deref(), body.color, body.permissions.as_deref(), body.mentionable, body.hoist)
        .await?;
    Ok(Json(result))
}

/// DELETE /api/discord-roles/{guild_id}/{role_id} — Supprimer un role Discord
pub async fn delete_role(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, role_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_role(&rbac, Role::Admin, "admin+ requis pour supprimer un role")?;
    state.discord_api.delete_role(&guild_id, &role_id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub color: u32,
    pub permissions: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct EditRoleRequest {
    pub name: Option<String>,
    pub color: Option<u32>,
    pub permissions: Option<String>,
    pub mentionable: Option<bool>,
    pub hoist: Option<bool>,
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
            // Regle metier : parse bitfield permissions -> `domain::entities::parse_discord_permissions_bitfield`
            permissions: parse_discord_permissions_bitfield(&r.permissions),
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

#[cfg(test)]
#[path = "tests/discord_roles.rs"]
mod tests;
