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

    sqlx::query(
        "INSERT INTO audit_logs (guild_id, event_type, target_id, target_name, details) \
         VALUES ($1, 'member_nickname_history', $2, $3, $4)",
    )
    .bind(&dto.guild_id)
    .bind(&dto.user_id)
    .bind(&dto.new_name)
    .bind(serde_json::json!({
        "old_name": dto.old_name,
        "new_name": dto.new_name,
    }))
    .execute(&state.pg_pool)
    .await
    .inspect_err(|e| warn!(error = %e, user_id = %dto.user_id, "Echec insert name_history"))
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
    // Construire le SET dynamiquement selon les champs presents
    if let Some(ref fr) = dto.first_response_at {
        sqlx::query("UPDATE tickets SET first_response_at = $1, updated_at = NOW() WHERE id = $2::uuid")
            .bind(fr)
            .bind(&id)
            .execute(&state.pg_pool)
            .await
            .inspect_err(|e| warn!(error = %e, ticket_id = %id, "Echec update first_response_at"))
            .ok();
    }

    if let Some(ref ra) = dto.resolved_at {
        sqlx::query("UPDATE tickets SET resolved_at = $1, updated_at = NOW() WHERE id = $2::uuid")
            .bind(ra)
            .bind(&id)
            .execute(&state.pg_pool)
            .await
            .inspect_err(|e| warn!(error = %e, ticket_id = %id, "Echec update resolved_at"))
            .ok();
    }

    if let Some(rating) = dto.satisfaction_rating {
        sqlx::query("UPDATE tickets SET satisfaction_rating = $1, updated_at = NOW() WHERE id = $2::uuid")
            .bind(rating)
            .bind(&id)
            .execute(&state.pg_pool)
            .await
            .inspect_err(|e| warn!(error = %e, ticket_id = %id, "Echec update satisfaction_rating"))
            .ok();
    }

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

    sqlx::query(
        "INSERT INTO sponsorships (guild_id, sponsor_id, sponsored_id) \
         VALUES ($1, $2, $3) ON CONFLICT (guild_id, sponsored_id) DO NOTHING",
    )
    .bind(&dto.guild_id)
    .bind(&dto.sponsor_id)
    .bind(&dto.sponsored_id)
    .execute(&state.pg_pool)
    .await
    .inspect_err(|e| warn!(error = %e, guild_id = %dto.guild_id, "Echec insert sponsorship"))
    .ok();

    Ok(ok_response())
}

/// GET /api/sponsorships/{guild_id}
pub async fn list_sponsorships(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<SponsorshipRow>>, ApiError> {
    // Validation
    validation::validate_guild_id_path(&guild_id).map_err(ApiError)?;

    let rows = sqlx::query_as::<_, SponsorshipRow>(
        "SELECT id, guild_id, sponsor_id, sponsored_id, created_at \
         FROM sponsorships WHERE guild_id = $1 ORDER BY created_at DESC",
    )
    .bind(&guild_id)
    .fetch_all(&state.pg_pool)
    .await
    .unwrap_or_else(|e| {
        warn!(error = %e, guild_id = %guild_id, "Echec SELECT sponsorships");
        vec![]
    });

    Ok(Json(rows))
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

    sqlx::query(
        "INSERT INTO temp_roles (guild_id, user_id, role_id, expires_at) \
         VALUES ($1, $2, $3, $4::timestamptz) \
         ON CONFLICT (guild_id, user_id, role_id) DO UPDATE SET expires_at = $4::timestamptz",
    )
    .bind(&dto.guild_id)
    .bind(&dto.user_id)
    .bind(&dto.role_id)
    .bind(&dto.expires_at)
    .execute(&state.pg_pool)
    .await
    .inspect_err(|e| warn!(error = %e, guild_id = %dto.guild_id, "Echec insert temp_role"))
    .ok();

    Ok(ok_response())
}

/// GET /api/temp-roles/{guild_id}
pub async fn list_temp_roles(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<TempRoleRow>>, ApiError> {
    // Validation
    validation::validate_guild_id_path(&guild_id).map_err(ApiError)?;

    let rows = sqlx::query_as::<_, TempRoleRow>(
        "SELECT id, guild_id, user_id, role_id, expires_at, created_at \
         FROM temp_roles WHERE guild_id = $1 AND expires_at > NOW() \
         ORDER BY expires_at ASC",
    )
    .bind(&guild_id)
    .fetch_all(&state.pg_pool)
    .await
    .unwrap_or_else(|e| {
        warn!(error = %e, guild_id = %guild_id, "Echec SELECT temp_roles");
        vec![]
    });

    Ok(Json(rows))
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

    sqlx::query(
        "DELETE FROM temp_roles WHERE guild_id = $1 AND user_id = $2 AND role_id = $3",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(&role_id)
    .execute(&state.pg_pool)
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

    let result = sqlx::query_scalar::<_, sqlx::types::Uuid>(
        "INSERT INTO pending_mod_actions \
         (guild_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, gravity, duration) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         RETURNING id",
    )
    .bind(&dto.guild_id)
    .bind(&dto.moderator_id)
    .bind(&dto.moderator_name)
    .bind(&dto.target_id)
    .bind(&dto.target_name)
    .bind(&dto.action_type)
    .bind(&dto.reason)
    .bind(&dto.gravity)
    .bind(dto.duration)
    .fetch_one(&state.pg_pool)
    .await;

    match result {
        Ok(id) => Ok(Json(serde_json::json!({ "id": id.to_string() }))),
        Err(e) => {
            warn!(error = %e, guild_id = %dto.guild_id, target_id = %dto.target_id, "Echec creation pending_action");
            Ok(ok_response())
        }
    }
}

/// GET /api/moderation/pending/{guild_id}
pub async fn list_pending_actions(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<PendingActionRow>>, ApiError> {
    // Validation
    validation::validate_guild_id_path(&guild_id).map_err(ApiError)?;

    let rows = sqlx::query_as::<_, PendingActionRow>(
        "SELECT id, guild_id, moderator_id, moderator_name, target_id, target_name, \
         action_type, reason, gravity, duration, status, reviewed_by, created_at, updated_at \
         FROM pending_mod_actions WHERE guild_id = $1 AND status = 'pending' \
         ORDER BY created_at DESC",
    )
    .bind(&guild_id)
    .fetch_all(&state.pg_pool)
    .await
    .unwrap_or_else(|e| {
        warn!(error = %e, guild_id = %guild_id, "Echec SELECT pending_mod_actions");
        vec![]
    });

    Ok(Json(rows))
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
    if rbac.is_some() {
        let gid: Option<(String,)> = sqlx::query_as(
            "SELECT guild_id FROM pending_mod_actions WHERE id = $1::uuid",
        )
        .bind(&id)
        .fetch_optional(&state.pg_pool)
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("fetch pending guild_id: {e}"))))?;
        if let Some((guild_id,)) = gid {
            check_role_for_guild(
                &state,
                &rbac,
                &guild_id,
                Role::Moderator,
                "moderator+ requis pour resoudre une action en attente",
            )
            .await?;
        }
    }

    sqlx::query(
        "UPDATE pending_mod_actions SET status = $1, reviewed_by = $2, updated_at = NOW() \
         WHERE id = $3::uuid",
    )
    .bind(&dto.status)
    .bind(&dto.reviewed_by)
    .bind(&id)
    .execute(&state.pg_pool)
    .await
    .inspect_err(|e| warn!(error = %e, action_id = %id, "Echec resolution pending_action"))
    .ok();

    Ok(ok_response())
}
