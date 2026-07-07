use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::system::tickets::AssignDto;
use crate::adapters::inbound::http::dto::system::tickets::CreateTicketDto;
use crate::adapters::inbound::http::dto::system::tickets::ListTicketsQuery;
use crate::adapters::inbound::http::dto::system::tickets::ReplyDto;
use crate::adapters::inbound::http::dto::system::tickets::TicketDetailDto;
use crate::adapters::inbound::http::dto::system::tickets::TicketResponseDto;
use crate::adapters::inbound::http::dto::system::tickets::UpdateStatusDto;
use crate::adapters::inbound::http::dto::system::tickets::UpdateTicketChannelDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::middleware::rbac::check_role_for_guild;
use crate::adapters::inbound::http::middleware::rbac::lookup_role;
use crate::adapters::inbound::http::middleware::rbac::require_role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use crate::ports::inbound::system::manage_tickets::AssignTicketCommand;
use crate::ports::inbound::system::manage_tickets::ReplyTicketCommand;
use crate::ports::inbound::system::manage_tickets::UpdateTicketChannelCommand;
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::enums::system::ticket_status::TicketStatus;
use sentinel_core::domain::errors::DomainError;

/// S1 — Autorisation web pour un endpoint mono-ticket (lecture ou mutation).
///
/// - **Appel bot / interne** (pas de `RoleContext`, Bearer api_key de confiance) :
///   retourne `Ok(None)` -> comportement inchange, le bot est de confiance et
///   agit de toute facon via gRPC.
/// - **Appel web** (`RoleContext` present via `X-Discord-Token`) : on resout le
///   `guild_id` du ticket et on exige Moderator+ sur CETTE guild. Un superadmin
///   bypass la gate. Si le ticket est legacy (guild_id NULL), l'acces web est
///   REFUSE (403) -> fail-closed (mieux vaut refuser que fuiter cross-guild).
///
/// Retourne `Ok(Some((guild_id, role_effectif)))` pour un web autorise (le role
/// sert a deriver l'identite cote `reply_ticket`, anti-impersonation S4).
async fn require_ticket_web(
    state: &AppState,
    rbac: &Option<Extension<RoleContext>>,
    id: &str,
) -> Result<Option<(String, Role)>, ApiError> {
    let Some(Extension(ctx)) = rbac.as_ref() else {
        return Ok(None);
    };
    let is_superadmin = state
        .superadmin_user_ids
        .iter()
        .any(|sid| sid == &ctx.discord_user_id);

    let detail = state.tickets_uc.get_ticket_detail(id).await?;
    let Some(gid) = detail.ticket.guild_id else {
        // Ticket legacy sans guild_id : acces web refuse (le bot gRPC y accede).
        if is_superadmin {
            return Ok(Some((String::new(), Role::Owner)));
        }
        return Err(ApiError(DomainError::Forbidden(
            "ticket sans guild (legacy) : acces web refuse".into(),
        )));
    };

    let role = if is_superadmin {
        Role::Owner
    } else {
        lookup_role(state, &ctx.discord_user_id, &gid)
            .await
            .map_err(|e| {
                ApiError(DomainError::Internal(format!(
                    "RBAC lookup role (ticket) : {e}"
                )))
            })?
    };
    if !role.satisfies(Role::Moderator) {
        return Err(ApiError(DomainError::Forbidden(
            "moderator+ requis pour ce ticket".into(),
        )));
    }
    Ok(Some((gid, role)))
}

pub async fn list_tickets(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Query(params): Query<ListTicketsQuery>,
) -> Result<Json<Vec<TicketResponseDto>>, ApiError> {
    // Validation
    validation::validate_pagination(params.limit, params.offset).map_err(ApiError)?;
    validation::validate_search(&params.search).map_err(ApiError)?;

    let limit = crate::adapters::inbound::http::helpers::normalize_limit(params.limit, 50, 200);
    let offset = crate::adapters::inbound::http::helpers::normalize_offset(params.offset);
    let tickets = state
        .tickets_uc
        .list_tickets(
            params.status,
            params.priority,
            params.search,
            params.author_id,
            limit,
            offset,
        )
        .await?;

    // S1 — scope web : on ne retourne que les tickets des guilds ou le caller
    // est Moderator+. Les tickets legacy (guild_id NULL) sont exclus du web.
    // Le chemin bot/interne (pas de RoleContext) n'est PAS filtre.
    let tickets = match rbac.as_ref() {
        None => tickets,
        Some(Extension(ctx)) => {
            if state
                .superadmin_user_ids
                .iter()
                .any(|sid| sid == &ctx.discord_user_id)
            {
                tickets
            } else {
                let allowed = state
                    .tickets_uc
                    .moderated_guilds(&ctx.discord_user_id)
                    .await?;
                tickets
                    .into_iter()
                    .filter(|t| t.guild_id.as_ref().is_some_and(|g| allowed.contains(g)))
                    .collect()
            }
        }
    };
    Ok(map_to_dtos(tickets))
}

pub async fn get_ticket_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<TicketDetailDto>, ApiError> {
    require_ticket_web(&state, &rbac, &id).await?;
    let detail = state.tickets_uc.get_ticket_detail(&id).await?;
    Ok(single_dto(detail))
}

pub async fn create_ticket(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<CreateTicketDto>,
) -> Result<Json<TicketResponseDto>, ApiError> {
    // Validation
    validation::validate_title(&dto.title).map_err(ApiError)?;

    let mut command: crate::ports::inbound::system::manage_tickets::CreateTicketCommand =
        dto.into();

    // S1/S4 — chemin web : la creation HTTP exige Moderator+ sur la guild cible,
    // et l'auteur est DERIVE du principal authentifie (on n'autorise pas un
    // `author_id` arbitraire dans le body -> anti-impersonation). Le chemin
    // bot/interne (gRPC, qui pose legitimement author = l'utilisateur Discord)
    // reste inchange.
    if let Some(Extension(ctx)) = rbac.as_ref() {
        let Some(gid) = command.guild_id.clone() else {
            return Err(ApiError(DomainError::Forbidden(
                "guild_id requis pour creer un ticket via le web".into(),
            )));
        };
        check_role_for_guild(
            &state,
            &rbac,
            &gid,
            Role::Moderator,
            "moderator+ requis pour creer un ticket",
        )
        .await?;
        command.author_id = ctx.discord_user_id.clone();
    }

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
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<ReplyDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // S1 autorisation + S4 identite. Web : on derive `author_name`/`author_role`
    // du principal REEL (le body est ignore pour ces champs) -> impossible de
    // se faire passer pour un "admin" via un JSON forge. Bot/interne : on garde
    // les valeurs du body (vraies perms Discord).
    let (author_name, author_role) = match require_ticket_web(&state, &rbac, &id).await? {
        None => (dto.author_name, dto.author_role),
        Some((_gid, role)) => {
            // RoleContext garanti present sur ce chemin.
            let principal = rbac
                .as_ref()
                .map(|Extension(c)| c.discord_user_id.clone())
                .unwrap_or_default();
            let derived_role = if role >= Role::Admin {
                "admin"
            } else if role >= Role::Moderator {
                "moderator"
            } else {
                "user"
            };
            (principal, derived_role.to_string())
        }
    };

    let broadcast_name = author_name.clone();

    state
        .tickets_uc
        .reply_ticket(ReplyTicketCommand {
            ticket_id: id.clone(),
            content: dto.content,
            author_name,
            author_role,
        })
        .await?;

    state.broadcaster.broadcast(
        "ticket_message",
        serde_json::json!({
            "ticket_id": &id,
            "author_name": &broadcast_name,
        }),
    );

    Ok(ok_response())
}

pub async fn close_ticket(
    State(state): State<AppState>,
    Path(id): Path<String>,
    rbac: Option<Extension<RoleContext>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_ticket_web(&state, &rbac, &id).await?;
    state.tickets_uc.close_ticket(&id).await?;

    // Phase 2 sync : enrichi avec `action_id` (= ticket_id parse en UUID)
    // pour que le bot puisse retrouver le mapping discord_action_messages
    // et lock le channel Discord. Format aligne sur SYNC_DISCORD_WEB_DESIGN.md.
    let action_id = uuid::Uuid::parse_str(&id).ok();
    state.broadcaster.broadcast(
        "ticket_closed",
        serde_json::json!({
            "ticket_id": &id,
            "action_id": action_id,
            "actor": { "source": "web" },
        }),
    );

    Ok(ok_response())
}

pub async fn assign_ticket(
    State(state): State<AppState>,
    Path(id): Path<String>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<AssignDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_ticket_web(&state, &rbac, &id).await?;
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
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<UpdateStatusDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_ticket_web(&state, &rbac, &id).await?;
    let status = match TicketStatus::from_str(&dto.status) {
        Some(s) => s,
        None => {
            return Err(DomainError::ValidationError(format!(
                "Statut invalide : {}. Valeurs acceptees : {:?}",
                dto.status,
                TicketStatus::VALID_VALUES
            ))
            .into())
        }
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
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<UpdateTicketChannelDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_ticket_web(&state, &rbac, &id).await?;
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

    let deleted = state
        .tickets_uc
        .bulk_delete_tickets(params.author_id.as_deref(), from_dt, to_dt)
        .await?;
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
