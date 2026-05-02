use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use crate::adapters::inbound::http::dto::community::announcements::{
    parse_content_type, parse_recurrence, AnnouncementDto, AnnouncementRunDto, ButtonClickDto,
    ButtonInteractionDto, CreateAnnouncementDto, RecordRunResultDto, ToggleAnnouncementDto,
    UpdateAnnouncementDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;
use crate::ports::inbound::community::manage_announcements::{
    CreateAnnouncementCommand, RenderedAnnouncement, UpdateAnnouncementCommand,
};

fn map_validation_string<T>(r: Result<T, String>) -> Result<T, ApiError> {
    r.map_err(|m| ApiError(DomainError::ValidationError(m)))
}

pub async fn create_announcement(
    State(state): State<AppState>,
    Json(dto): Json<CreateAnnouncementDto>,
) -> Result<Json<AnnouncementDto>, ApiError> {
    let recurrence_type = map_validation_string(parse_recurrence(&dto.recurrence_type))?;
    let content_type = map_validation_string(parse_content_type(&dto.content_type))?;

    // TODO RBAC : verifier que l'auteur est admin+ pour cette guild via
    // require_role(...) si on extrait le RoleContext de la request.
    let created_by = "web".to_string();

    let cmd = CreateAnnouncementCommand {
        guild_id: dto.guild_id,
        name: dto.name,
        recurrence_type,
        recurrence_hour: dto.recurrence_hour,
        recurrence_minute: dto.recurrence_minute,
        recurrence_day_of_week: dto.recurrence_day_of_week,
        recurrence_day_of_month: dto.recurrence_day_of_month,
        scheduled_at: dto.scheduled_at,
        end_date: dto.end_date,
        content_type,
        content_text: dto.content_text,
        embed_title: dto.embed_title,
        embed_color: dto.embed_color,
        embed_image_url: dto.embed_image_url,
        embed_thumbnail_url: dto.embed_thumbnail_url,
        mention_everyone: dto.mention_everyone,
        mention_here: dto.mention_here,
        mention_role_ids: dto.mention_role_ids,
        channel_ids: dto.channel_ids,
        buttons: dto.buttons,
        auto_reactions: dto.auto_reactions,
        created_by,
    };
    let ann = state.announcements_uc.create(cmd).await?;
    Ok(single_dto(ann))
}

pub async fn update_announcement(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateAnnouncementDto>,
) -> Result<Json<AnnouncementDto>, ApiError> {
    let recurrence_type = map_validation_string(parse_recurrence(&dto.recurrence_type))?;
    let content_type = map_validation_string(parse_content_type(&dto.content_type))?;
    let cmd = UpdateAnnouncementCommand {
        id,
        name: dto.name,
        recurrence_type,
        recurrence_hour: dto.recurrence_hour,
        recurrence_minute: dto.recurrence_minute,
        recurrence_day_of_week: dto.recurrence_day_of_week,
        recurrence_day_of_month: dto.recurrence_day_of_month,
        scheduled_at: dto.scheduled_at,
        end_date: dto.end_date,
        content_type,
        content_text: dto.content_text,
        embed_title: dto.embed_title,
        embed_color: dto.embed_color,
        embed_image_url: dto.embed_image_url,
        embed_thumbnail_url: dto.embed_thumbnail_url,
        mention_everyone: dto.mention_everyone,
        mention_here: dto.mention_here,
        mention_role_ids: dto.mention_role_ids,
        channel_ids: dto.channel_ids,
        buttons: dto.buttons,
        auto_reactions: dto.auto_reactions,
    };
    let ann = state.announcements_uc.update(cmd).await?;
    Ok(single_dto(ann))
}

pub async fn delete_announcement(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<()>, ApiError> {
    state.announcements_uc.delete(id).await?;
    Ok(Json(()))
}

pub async fn get_announcement(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AnnouncementDto>, ApiError> {
    let ann = state.announcements_uc.get(id).await?;
    Ok(single_dto(ann))
}

pub async fn list_announcements(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<AnnouncementDto>>, ApiError> {
    let list = state.announcements_uc.list_by_guild(&guild_id).await?;
    Ok(map_to_dtos(list))
}

pub async fn toggle_announcement(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<ToggleAnnouncementDto>,
) -> Result<Json<bool>, ApiError> {
    let new_state = state.announcements_uc.toggle(id, dto.enabled).await?;
    Ok(Json(new_state))
}

pub async fn preview_announcement(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<RenderedAnnouncement>, ApiError> {
    let rendered = state.announcements_uc.preview(id).await?;
    Ok(Json(rendered))
}

pub async fn list_runs(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<RunsLimitQuery>,
) -> Result<Json<Vec<AnnouncementRunDto>>, ApiError> {
    let limit = params.limit.unwrap_or(50).min(500);
    let runs = state.announcements_uc.list_runs(id, limit).await?;
    Ok(map_to_dtos(runs))
}

#[derive(serde::Deserialize)]
pub struct RunsLimitQuery {
    pub limit: Option<i64>,
}

// ── Endpoints internes worker / bot ─────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct FetchDueQuery {
    pub limit: Option<i64>,
}

/// GET /api/announcements/internal/due — appele par announcement-worker.
/// Retourne les annonces dues, cree les runs (status=pending) et avance
/// next_run_at de chaque annonce. Le bot consume ensuite via Redis stream
/// et appelle /runs/{id}/result une fois le post fait.
pub async fn fetch_due(
    State(state): State<AppState>,
    Query(params): Query<FetchDueQuery>,
) -> Result<Json<Vec<RenderedAnnouncement>>, ApiError> {
    let limit = params.limit.unwrap_or(50).min(200);
    let payloads = state
        .announcements_uc
        .fetch_due_and_prepare(Utc::now(), limit)
        .await?;
    Ok(Json(payloads))
}

/// POST /api/announcements/internal/runs/{run_id}/result — appele par
/// le bot apres publication des messages Discord.
pub async fn record_run_result(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
    Json(dto): Json<RecordRunResultDto>,
) -> Result<Json<()>, ApiError> {
    state
        .announcements_uc
        .record_run_result(run_id, dto.channels_posted)
        .await?;
    Ok(Json(()))
}

/// POST /api/announcements/internal/button-click — appele par le bot
/// quand un user clique sur un bouton interactif d'une annonce.
pub async fn record_button_click(
    State(state): State<AppState>,
    Json(dto): Json<ButtonClickDto>,
) -> Result<Json<()>, ApiError> {
    state
        .announcements_uc
        .record_button_interaction(
            dto.announcement_id,
            dto.run_id,
            dto.user_id,
            dto.user_name,
            dto.button_custom_id,
            dto.button_label,
        )
        .await?;
    Ok(Json(()))
}

/// GET /api/announcements/{id}/interactions — liste des clics sur les boutons.
pub async fn list_button_interactions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<RunsLimitQuery>,
) -> Result<Json<Vec<ButtonInteractionDto>>, ApiError> {
    let limit = params.limit.unwrap_or(100).min(1000);
    let interactions = state
        .announcements_uc
        .list_button_interactions(id, limit)
        .await?;
    Ok(map_to_dtos(interactions))
}
