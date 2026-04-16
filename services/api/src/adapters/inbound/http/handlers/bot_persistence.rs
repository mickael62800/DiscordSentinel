//! Handlers pour la persistance des donnees fire-and-forget des bots.
//! Ces endpoints sont appeles par les bots pour persister des donnees
//! qui etaient auparavant uniquement en memoire (DashMap).
//!
//! Approche pragmatique : sqlx direct depuis le handler (pas de full hexagonal)
//! car ces endpoints sont simples et fire-and-forget cote bot.

use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::Deserialize;

use tracing::warn;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::middleware::rbac::{check_role_for_guild, require_role, Role, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use crate::domain::errors::DomainError;

// ═══════════════════════════════════════════════════
// Name History (Audit Bot)
// ═══════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct CreateNameHistoryDto {
    pub guild_id: String,
    pub user_id: String,
    pub old_name: String,
    pub new_name: String,
}

/// POST /api/name-history
pub async fn create_name_history(
    State(state): State<AppState>,
    Json(dto): Json<CreateNameHistoryDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation
    validation::validate_guild_user_path(&dto.guild_id, &dto.user_id).map_err(ApiError)?;

    state.audit_logs_uc.create(crate::ports::inbound::CreateAuditLogCommand {
        guild_id: dto.guild_id,
        event_type: "member_nickname_history".into(),
        actor_id: None,
        actor_name: None,
        target_id: Some(dto.user_id.clone()),
        target_name: Some(dto.new_name.clone()),
        channel_id: None,
        channel_name: None,
        details: serde_json::json!({
            "old_name": dto.old_name,
            "new_name": dto.new_name,
        }),
    }).await
    .inspect_err(|e| warn!(error = %e, "Echec insert name_history"))
    .ok();

    Ok(ok_response())
}

// ═══════════════════════════════════════════════════
// Streaks (Progression Bot)
// ═══════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct UpdateStreakDto {
    pub streak_current: i32,
    pub streak_best: i32,
    pub streak_last_day: i32,
    pub streak_last_year: i32,
}

/// PATCH /api/levels/{guild_id}/{user_id}/streak
pub async fn update_streak(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<UpdateStreakDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation
    validation::validate_guild_user_path(&guild_id, &user_id).map_err(ApiError)?;

    sqlx::query(
        "UPDATE user_levels SET streak_current = $1, streak_best = $2, \
         streak_last_day = $3, streak_last_year = $4, updated_at = NOW() \
         WHERE guild_id = $5 AND user_id = $6",
    )
    .bind(dto.streak_current)
    .bind(dto.streak_best)
    .bind(dto.streak_last_day)
    .bind(dto.streak_last_year)
    .bind(&guild_id)
    .bind(&user_id)
    .execute(&state.pg_pool)
    .await
    .inspect_err(|e| warn!(error = %e, guild_id = %guild_id, user_id = %user_id, "Echec update streak"))
    .ok();

    Ok(ok_response())
}

// ═══════════════════════════════════════════════════
// SLA Tickets (Ticket Bot)
// ═══════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct UpdateTicketSlaDto {
    pub first_response_at: Option<String>,
    pub resolved_at: Option<String>,
    pub satisfaction_rating: Option<i32>,
}

/// PATCH /api/tickets/{id}/sla
pub async fn update_ticket_sla(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateTicketSlaDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let uuid = uuid::Uuid::parse_str(&id)
        .map_err(|_| ApiError(DomainError::ValidationError("ticket id invalide".into())))?;
    state.tickets_uc
        .update_sla(uuid, dto.first_response_at.as_deref(), dto.resolved_at.as_deref(), dto.satisfaction_rating)
        .await
        .inspect_err(|e| warn!(error = %e, ticket_id = %id, "Echec update_sla"))
        .ok();

    Ok(ok_response())
}

// ═══════════════════════════════════════════════════
// Sponsorships (Community Bot)
// ═══════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct CreateSponsorshipDto {
    pub guild_id: String,
    pub sponsor_id: String,
    pub sponsored_id: String,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SponsorshipRow {
    pub id: sqlx::types::Uuid,
    pub guild_id: String,
    pub sponsor_id: String,
    pub sponsored_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// POST /api/sponsorships
pub async fn create_sponsorship(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<CreateSponsorshipDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("sponsor_id", &dto.sponsor_id).map_err(ApiError)?;
    validation::validate_discord_id("sponsored_id", &dto.sponsored_id).map_err(ApiError)?;

    // C4 — Gate RBAC : moderator+ requis pour creer un parrainage.
    // Pass-through pour les appels bot-internal (rbac absent).
    check_role_for_guild(
        &state, &rbac, &dto.guild_id, Role::Moderator,
        "moderator+ requis pour creer un parrainage",
    )
    .await?;

    state.sponsorship_repo
        .create(&dto.guild_id, &dto.sponsor_id, &dto.sponsored_id)
        .await
        .inspect_err(|e| warn!(error = %e, guild_id = %dto.guild_id, "Echec insert sponsorship"))
        .ok();

    Ok(ok_response())
}

/// GET /api/sponsorships/{guild_id}
pub async fn list_sponsorships(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<crate::ports::outbound::Sponsorship>>, ApiError> {
    // Validation
    validation::validate_guild_id_path(&guild_id).map_err(ApiError)?;

    let entries = state.sponsorship_repo.list(&guild_id).await.unwrap_or_else(|e| {
        warn!(error = %e, guild_id = %guild_id, "Echec list sponsorships");
        vec![]
    });

    Ok(Json(entries))
}

// ═══════════════════════════════════════════════════
// Temp Roles (Community Bot)
// ═══════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct CreateTempRoleDto {
    pub guild_id: String,
    pub user_id: String,
    pub role_id: String,
    pub expires_at: String,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct TempRoleRow {
    pub id: sqlx::types::Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub role_id: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// POST /api/temp-roles
pub async fn create_temp_role(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<CreateTempRoleDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &dto.user_id).map_err(ApiError)?;
    validation::validate_discord_id("role_id", &dto.role_id).map_err(ApiError)?;

    // C5 — Gate RBAC : moderator+ requis pour assigner un role temporaire.
    check_role_for_guild(
        &state, &rbac, &dto.guild_id, Role::Moderator,
        "moderator+ requis pour creer un temp_role",
    )
    .await?;

    state.temp_role_repo
        .create(&dto.guild_id, &dto.user_id, &dto.role_id, &dto.expires_at)
        .await
        .inspect_err(|e| warn!(error = %e, guild_id = %dto.guild_id, "Echec insert temp_role"))
        .ok();

    Ok(ok_response())
}

/// GET /api/temp-roles/{guild_id}
pub async fn list_temp_roles(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<crate::ports::outbound::TempRole>>, ApiError> {
    // Validation
    validation::validate_guild_id_path(&guild_id).map_err(ApiError)?;

    let entries = state.temp_role_repo.list_active(&guild_id).await.unwrap_or_else(|e| {
        warn!(error = %e, guild_id = %guild_id, "Echec list temp_roles");
        vec![]
    });

    Ok(Json(entries))
}

/// DELETE /api/temp-roles/{guild_id}/{user_id}/{role_id}
pub async fn delete_temp_role(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id, role_id)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &user_id).map_err(ApiError)?;
    validation::validate_discord_id("role_id", &role_id).map_err(ApiError)?;

    // Phase 7 B — Gate RBAC : moderator+ requis depuis le desktop. Les bots
    // (community-bot qui consume l'event temp_role_expire) appellent sans
    // X-Discord-Token → pass-through non-breaking.
    if let Some(Extension(ctx)) = rbac {
        require_role(&ctx, Role::Moderator)
            .map_err(|_| ApiError(DomainError::Forbidden("moderator+ requis pour supprimer un temp role".into())))?;
    }

    state.temp_role_repo
        .delete(&guild_id, &user_id, &role_id)
        .await
        .inspect_err(|e| warn!(error = %e, guild_id = %guild_id, "Echec delete temp_role"))
        .ok();

    Ok(ok_response())
}

// ═══════════════════════════════════════════════════
// Pending Moderation Actions (Moderation Bot - Mode Apprenti)
// ═══════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct CreatePendingActionDto {
    pub guild_id: String,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub gravity: Option<String>,
    pub duration: Option<i64>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct PendingActionRow {
    pub id: sqlx::types::Uuid,
    pub guild_id: String,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub gravity: Option<String>,
    pub duration: Option<i64>,
    pub status: String,
    pub reviewed_by: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// POST /api/moderation/pending
pub async fn create_pending_action(
    State(state): State<AppState>,
    Json(dto): Json<CreatePendingActionDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation
    validation::validate_moderation_action(
        &dto.guild_id, &dto.moderator_id, &dto.target_id, &dto.reason, &dto.action_type,
    ).map_err(ApiError)?;

    match state.pending_action_repo.create(
        &dto.guild_id, &dto.moderator_id, &dto.moderator_name,
        &dto.target_id, &dto.target_name, &dto.action_type,
        &dto.reason, dto.gravity.as_deref(), dto.duration,
    ).await {
        Ok(id) => Ok(Json(serde_json::json!({ "id": id.to_string() }))),
        Err(e) => {
            warn!(error = %e, guild_id = %dto.guild_id, "Echec creation pending_action");
            Ok(ok_response())
        }
    }
}

/// GET /api/moderation/pending/{guild_id}
pub async fn list_pending_actions(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<crate::ports::outbound::PendingAction>>, ApiError> {
    // Validation
    validation::validate_guild_id_path(&guild_id).map_err(ApiError)?;

    let entries = state.pending_action_repo.list_pending(&guild_id).await.unwrap_or_else(|e| {
        warn!(error = %e, guild_id = %guild_id, "Echec list pending_mod_actions");
        vec![]
    });

    Ok(Json(entries))
}

#[derive(Debug, Deserialize)]
pub struct ResolvePendingActionDto {
    pub status: String,
    pub reviewed_by: String,
}

/// PATCH /api/moderation/pending/{id}
pub async fn resolve_pending_action(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<String>,
    Json(dto): Json<ResolvePendingActionDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // H10 — Revérif permission serveur : on lookup le guild_id de l'action
    // pending puis on gate sur Moderator+.
    let uuid = uuid::Uuid::parse_str(&id)
        .map_err(|_| ApiError(DomainError::ValidationError("id invalide".into())))?;

    if rbac.is_some() {
        if let Some(guild_id) = state.pending_action_repo.get_guild_id(uuid).await? {
            check_role_for_guild(&state, &rbac, &guild_id, Role::Moderator, "moderator+ requis pour resoudre une action en attente").await?;
        }
    }

    state.pending_action_repo
        .resolve(uuid, &dto.status, &dto.reviewed_by)
        .await
        .inspect_err(|e| warn!(error = %e, action_id = %id, "Echec resolution pending_action"))
        .ok();

    Ok(ok_response())
}
