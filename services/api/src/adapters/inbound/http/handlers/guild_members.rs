use axum::extract::{Path, State};
use axum::{Extension, Json};
use redis::AsyncCommands;
use serde::Deserialize;

use tracing::warn;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::middleware::rbac::{require_role, Role, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::{GuildMember, MemberSummary};
use crate::domain::errors::DomainError;
use crate::domain::services::DiscordMember;
use crate::ports::inbound::{RegisterMemberCommand, SyncMembersCommand, UpdateMemberCommand};

const MEMBERS_TTL: u64 = 600; // 10 minutes

/// GET /api/guilds/{guild_id}/members — liste les membres Discord (cache 10min, fallback Discord API)
pub async fn list_members(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<DiscordMember>>, ApiError> {
    let cache_key = format!("guild:members:{guild_id}");

    // Cache-first
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(cached) = conn.get::<_, Option<String>>(&cache_key).await {
            if let Some(json) = cached {
                if let Ok(members) = serde_json::from_str::<Vec<DiscordMember>>(&json) {
                    return Ok(Json(members));
                }
            }
        }
    }

    let members = state.discord_api.list_members(&guild_id, 1000).await?;

    // Populate cache
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(&members) {
            if let Err(e) = conn.set_ex::<_, _, ()>(&cache_key, json, MEMBERS_TTL).await {
                warn!(error = %e, cache_key = %cache_key, "Echec cache set members");
            }
        }
    }

    Ok(Json(members))
}

/// GET /api/members/{guild_id} — liste les membres depuis la BDD
pub async fn list_members_db(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<GuildMember>>, ApiError> {
    let members = state.members_uc.list_members(&guild_id).await?;
    Ok(Json(members))
}

/// GET /api/members/{guild_id}/{user_id} — profil d'un membre
pub async fn get_member(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<GuildMember>, ApiError> {
    let member = state.members_uc.get_member(&guild_id, &user_id).await?;
    Ok(Json(member))
}

/// GET /api/members/{guild_id}/{user_id}/summary — profil complet agrege
pub async fn get_member_summary(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<MemberSummary>, ApiError> {
    let summary = state.members_uc.get_member_summary(&guild_id, &user_id).await?;
    Ok(Json(summary))
}

/// POST /api/members/sync — sync bulk depuis un bot
pub async fn sync_members(
    State(state): State<AppState>,
    Json(payload): Json<SyncMembersPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let count = state.members_uc.sync_members(SyncMembersCommand {
        guild_id: payload.guild_id,
        members: payload.members,
    }).await?;
    Ok(Json(serde_json::json!({ "synced": count })))
}

/// POST /api/members/register — enregistre un nouveau membre
pub async fn register_member(
    State(state): State<AppState>,
    Json(member): Json<GuildMember>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.members_uc.register_member(RegisterMemberCommand { member }).await?;
    Ok(ok_response())
}

/// DELETE /api/members/{guild_id}/{user_id} — supprime un membre
pub async fn remove_member(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Phase 7 B — Gate RBAC : moderator+ requis pour retirer un membre du cache local.
    if let Some(Extension(ctx)) = rbac {
        require_role(&ctx, Role::Moderator)
            .map_err(|_| ApiError(DomainError::Forbidden("moderator+ requis pour retirer un membre".into())))?;
    }
    state.members_uc.remove_member(&guild_id, &user_id).await?;
    Ok(ok_response())
}

/// PATCH /api/members/{guild_id}/{user_id} — met a jour un membre
pub async fn update_member(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(payload): Json<UpdateMemberPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.members_uc.update_member(UpdateMemberCommand {
        guild_id,
        user_id,
        username: payload.username,
        display_name: payload.display_name,
        avatar: payload.avatar,
        roles: payload.roles,
    }).await?;
    Ok(ok_response())
}

#[derive(Deserialize)]
pub struct SyncMembersPayload {
    pub guild_id: String,
    pub members: Vec<GuildMember>,
}

#[derive(Deserialize)]
pub struct UpdateMemberPayload {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub roles: Option<serde_json::Value>,
}
