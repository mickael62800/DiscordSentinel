use axum::extract::Path;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use redis::AsyncCommands;
use serde::Deserialize;

use tracing::warn;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::middleware::rbac::check_role_for_guild;
use crate::adapters::inbound::http::middleware::rbac::require_role;
use crate::domain::enums::system::role::Role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::community::guild_member::GuildMember;
use crate::domain::entities::community::guild_member::MemberSummary;
use crate::domain::entities::community::guild_member_reset::DISCORD_LIST_MEMBERS_CAP;
use crate::domain::entities::community::guild_member_reset::MEMBER_RESET_TABLES;
use crate::domain::entities::community::guild_member_reset::MEMBERS_CACHE_TTL_SECS;
use crate::domain::errors::DomainError;
use crate::adapters::outbound::discord_api::DiscordMember;
use crate::ports::inbound::community::manage_members::RegisterMemberCommand;
use crate::ports::inbound::community::manage_members::SyncMembersCommand;
use crate::ports::inbound::community::manage_members::UpdateMemberCommand;
/// GET /api/guilds/{guild_id}/members — liste les membres Discord (cache 10min, fallback Discord API)
pub async fn list_members(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<DiscordMember>>, ApiError> {
    let cache_key = format!("guild:members:{guild_id}");

    // Cache-first
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(Some(json)) = conn.get::<_, Option<String>>(&cache_key).await {
            if let Ok(members) = serde_json::from_str::<Vec<DiscordMember>>(&json) {
                return Ok(Json(members));
            }
        }
    }

    let members = state.discord_api.list_members(&guild_id, DISCORD_LIST_MEMBERS_CAP).await?;

    // Populate cache
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(&members) {
            if let Err(e) = conn.set_ex::<_, _, ()>(&cache_key, json, MEMBERS_CACHE_TTL_SECS).await {
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
        user_id: user_id.into(),
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

/// POST /api/members/{guild_id}/{user_id}/reset — nettoie TOUTES les donnees
/// de moderation d'un membre sur une guild en une seule transaction.
///
/// Supprime :
/// - infractions (table `infractions`)
/// - actions de moderation (table `moderation_actions`, colonne target_id)
/// - points de conduite + historique (`user_conduct_points`, `conduct_points_log`)
/// - strikes (`user_strikes`)
/// - notes moderateurs (`user_notes`)
/// - surveillance manuelle (`manual_watched_users`)
/// - rappels de sanction (`sanction_reminders`, par target_id)
///
/// **Operation irreversible**, gatee derriere `Role::Admin` + bypass superadmin.
/// Tout se fait dans une transaction atomique : en cas d'erreur sur un DELETE,
/// on rollback et on retourne l'erreur — l'etat DB reste coherent.
pub async fn reset_member(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_role_for_guild(
        &state,
        &rbac,
        &guild_id,
        Role::Admin,
        "admin+ requis pour reinitialiser un membre",
    )
    .await?;

    let mut tx = state
        .pg_pool
        .begin()
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("begin tx reset: {e}"))))?;

    // Liste des tables a purger : regle metier dans
    // `domain/entities/guild_member_reset.rs::MEMBER_RESET_TABLES`.
    let mut totals = serde_json::Map::new();
    for entry in MEMBER_RESET_TABLES {
        let sql = format!(
            "DELETE FROM {} WHERE guild_id = $1 AND {} = $2",
            entry.sql_table, entry.user_column,
        );
        let res = sqlx::query(&sql)
            .bind(&guild_id)
            .bind(&user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError(DomainError::Internal(format!(
                "reset_member {}: {e}", entry.sql_table,
            ))))?;
        totals.insert(entry.response_key.into(), res.rows_affected().into());
    }

    tx.commit()
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("commit tx reset: {e}"))))?;

    tracing::info!(
        guild_id = %guild_id,
        user_id = %user_id,
        "reset_member effectue"
    );

    state.broadcaster.broadcast(
        "member_reset",
        serde_json::json!({
            "guild_id": &guild_id,
            "user_id": &user_id,
            "totals": &totals,
        }),
    );

    Ok(Json(serde_json::json!({
        "status": "ok",
        "guild_id": guild_id,
        "user_id": user_id,
        "totals": totals,
    })))
}

#[derive(Deserialize)]
pub struct UpdateMemberPayload {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub roles: Option<serde_json::Value>,
}

#[cfg(test)]
#[path = "tests/guild_members.rs"]
mod tests;
