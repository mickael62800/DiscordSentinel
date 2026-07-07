//! Phase 7 B — Endpoints CRUD RBAC.
//!
//! Permettent a un `owner` de gerer les roles applicatifs de sa guild sans
//! passer par SQL direct. Tous les endpoints d'ecriture sont gates par
//! `require_role(Role::Owner)`. La lecture de liste est ouverte a `Admin+`
//! (visibilite). `/me/{guild_id}` est accessible a tout role (y compris
//! `viewer`) pour permettre au desktop de savoir quoi afficher.
//!
//! Pattern hexagonal : le handler reste mince (gate RBAC + parse DTO + map),
//! toute la persistance et les garde-fous metier passent par le use case
//! `rbac_admin_uc` (`ManageRbacUseCase`). Le SQL vit dans l'adapter Postgres
//! `RbacRepository`. Distinct du middleware RBAC (`middleware/rbac.rs`).

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::{ValidatedGuild, ValidatedGuildUser};
use crate::adapters::inbound::http::middleware::rbac::require_role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use sentinel_core::domain::entities::system::discord_ids::GuildId;
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::ports::inbound::system::manage_rbac::{
    GrantRoleCommand, RevokeRoleCommand, UpdateRoleCommand,
};
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct GrantRoleDto {
    pub role: String,
    /// Nom d'affichage pour la ligne api_users (seedee a la premiere attribution)
    #[serde(default)]
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleDto {
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct UserRoleDto {
    pub discord_user_id: String,
    pub guild_id: GuildId,
    pub role: String,
    pub granted_at: String,
    pub granted_by: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GuildUserEntryDto {
    pub discord_user_id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub granted_at: String,
    pub granted_by: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MyRoleDto {
    pub discord_user_id: String,
    pub guild_id: GuildId,
    pub role: String,
    /// True si l'utilisateur figure dans SUPERADMIN_USER_IDS — bypass total.
    #[serde(default)]
    pub is_superadmin: bool,
}

/// POST /api/rbac/guilds/{guild_id}/users/{user_id}
///
/// Grant un role a un user pour une guild. Gated `Owner`.
pub async fn grant_role(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
    Json(dto): Json<GrantRoleDto>,
) -> Result<Json<UserRoleDto>, ApiError> {
    require_role(&ctx, Role::Owner).map_err(|s| status_to_err(s, "owner requis pour grant"))?;

    let role = parse_role(&dto.role)?;

    let grant = state
        .rbac_admin_uc
        .grant_role(GrantRoleCommand {
            guild_id: guild_id.clone(),
            user_id,
            role,
            granted_by: ctx.discord_user_id,
            display_name: dto.display_name,
        })
        .await
        .map_err(ApiError)?;

    Ok(Json(UserRoleDto {
        discord_user_id: grant.discord_user_id,
        guild_id: guild_id.into(),
        role: grant.role,
        granted_at: grant.granted_at.to_rfc3339(),
        granted_by: grant.granted_by,
    }))
}

/// PATCH /api/rbac/guilds/{guild_id}/users/{user_id}
///
/// Modifie le role d'un user existant. Gated `Owner`.
pub async fn update_role(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
    Json(dto): Json<UpdateRoleDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&ctx, Role::Owner).map_err(|s| status_to_err(s, "owner requis pour update"))?;

    let role = parse_role(&dto.role)?;

    state
        .rbac_admin_uc
        .update_role(UpdateRoleCommand {
            guild_id,
            user_id,
            caller_id: ctx.discord_user_id,
            role,
        })
        .await
        .map_err(ApiError)?;

    Ok(Json(
        serde_json::json!({ "ok": true, "role": role.as_str() }),
    ))
}

/// DELETE /api/rbac/guilds/{guild_id}/users/{user_id}
///
/// Revoque le role d'un user. Gated `Owner`. Garde-fou : on ne peut pas
/// supprimer le dernier owner d'une guild.
pub async fn revoke_role(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&ctx, Role::Owner).map_err(|s| status_to_err(s, "owner requis pour revoke"))?;

    state
        .rbac_admin_uc
        .revoke_role(RevokeRoleCommand { guild_id, user_id })
        .await
        .map_err(ApiError)?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /api/rbac/guilds/{guild_id}/users
///
/// Liste les users ayant un role sur une guild. Gated `Admin+`.
pub async fn list_guild_users(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<GuildUserEntryDto>>, ApiError> {
    require_role(&ctx, Role::Admin).map_err(|s| status_to_err(s, "admin+ requis pour lister"))?;

    let rows = state
        .rbac_admin_uc
        .list_guild_users(&guild_id)
        .await
        .map_err(ApiError)?;

    Ok(Json(
        rows.into_iter()
            .map(|r| GuildUserEntryDto {
                discord_user_id: r.discord_user_id,
                display_name: r.display_name,
                avatar_url: r.avatar_url,
                role: r.role,
                granted_at: r.granted_at.to_rfc3339(),
                granted_by: r.granted_by,
            })
            .collect(),
    ))
}

/// GET /api/rbac/me/{guild_id}
///
/// Retourne le role effectif du caller sur la guild courante. Ouvert a tout
/// role (y compris viewer) — pas besoin de `require_role`. Le desktop l'utilise
/// pour savoir quoi afficher (masquer les boutons admin si viewer, etc.).
pub async fn get_my_role(
    State(state): State<AppState>,
    rbac: Option<axum::Extension<RoleContext>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<MyRoleDto>, ApiError> {
    // Si pas de RoleContext (middleware n'a pas pu resoudre = pas auth), 401.
    let Some(axum::Extension(ctx)) = rbac else {
        return Err(ApiError(
            sentinel_core::domain::errors::DomainError::Forbidden("auth Discord requise".into()),
        ));
    };

    let is_super = state
        .superadmin_user_ids
        .iter()
        .any(|id| id == &ctx.discord_user_id);

    let role = match ctx.role {
        Some(r) => r,
        None if is_super => Role::Owner,
        None => {
            return Err(ApiError(
                sentinel_core::domain::errors::DomainError::NotFound(
                    "pas de role pour ce guild (endpoint necessite X-Discord-Token)".into(),
                ),
            ));
        }
    };

    Ok(Json(MyRoleDto {
        discord_user_id: ctx.discord_user_id,
        guild_id: guild_id.into(),
        role: role.as_str().to_string(),
        is_superadmin: is_super,
    }))
}

// ─────────────── helpers ───────────────

fn parse_role(s: &str) -> Result<Role, ApiError> {
    Role::from_str(s).ok_or_else(|| {
        ApiError(sentinel_core::domain::errors::DomainError::ValidationError(
            format!("role invalide: {s} (attendu: owner|admin|moderator|viewer)"),
        ))
    })
}

fn status_to_err(status: StatusCode, context: &str) -> ApiError {
    match status {
        StatusCode::FORBIDDEN => ApiError(sentinel_core::domain::errors::DomainError::Forbidden(
            context.to_string(),
        )),
        _ => ApiError(sentinel_core::domain::errors::DomainError::Internal(
            format!("rbac gate failed: {status}"),
        )),
    }
}

#[cfg(test)]
#[path = "tests/rbac.rs"]
mod tests;
