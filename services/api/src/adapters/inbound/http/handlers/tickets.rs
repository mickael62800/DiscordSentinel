use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use serde::Deserialize;

use crate::adapters::inbound::http::dto::tickets::{
    AssignDto, CreateTicketDto, ListTicketsQuery, ReplyDto, TicketDetailDto, TicketResponseDto,
    UpdateStatusDto, UpdateTicketChannelDto,
};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::errors_helpers::sqlx_internal;
use crate::adapters::inbound::http::helpers::{map_to_dtos, ok_response, single_dto};
use crate::adapters::inbound::http::middleware::rbac::{require_role, Role, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use crate::domain::errors::DomainError;
use crate::domain::value_objects::TicketStatus;
use crate::ports::inbound::{AssignTicketCommand, ReplyTicketCommand, UpdateTicketChannelCommand};

pub async fn list_tickets(
    State(state): State<AppState>,
    Query(params): Query<ListTicketsQuery>,
) -> Result<Json<Vec<TicketResponseDto>>, ApiError> {
    // Validation
    validation::validate_pagination(params.limit, params.offset).map_err(ApiError)?;
    validation::validate_search(&params.search).map_err(ApiError)?;

    let limit = crate::adapters::inbound::http::helpers::normalize_limit(params.limit, 50, 200);
    let offset = crate::adapters::inbound::http::helpers::normalize_offset(params.offset);
    let tickets = state.tickets_uc.list_tickets(params.status, params.priority, params.search, params.author_id, limit, offset).await?;
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
    // Validation
    validation::validate_title(&dto.title).map_err(ApiError)?;

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
    let status = match TicketStatus::from_str(&dto.status) {
        Some(s) => s,
        None => return Err(DomainError::InvalidRule(format!(
            "Statut invalide : {}. Valeurs acceptees : {:?}",
            dto.status, TicketStatus::VALID_VALUES
        )).into()),
    };

    if status == TicketStatus::Closed {
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

#[derive(Debug, Deserialize)]
pub struct BulkDeleteTicketsParams {
    /// Filtrer par auteur (Discord user id). Optionnel.
    pub author_id: Option<String>,
    /// Borne inclusive de date (RFC3339). Optionnel.
    pub from: Option<String>,
    /// Borne inclusive de date (RFC3339). Optionnel.
    pub to: Option<String>,
    /// Safety : si true, permet de supprimer TOUS les tickets (pas de filtre).
    /// Sinon au moins un filtre est requis pour eviter un DELETE sans bornes
    /// par accident.
    #[serde(default)]
    pub all: bool,
}

/// DELETE /api/tickets/bulk — suppression en masse avec filtres optionnels.
///
/// Filtres combinables (AND) :
/// - `author_id` : ne supprime que les tickets crees par ce user
/// - `from` / `to` : plage de dates (inclusive), format RFC3339 ou YYYY-MM-DD
/// - `all=true` : autorise la suppression totale si aucun filtre fourni
///
/// Utilise un CTE pour supprimer en premier les `ticket_messages` lies
/// (meme si ON DELETE CASCADE est en place — on reste explicite pour
/// pouvoir compter ce qui a ete supprime sans joindre).
///
/// Gate RBAC : admin+ (avec bypass superadmin).
pub async fn bulk_delete_tickets(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Query(params): Query<BulkDeleteTicketsParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Gate RBAC. On utilise require_role (les tickets ne sont pas scopes
    // par guild de maniere fiable via le path). Superadmin bypass explicite.
    if let Some(Extension(ctx)) = rbac.as_ref() {
        let is_superadmin = state
            .superadmin_user_ids
            .iter()
            .any(|id| id == &ctx.discord_user_id);
        if !is_superadmin {
            require_role(ctx, Role::Admin).map_err(|_| {
                ApiError(DomainError::Forbidden(
                    "admin+ requis pour supprimer en masse des tickets".into(),
                ))
            })?;
        }
    }

    let has_filter = params.author_id.is_some() || params.from.is_some() || params.to.is_some();
    if !has_filter && !params.all {
        return Err(ApiError(DomainError::ValidationError(
            "Aucun filtre fourni. Passe all=true pour supprimer TOUS les tickets.".into(),
        )));
    }

    // Parse optionnel des dates (RFC3339 ou YYYY-MM-DD → UTC minuit).
    fn parse_date(s: &str, end_of_day: bool) -> Result<chrono::DateTime<chrono::Utc>, DomainError> {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return Ok(dt.with_timezone(&chrono::Utc));
        }
        if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            let time = if end_of_day {
                chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap()
            } else {
                chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
            };
            return Ok(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                d.and_time(time),
                chrono::Utc,
            ));
        }
        Err(DomainError::ValidationError(format!(
            "Date invalide '{s}' (attendu RFC3339 ou YYYY-MM-DD)"
        )))
    }

    let from_dt = params
        .from
        .as_deref()
        .map(|s| parse_date(s, false))
        .transpose()
        .map_err(ApiError)?;
    let to_dt = params
        .to
        .as_deref()
        .map(|s| parse_date(s, true))
        .transpose()
        .map_err(ApiError)?;

    // DELETE avec filtres dynamiques. sqlx ne gere pas les WHERE conditionnels,
    // donc on construit une clause COALESCE-based qui est neutre si le param
    // est NULL.
    let res = sqlx::query(
        r#"
        DELETE FROM tickets
        WHERE ($1::text IS NULL OR author_id = $1)
          AND ($2::timestamptz IS NULL OR created_at >= $2)
          AND ($3::timestamptz IS NULL OR created_at <= $3)
        "#,
    )
    .bind(params.author_id.as_deref())
    .bind(from_dt)
    .bind(to_dt)
    .execute(&state.pg_pool)
    .await
    .map_err(sqlx_internal("bulk_delete_tickets"))?;

    let deleted = res.rows_affected();
    tracing::info!(
        deleted,
        author_id = ?params.author_id,
        from = ?params.from,
        to = ?params.to,
        all = params.all,
        "bulk_delete_tickets"
    );

    Ok(Json(serde_json::json!({
        "deleted": deleted,
        "author_id": params.author_id,
        "from": params.from,
        "to": params.to,
    })))
}

#[cfg(test)]
#[path = "tests/tickets.rs"]
mod tests;
