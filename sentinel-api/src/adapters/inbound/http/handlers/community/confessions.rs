use axum::extract::{Path, Query, State};
use axum::Json;
use uuid::Uuid;

use crate::adapters::inbound::http::dto::community::confessions::{
    parse_report_status, ConfessionDto, ConfigDto, CreateConfessionDto, CreateReplyDto,
    CreateReportDto, DeleteConfessionDto, EditConfessionDto, ReplyDto, ReportDto, ResolveReportDto,
    SaveConfigDto, UpdateMessageRefsDto, UpdateReplyMessageDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, single_dto};
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::entities::community::confession::{ConfessionConfig, ReportStatus};
use sentinel_core::domain::errors::DomainError;
use crate::ports::inbound::community::manage_confessions::{
    CreateConfessionCommand, CreateReplyCommand, CreateReportCommand,
};

#[derive(serde::Deserialize)]
pub struct ListConfessionsQuery {
    pub limit: Option<i64>,
    pub include_deleted: Option<bool>,
}

#[derive(serde::Deserialize)]
pub struct ListReportsQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

// ── Confessions ─────────────────────────────────────────────────────────

pub async fn create_confession(
    State(state): State<AppState>,
    Json(dto): Json<CreateConfessionDto>,
) -> Result<Json<ConfessionDto>, ApiError> {
    let c = state
        .confessions_uc
        .create(CreateConfessionCommand {
            guild_id: dto.guild_id.clone(),
            author_user_id: dto.author_user_id,
            content: dto.content,
        })
        .await?;
    state.broadcaster.broadcast(
        "confession_created",
        serde_json::json!({
            "guild_id": &c.guild_id,
            "id": c.id,
            "public_number": c.public_number,
        }),
    );
    Ok(single_dto(c))
}

pub async fn update_message_refs(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateMessageRefsDto>,
) -> Result<Json<()>, ApiError> {
    state
        .confessions_uc
        .update_message_refs(id, dto.message_id, dto.channel_id, dto.thread_id)
        .await?;
    Ok(Json(()))
}

pub async fn edit_confession(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<EditConfessionDto>,
) -> Result<Json<ConfessionDto>, ApiError> {
    let c = state
        .confessions_uc
        .edit_content(id, &dto.author_user_id, dto.content)
        .await?;
    state.broadcaster.broadcast(
        "confession_edited",
        serde_json::json!({
            "guild_id": &c.guild_id,
            "id": c.id,
            "public_number": c.public_number,
        }),
    );
    Ok(single_dto(c))
}

pub async fn delete_confession(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<DeleteConfessionDto>,
) -> Result<Json<ConfessionDto>, ApiError> {
    let c = state
        .confessions_uc
        .delete(id, dto.deleted_by, dto.reason)
        .await?;
    state.broadcaster.broadcast(
        "confession_deleted",
        serde_json::json!({
            "guild_id": &c.guild_id,
            "id": c.id,
            "public_number": c.public_number,
            "message_id": &c.message_id,
            "channel_id": &c.channel_id,
        }),
    );
    Ok(single_dto(c))
}

pub async fn get_confession(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ConfessionDto>, ApiError> {
    let c = state.confessions_uc.get(id).await?;
    Ok(single_dto(c))
}

pub async fn get_by_message_id(
    State(state): State<AppState>,
    Path(message_id): Path<String>,
) -> Result<Json<Option<ConfessionDto>>, ApiError> {
    let c = state.confessions_uc.get_by_message_id(&message_id).await?;
    Ok(Json(c.map(ConfessionDto::from)))
}

pub async fn list_confessions(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<ListConfessionsQuery>,
) -> Result<Json<Vec<ConfessionDto>>, ApiError> {
    let limit = params.limit.unwrap_or(50).min(500);
    let include_deleted = params.include_deleted.unwrap_or(false);
    let list = state
        .confessions_uc
        .list(&guild_id, limit, include_deleted)
        .await?;
    Ok(map_to_dtos(list))
}

// ── Replies ─────────────────────────────────────────────────────────────

pub async fn create_reply(
    State(state): State<AppState>,
    Path(confession_id): Path<Uuid>,
    Json(dto): Json<CreateReplyDto>,
) -> Result<Json<ReplyDto>, ApiError> {
    let r = state
        .confessions_uc
        .create_reply(CreateReplyCommand {
            confession_id,
            author_user_id: dto.author_user_id,
            content: dto.content,
            is_anonymous: dto.is_anonymous,
        })
        .await?;
    state.broadcaster.broadcast(
        "confession_reply_created",
        serde_json::json!({
            "confession_id": confession_id,
            "id": r.id,
            "public_number": r.public_number,
        }),
    );
    Ok(single_dto(r))
}

pub async fn update_reply_message_id(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateReplyMessageDto>,
) -> Result<Json<()>, ApiError> {
    state
        .confessions_uc
        .update_reply_message_id(id, dto.message_id)
        .await?;
    Ok(Json(()))
}

pub async fn delete_reply(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<DeleteConfessionDto>,
) -> Result<Json<ReplyDto>, ApiError> {
    let r = state.confessions_uc.delete_reply(id, dto.deleted_by).await?;
    state.broadcaster.broadcast(
        "confession_reply_deleted",
        serde_json::json!({
            "confession_id": r.confession_id,
            "id": r.id,
            "message_id": &r.message_id,
        }),
    );
    Ok(single_dto(r))
}

pub async fn list_replies(
    State(state): State<AppState>,
    Path(confession_id): Path<Uuid>,
) -> Result<Json<Vec<ReplyDto>>, ApiError> {
    let list = state.confessions_uc.list_replies(confession_id).await?;
    Ok(map_to_dtos(list))
}

// ── Reports ─────────────────────────────────────────────────────────────

pub async fn create_report(
    State(state): State<AppState>,
    Json(dto): Json<CreateReportDto>,
) -> Result<Json<ReportDto>, ApiError> {
    let r = state
        .confessions_uc
        .create_report(CreateReportCommand {
            guild_id: dto.guild_id.clone(),
            confession_id: dto.confession_id,
            reply_id: dto.reply_id,
            reporter_user_id: dto.reporter_user_id,
            reason: dto.reason,
        })
        .await?;
    state.broadcaster.broadcast(
        "confession_report_created",
        serde_json::json!({ "guild_id": &r.guild_id, "id": r.id }),
    );
    Ok(single_dto(r))
}

pub async fn list_reports(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<ListReportsQuery>,
) -> Result<Json<Vec<ReportDto>>, ApiError> {
    let limit = params.limit.unwrap_or(50).min(500);
    let status = params.status.as_deref().and_then(ReportStatus::from_str);
    let list = state.confessions_uc.list_reports(&guild_id, status, limit).await?;
    Ok(map_to_dtos(list))
}

pub async fn resolve_report(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<ResolveReportDto>,
) -> Result<Json<()>, ApiError> {
    let status = parse_report_status(&dto.status)
        .map_err(|m| ApiError(DomainError::ValidationError(m)))?;
    state
        .confessions_uc
        .resolve_report(id, status, dto.resolved_by)
        .await?;
    Ok(Json(()))
}

// ── Config ──────────────────────────────────────────────────────────────

pub async fn get_config(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<ConfigDto>, ApiError> {
    let cfg = state.confessions_uc.get_config(&guild_id).await?;
    Ok(single_dto(cfg))
}

pub async fn save_config(
    State(state): State<AppState>,
    Json(dto): Json<SaveConfigDto>,
) -> Result<Json<ConfigDto>, ApiError> {
    let cfg = ConfessionConfig {
        guild_id: dto.guild_id,
        enabled: dto.enabled,
        channel_id: dto.channel_id,
        panel_message_id: dto.panel_message_id,
        cooldown_secs: dto.cooldown_secs,
        max_per_day: dto.max_per_day,
        min_chars: dto.min_chars,
        max_chars: dto.max_chars,
        automod_enabled: dto.automod_enabled,
        banned_user_ids: dto.banned_user_ids,
        updated_at: chrono::Utc::now(),
    };
    let saved = state.confessions_uc.save_config(cfg).await?;
    Ok(single_dto(saved))
}
