//! Endpoints des bans de verification d'age.
//!
//! - POST /api/age-bans            : le bot enregistre un ban (apres avoir
//!   banni le membre sur Discord) pour que le worker puisse le lever a terme.
//! - GET  /api/age-bans/due        : le worker liste les bans echus a lever.
//! - POST /api/age-bans/{id}/lift  : le worker marque un ban comme leve.

use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::{check_role, check_role_for_guild, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::entities::community::age_ban::{AgeBan, AgeBanStatus};

#[derive(Deserialize)]
pub struct CreateAgeBanDto {
    pub guild_id: String,
    pub user_id: String,
    pub declared_age: i32,
    /// Date a partir de laquelle on debannit (RFC3339).
    pub unban_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct AgeBanDto {
    pub id: String,
    pub guild_id: String,
    pub user_id: String,
    pub declared_age: i32,
    pub unban_at: String,
}

impl From<AgeBan> for AgeBanDto {
    fn from(b: AgeBan) -> Self {
        Self {
            id: b.id.to_string(),
            guild_id: b.guild_id,
            user_id: b.user_id,
            declared_age: b.declared_age,
            unban_at: b.unban_at.to_rfc3339(),
        }
    }
}

/// POST /api/age-bans
pub async fn create_age_ban(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<CreateAgeBanDto>,
) -> Result<Json<AgeBanDto>, ApiError> {
    // Moderation : reserve moderator+ (le bot passe en Internal -> bypass). Avant :
    // aucun RBAC -> ecriture IDOR d'un ban pour n'importe quelle guilde.
    check_role_for_guild(
        &state,
        &rbac,
        &dto.guild_id,
        Role::Moderator,
        "moderator+ requis pour enregistrer un age-ban",
    )
    .await?;
    let ban = AgeBan {
        id: Uuid::new_v4(),
        guild_id: dto.guild_id,
        user_id: dto.user_id,
        declared_age: dto.declared_age,
        banned_at: Utc::now(),
        unban_at: dto.unban_at,
        status: AgeBanStatus::Pending,
        lifted_at: None,
    };
    state.age_ban_repo.create(&ban).await?;
    Ok(Json(ban.into()))
}

#[derive(Deserialize)]
pub struct DueQuery {
    pub limit: Option<i64>,
}

/// GET /api/age-bans/due — bans echus a lever (worker).
pub async fn list_due_age_bans(
    State(state): State<AppState>,
    Query(q): Query<DueQuery>,
) -> Result<Json<Vec<AgeBanDto>>, ApiError> {
    let limit = q.limit.unwrap_or(200).clamp(1, 1000);
    let list = state.age_ban_repo.list_due(limit).await?;
    Ok(Json(list.into_iter().map(AgeBanDto::from).collect()))
}

/// POST /api/age-bans/{id}/lift — marque un ban comme leve (worker).
pub async fn lift_age_ban(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Pas de guild dans le path (le worker Internal appelle ceci) : un appelant
    // web ne doit pas pouvoir lever un ban -> Owner requis ; le worker (pas de
    // RoleContext) passe. Avant : aucun RBAC -> contournement de moderation.
    check_role(&rbac, Role::Owner, "owner requis pour lever un age-ban")?;
    state.age_ban_repo.mark_lifted(id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
