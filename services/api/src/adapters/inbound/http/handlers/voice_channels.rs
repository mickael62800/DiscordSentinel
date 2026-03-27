use axum::extract::{Path, State};
use axum::Json;

use crate::adapters::inbound::http::dto::voice_channels::{
    AddCoAdminDto, AddWhitelistDto, BanFromChannelDto, CreateVoiceChannelDto,
    TransferOwnershipDto, UpdateVoiceChannelDto, VoiceChannelDetailDto, VoiceChannelResponseDto,
    WhitelistEntryResponseDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::{
    BanFromChannelCommand, ManageCoAdminCommand, ManageWhitelistCommand,
    TransferOwnershipCommand, UpdateVoiceChannelCommand,
};

// ── Channels ──

pub async fn list_all_channels(
    State(state): State<AppState>,
) -> Result<Json<Vec<VoiceChannelResponseDto>>, ApiError> {
    let channels = state.voice_channels_uc.list_all_channels().await?;
    let dtos: Vec<VoiceChannelResponseDto> = channels.into_iter().map(VoiceChannelResponseDto::from).collect();
    Ok(Json(dtos))
}

pub async fn list_channels(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<VoiceChannelResponseDto>>, ApiError> {
    let channels = state.voice_channels_uc.list_channels(&guild_id).await?;
    let dtos: Vec<VoiceChannelResponseDto> = channels.into_iter().map(VoiceChannelResponseDto::from).collect();
    Ok(Json(dtos))
}

pub async fn get_channel_detail(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
) -> Result<Json<VoiceChannelDetailDto>, ApiError> {
    let detail = state.voice_channels_uc.get_channel_detail(&channel_id).await?;
    Ok(Json(VoiceChannelDetailDto::from(detail)))
}

pub async fn create_channel(
    State(state): State<AppState>,
    Json(dto): Json<CreateVoiceChannelDto>,
) -> Result<Json<VoiceChannelResponseDto>, ApiError> {
    let command = dto.into();
    let channel = state.voice_channels_uc.create_channel(command).await?;

    state.broadcaster.broadcast(
        "voice_channel_created",
        serde_json::json!({
            "id": channel.id.to_string(),
            "channel_name": &channel.channel_name,
            "owner_name": &channel.owner_name,
            "kind": &channel.kind,
        }),
    );

    Ok(Json(VoiceChannelResponseDto::from(channel)))
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

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn delete_channel(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // DELETE fait un soft-delete (close)
    state.voice_channels_uc.delete_channel(&channel_id).await?;

    state.broadcaster.broadcast(
        "voice_channel_closed",
        serde_json::json!({ "channel_id": &channel_id }),
    );

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn update_channel(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
    Json(dto): Json<UpdateVoiceChannelDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
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
        })
        .await?;

    state.broadcaster.broadcast(
        "voice_channel_updated",
        serde_json::json!({ "channel_id": &channel_id }),
    );

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn transfer_ownership(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
    Json(dto): Json<TransferOwnershipDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
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

    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── Co-admins ──

pub async fn add_co_admin(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
    Json(dto): Json<AddCoAdminDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .voice_channels_uc
        .add_co_admin(ManageCoAdminCommand {
            channel_id,
            user_id: dto.user_id,
            user_name: dto.user_name,
        })
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn remove_co_admin(
    State(state): State<AppState>,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .voice_channels_uc
        .remove_co_admin(&channel_id, &user_id)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── Whitelist ──

pub async fn get_whitelist(
    State(state): State<AppState>,
    Path((guild_id, owner_id)): Path<(String, String)>,
) -> Result<Json<Vec<WhitelistEntryResponseDto>>, ApiError> {
    let entries = state.voice_channels_uc.get_whitelist(&guild_id, &owner_id).await?;
    let dtos: Vec<WhitelistEntryResponseDto> = entries.into_iter().map(WhitelistEntryResponseDto::from).collect();
    Ok(Json(dtos))
}

pub async fn add_to_whitelist(
    State(state): State<AppState>,
    Json(dto): Json<AddWhitelistDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .voice_channels_uc
        .add_to_whitelist(ManageWhitelistCommand {
            guild_id: dto.guild_id,
            owner_id: dto.owner_id,
            target_id: dto.target_id,
            target_name: dto.target_name,
        })
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn remove_from_whitelist(
    State(state): State<AppState>,
    Path((guild_id, owner_id, target_id)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .voice_channels_uc
        .remove_from_whitelist(&guild_id, &owner_id, &target_id)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── Bans ──

pub async fn ban_from_channel(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
    Json(dto): Json<BanFromChannelDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
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

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn unban_from_channel(
    State(state): State<AppState>,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .voice_channels_uc
        .unban_from_channel(&channel_id, &user_id)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn check_ban(
    State(state): State<AppState>,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let banned = state.voice_channels_uc.is_banned(&channel_id, &user_id).await?;
    Ok(Json(serde_json::json!({ "banned": banned })))
}
