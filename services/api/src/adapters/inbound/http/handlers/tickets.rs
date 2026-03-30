use axum::extract::{Path, Query, State};
use axum::Json;

use crate::adapters::inbound::http::dto::tickets::{
    AssignDto, CreateTicketDto, ListTicketsQuery, ReplyDto, TicketDetailDto, TicketResponseDto,
    UpdateStatusDto, UpdateTicketChannelDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, ok_response, single_dto};
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;
use crate::ports::inbound::{AssignTicketCommand, ReplyTicketCommand, UpdateTicketChannelCommand};

pub async fn list_tickets(
    State(state): State<AppState>,
    Query(params): Query<ListTicketsQuery>,
) -> Result<Json<Vec<TicketResponseDto>>, ApiError> {
    let tickets = state.tickets_uc.list_tickets(params.status, params.priority, params.search, params.author_id).await?;
    Ok(map_to_dtos(tickets))
}

pub async fn get_ticket_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TicketDetailDto>, ApiError> {
    let detail = state.tickets_uc.get_ticket_detail(&id).await?;
    Ok(single_dto(detail))
}

pub async fn create_ticket(
    State(state): State<AppState>,
    Json(dto): Json<CreateTicketDto>,
) -> Result<Json<TicketResponseDto>, ApiError> {
    let command = dto.into();
    let ticket = state.tickets_uc.create_ticket(command).await?;

    state.broadcaster.broadcast(
        "ticket_new",
        serde_json::json!({
            "id": ticket.id.to_string(),
            "title": &ticket.title,
            "author_name": &ticket.author_name,
            "priority": &ticket.priority,
        }),
    );

    Ok(single_dto(ticket))
}

pub async fn reply_ticket(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(dto): Json<ReplyDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let author_name = dto.author_name.clone();

    state
        .tickets_uc
        .reply_ticket(ReplyTicketCommand {
            ticket_id: id.clone(),
            content: dto.content,
            author_name: dto.author_name,
            author_role: dto.author_role,
        })
        .await?;

    state.broadcaster.broadcast(
        "ticket_message",
        serde_json::json!({
            "ticket_id": &id,
            "author_name": &author_name,
        }),
    );

    Ok(ok_response())
}

pub async fn close_ticket(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.tickets_uc.close_ticket(&id).await?;

    state.broadcaster.broadcast(
        "ticket_closed",
        serde_json::json!({ "ticket_id": &id }),
    );

    Ok(ok_response())
}

pub async fn assign_ticket(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(dto): Json<AssignDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let assignee = dto.assignee;
    state
        .tickets_uc
        .assign_ticket(AssignTicketCommand {
            ticket_id: id.clone(),
            assignee: assignee.clone(),
        })
        .await?;

    state.broadcaster.broadcast(
        "ticket_assigned",
        serde_json::json!({
            "ticket_id": &id,
            "assignee": &assignee,
        }),
    );

    Ok(ok_response())
}

pub async fn update_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateStatusDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let valid_statuses = ["open", "pending", "closed"];
    if !valid_statuses.contains(&dto.status.as_str()) {
        return Err(DomainError::InvalidRule(format!(
            "Statut invalide : {}. Valeurs acceptees : open, pending, closed",
            dto.status
        )).into());
    }

    if dto.status == "closed" {
        state.tickets_uc.close_ticket(&id).await?;
    } else {
        state.tickets_uc.update_status(&id, &dto.status).await?;
    }

    state.broadcaster.broadcast(
        "ticket_status_updated",
        serde_json::json!({ "ticket_id": &id, "status": &dto.status }),
    );

    Ok(ok_response())
}

pub async fn update_ticket_channel(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateTicketChannelDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .tickets_uc
        .update_ticket_channel(UpdateTicketChannelCommand {
            ticket_id: id.clone(),
            voice_channel_id: dto.voice_channel_id,
            invited_user_id: dto.invited_user_id,
        })
        .await?;

    state.broadcaster.broadcast(
        "ticket_channel_updated",
        serde_json::json!({ "ticket_id": &id }),
    );

    Ok(ok_response())
}
