use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use serde::Deserialize;

use crate::adapters::inbound::http::dto::voice_channels::{
    AddCoAdminDto, AddWhitelistDto, BanFromChannelDto, CreateInviteLinkDto, CreateThemeDto,
    CreateVoiceChannelDto, InviteLinkResponseDto, ThemeResponseDto, TransferOwnershipDto,
    UpdateVoiceChannelDto, UseInviteLinkDto, VoiceChannelDetailDto, VoiceChannelResponseDto,
    WhitelistEntryResponseDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, ok_response, single_dto};
use crate::adapters::inbound::http::middleware::rbac::{
    check_role_for_guild, require_role, Role, RoleContext,
};
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;

/// Helper Phase 7 B — fetch le `guild_id` associe a un `channel_id` voice
/// et gate via `check_role_for_guild`. Pass-through si `rbac` absent.
///
/// Post-fix P0.C : on utilise le helper `check_role_for_guild` qui distingue
/// les erreurs DB (503 Internal) des refus de role (403 Forbidden), au lieu
/// de mapper tout en Forbidden (ce qui cachait les vraies erreurs DB
/// derriere un message trompeur "role requis").
async fn gate_by_channel_id(
    state: &AppState,
    rbac: &Option<Extension<RoleContext>>,
    channel_id: &str,
    required: Role,
    label: &'static str,
) -> Result<(), ApiError> {
    if rbac.is_none() {
        return Ok(());
    }
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT guild_id FROM voice_channels WHERE channel_id = $1",
    )
    .bind(channel_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("fetch voice channel guild: {e}"))))?;

    if let Some((guild_id,)) = row {
        check_role_for_guild(state, rbac, &guild_id, required, label).await?;
    }
    Ok(())
}
use crate::ports::inbound::{
    BanFromChannelCommand, CreateInviteLinkCommand, CreateThemeCommand, ManageCoAdminCommand,
    ManageWhitelistCommand, TransferOwnershipCommand, UpdateVoiceChannelCommand, UseInviteLinkCommand,
};

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ── Channels ──

pub async fn list_all_channels(
    State(state): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Vec<VoiceChannelResponseDto>>, ApiError> {
    let limit = crate::adapters::inbound::http::helpers::normalize_limit(params.limit, 50, 500) as usize;
    let offset = params.offset.unwrap_or(0).max(0) as usize;
    let channels = state.voice_channels_uc.list_all_channels().await?;
    let page: Vec<_> = channels.into_iter().skip(offset).take(limit).collect();
    Ok(map_to_dtos(page))
}

pub async fn list_channels(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Vec<VoiceChannelResponseDto>>, ApiError> {
    let limit = crate::adapters::inbound::http::helpers::normalize_limit(params.limit, 50, 200) as usize;
    let offset = params.offset.unwrap_or(0).max(0) as usize;
    let channels = state.voice_channels_uc.list_channels(&guild_id).await?;
    let page: Vec<_> = channels.into_iter().skip(offset).take(limit).collect();
    Ok(map_to_dtos(page))
}

pub async fn get_channel_detail(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
) -> Result<Json<VoiceChannelDetailDto>, ApiError> {
    let detail = state.voice_channels_uc.get_channel_detail(&channel_id).await?;
    Ok(single_dto(detail))
}

pub async fn create_channel(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<CreateVoiceChannelDto>,
) -> Result<Json<VoiceChannelResponseDto>, ApiError> {
    // Gate RBAC : moderator+ requis pour creer un voice channel.
    // Pass-through pour les appels bot-internal (rbac absent).
    check_role_for_guild(
        &state, &rbac, &dto.guild_id, Role::Moderator,
        "moderator+ requis pour creer un voice channel",
    )
    .await?;
    let command = dto.into();
    let channel = state.voice_channels_uc.create_channel(command).await?;

    state.broadcaster.broadcast(
        "voice_channel_created",
        serde_json::json!({
            "guild_id": &channel.guild_id,
            "id": channel.id.to_string(),
            "channel_name": &channel.channel_name,
            "owner_name": &channel.owner_name,
            "kind": channel.kind.as_str(),
        }),
    );

    Ok(single_dto(channel))
}

pub async fn close_channel(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.voice_channels_uc.close_channel(&channel_id).await?;

    state.broadcaster.broadcast(
        "voice_channel_closed",
        serde_json::json!({ "channel_id": &channel_id }),
    );

    Ok(ok_response())
}

pub async fn delete_channel(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(channel_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Phase 7 B — Gate RBAC : moderator+ pour fermer un voice channel.
    gate_by_channel_id(&state, &rbac, &channel_id, Role::Moderator, "moderator+ pour fermer un voice channel").await?;
    // DELETE fait un soft-delete (close)
    state.voice_channels_uc.delete_channel(&channel_id).await?;

    state.broadcaster.broadcast(
        "voice_channel_closed",
        serde_json::json!({ "channel_id": &channel_id }),
    );

    Ok(ok_response())
}

pub async fn update_channel(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(channel_id): Path<String>,
    Json(dto): Json<UpdateVoiceChannelDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    gate_by_channel_id(&state, &rbac, &channel_id, Role::Moderator, "moderator+ pour modifier un voice channel").await?;
    state
        .voice_channels_uc
        .update_channel(UpdateVoiceChannelCommand {
            channel_id: channel_id.clone(),
            visibility: dto.visibility,
            locked: dto.locked,
            queue_enabled: dto.queue_enabled,
            name: dto.name,
            status: dto.status,
            member_limit: dto.member_limit,
            queue_channel_id: dto.queue_channel_id,
            stage_enabled: dto.stage_enabled,
        })
        .await?;

    state.broadcaster.broadcast(
        "voice_channel_updated",
        serde_json::json!({ "channel_id": &channel_id }),
    );

    Ok(ok_response())
}

pub async fn transfer_ownership(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(channel_id): Path<String>,
    Json(dto): Json<TransferOwnershipDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    gate_by_channel_id(&state, &rbac, &channel_id, Role::Moderator, "moderator+ pour transferer un voice channel").await?;
    let new_owner_name = dto.new_owner_name.clone();

    state
        .voice_channels_uc
        .transfer_ownership(TransferOwnershipCommand {
            channel_id: channel_id.clone(),
            new_owner_id: dto.new_owner_id,
            new_owner_name: dto.new_owner_name,
        })
        .await?;

    state.broadcaster.broadcast(
        "voice_channel_updated",
        serde_json::json!({
            "channel_id": &channel_id,
            "event": "transfer",
            "new_owner": &new_owner_name,
        }),
    );

    Ok(ok_response())
}

// ── Co-admins ──

pub async fn add_co_admin(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(channel_id): Path<String>,
    Json(dto): Json<AddCoAdminDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    gate_by_channel_id(&state, &rbac, &channel_id, Role::Moderator, "moderator+ pour ajouter un co-admin voice").await?;
    state
        .voice_channels_uc
        .add_co_admin(ManageCoAdminCommand {
            channel_id,
            user_id: dto.user_id,
            user_name: dto.user_name,
        })
        .await?;

    Ok(ok_response())
}

pub async fn remove_co_admin(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    gate_by_channel_id(&state, &rbac, &channel_id, Role::Moderator, "moderator+ pour retirer un co-admin voice").await?;
    state
        .voice_channels_uc
        .remove_co_admin(&channel_id, &user_id)
        .await?;

    Ok(ok_response())
}

// ── Whitelist ──

pub async fn get_whitelist(
    State(state): State<AppState>,
    Path((guild_id, owner_id)): Path<(String, String)>,
) -> Result<Json<Vec<WhitelistEntryResponseDto>>, ApiError> {
    let entries = state.voice_channels_uc.get_whitelist(&guild_id, &owner_id).await?;
    Ok(map_to_dtos(entries))
}

pub async fn add_to_whitelist(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<AddWhitelistDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_role_for_guild(
        &state, &rbac, &dto.guild_id, Role::Moderator,
        "moderator+ pour ajouter a la whitelist voice",
    )
    .await?;
    state
        .voice_channels_uc
        .add_to_whitelist(ManageWhitelistCommand {
            guild_id: dto.guild_id,
            owner_id: dto.owner_id,
            target_id: dto.target_id,
            target_name: dto.target_name,
        })
        .await?;

    Ok(ok_response())
}

pub async fn remove_from_whitelist(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, owner_id, target_id)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Phase 7 B — Gate RBAC : moderator+ pour toucher aux permissions voice.
    if let Some(Extension(ctx)) = rbac {
        require_role(&ctx, Role::Moderator)
            .map_err(|_| ApiError(DomainError::Forbidden("moderator+ requis pour la whitelist voice".into())))?;
    }
    state
        .voice_channels_uc
        .remove_from_whitelist(&guild_id, &owner_id, &target_id)
        .await?;

    Ok(ok_response())
}

// ── Bans ──

pub async fn ban_from_channel(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(channel_id): Path<String>,
    Json(dto): Json<BanFromChannelDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    gate_by_channel_id(&state, &rbac, &channel_id, Role::Moderator, "moderator+ pour bannir d'un voice channel").await?;
    state
        .voice_channels_uc
        .ban_from_channel(BanFromChannelCommand {
            channel_id,
            user_id: dto.user_id,
            user_name: dto.user_name,
            banned_by: dto.banned_by,
            reason: dto.reason,
            duration_secs: dto.duration_secs,
        })
        .await?;

    Ok(ok_response())
}

pub async fn unban_from_channel(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    gate_by_channel_id(&state, &rbac, &channel_id, Role::Moderator, "moderator+ pour unban voice channel").await?;
    state
        .voice_channels_uc
        .unban_from_channel(&channel_id, &user_id)
        .await?;

    Ok(ok_response())
}

pub async fn check_ban(
    State(state): State<AppState>,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let banned = state.voice_channels_uc.is_banned(&channel_id, &user_id).await?;
    Ok(Json(serde_json::json!({ "banned": banned })))
}

// ── Invite Links ──

pub async fn list_invite_links(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<InviteLinkResponseDto>>, ApiError> {
    let links = state.voice_channels_uc.list_invite_links(&channel_id).await?;
    Ok(map_to_dtos(links))
}

pub async fn create_invite_link(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
    Json(dto): Json<CreateInviteLinkDto>,
) -> Result<Json<InviteLinkResponseDto>, ApiError> {
    let cmd = CreateInviteLinkCommand {
        channel_id: channel_id.clone(),
        created_by: dto.created_by,
        created_by_name: dto.created_by_name,
        duration_secs: dto.duration_secs,
        max_uses: dto.max_uses,
    };

    let link = state.voice_channels_uc.create_invite_link(cmd).await?;

    state.broadcaster.broadcast(
        "voice_invite_created",
        serde_json::json!({
            "channel_id": &channel_id,
            "code": &link.code,
            "created_by_name": &link.created_by_name,
        }),
    );

    Ok(single_dto(link))
}

pub async fn use_invite_link(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(dto): Json<UseInviteLinkDto>,
) -> Result<Json<InviteLinkResponseDto>, ApiError> {
    let cmd = UseInviteLinkCommand {
        code: code.clone(),
        user_id: dto.user_id.clone(),
        user_name: dto.user_name,
    };

    let link = state.voice_channels_uc.use_invite_link(cmd).await?;

    state.broadcaster.broadcast(
        "voice_invite_used",
        serde_json::json!({
            "channel_id": &link.channel_id,
            "code": &code,
            "user_id": &dto.user_id,
        }),
    );

    Ok(single_dto(link))
}

pub async fn revoke_invite_link(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((channel_id, link_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    gate_by_channel_id(&state, &rbac, &channel_id, Role::Moderator, "moderator+ pour revoquer un invite voice").await?;
    state.voice_channels_uc.revoke_invite_link(&channel_id, &link_id).await?;

    state.broadcaster.broadcast(
        "voice_invite_revoked",
        serde_json::json!({ "channel_id": &channel_id, "link_id": &link_id }),
    );

    Ok(ok_response())
}

// ── Themes ──

pub async fn list_themes(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<ThemeResponseDto>>, ApiError> {
    let themes = state.voice_channels_uc.list_themes(&guild_id).await?;
    Ok(map_to_dtos(themes))
}

pub async fn create_theme(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<CreateThemeDto>,
) -> Result<Json<ThemeResponseDto>, ApiError> {
    let mut cmd: CreateThemeCommand = dto.into();
    cmd.guild_id = guild_id;

    let theme = state.voice_channels_uc.create_theme(cmd).await?;
    Ok(single_dto(theme))
}

pub async fn update_theme(
    State(state): State<AppState>,
    Path((guild_id, theme_id)): Path<(String, String)>,
    Json(dto): Json<CreateThemeDto>,
) -> Result<Json<ThemeResponseDto>, ApiError> {
    let mut cmd: CreateThemeCommand = dto.into();
    cmd.guild_id = guild_id;

    let theme = state.voice_channels_uc.update_theme(&theme_id, cmd).await?;
    Ok(single_dto(theme))
}

pub async fn delete_theme(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, theme_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Phase 7 B — Gate RBAC : admin+ requis pour modifier la config themes voice.
    if let Some(Extension(ctx)) = rbac {
        require_role(&ctx, Role::Admin)
            .map_err(|_| ApiError(DomainError::Forbidden("admin+ requis pour supprimer un theme voice".into())))?;
    }
    state.voice_channels_uc.delete_theme(&guild_id, &theme_id).await?;
    Ok(ok_response())
}
