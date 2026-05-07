//! Phase 7 B — Endpoints CRUD RBAC.
//!
//! Permettent a un `owner` de gerer les roles applicatifs de sa guild sans
//! passer par SQL direct. Tous les endpoints d'ecriture sont gates par
//! `require_role(Role::Owner)`. La lecture de liste est ouverte a `Admin+`
//! (visibilite). `/me/{guild_id}` est accessible a tout role (y compris
//! `viewer`) pour permettre au desktop de savoir quoi afficher.
//!
//! Pattern : direct sqlx (comme `bot_persistence.rs`, `rbac` simple, pas de
//! logique metier complexe, pas besoin de use-case).

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::require_role;
use sentinel_core::domain::enums::system::role::Role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use sentinel_core::domain::entities::system::discord_ids::GuildId;

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
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<GrantRoleDto>,
) -> Result<Json<UserRoleDto>, ApiError> {
    require_role(&ctx, Role::Owner).map_err(|s| status_to_err(s, "owner requis pour grant"))?;
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &user_id).map_err(ApiError)?;

    let role = parse_role(&dto.role)?;
    let display_name = sentinel_core::domain::entities::system::rbac::truncate_display_name(
        dto.display_name.as_deref().unwrap_or("user"),
    );

    // Upsert api_users (garantit la FK)
    sqlx::query(
        "INSERT INTO api_users (discord_user_id, display_name) \
         VALUES ($1, $2) \
         ON CONFLICT (discord_user_id) DO NOTHING",
    )
    .bind(&user_id)
    .bind(&display_name)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| internal(format!("upsert api_users: {e}")))?;

    // Insert api_user_guilds. Si deja existe (doublon grant), on retourne 409.
    #[derive(sqlx::FromRow)]
    struct Row {
        granted_at: chrono::DateTime<chrono::Utc>,
    }

    let res: Result<Row, sqlx::Error> = sqlx::query_as::<_, Row>(
        "INSERT INTO api_user_guilds (discord_user_id, guild_id, role, granted_by) \
         VALUES ($1, $2, $3, $4) \
         RETURNING granted_at",
    )
    .bind(&user_id)
    .bind(&guild_id)
    .bind(role.as_str())
    .bind(&ctx.discord_user_id)
    .fetch_one(&state.pg_pool)
    .await;

    match res {
        Ok(row) => Ok(Json(UserRoleDto {
            discord_user_id: user_id,
            guild_id: guild_id.into(),
            role: role.as_str().to_string(),
            granted_at: row.granted_at.to_rfc3339(),
            granted_by: Some(ctx.discord_user_id),
        })),
        Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
            Err(ApiError(sentinel_core::domain::errors::DomainError::ValidationError(
                "user a deja un role sur cette guild, utiliser PATCH pour modifier".into(),
            )))
        }
        Err(e) => Err(internal(format!("insert api_user_guilds: {e}"))),
    }
}

/// PATCH /api/rbac/guilds/{guild_id}/users/{user_id}
///
/// Modifie le role d'un user existant. Gated `Owner`.
pub async fn update_role(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<UpdateRoleDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&ctx, Role::Owner).map_err(|s| status_to_err(s, "owner requis pour update"))?;
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &user_id).map_err(ApiError)?;

    // Regle metier : anti-lockout du dernier owner-caller.
    if sentinel_core::domain::entities::system::rbac::is_owner_self_demotion(&ctx.discord_user_id, &user_id, &dto.role) {
        return Err(ApiError(sentinel_core::domain::errors::DomainError::ValidationError(
            "un owner ne peut pas se retrograder (lockout risk)".into(),
        )));
    }

    let role = parse_role(&dto.role)?;

    let res = sqlx::query(
        "UPDATE api_user_guilds SET role = $1 \
         WHERE discord_user_id = $2 AND guild_id = $3",
    )
    .bind(role.as_str())
    .bind(&user_id)
    .bind(&guild_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| internal(format!("update role: {e}")))?;

    if res.rows_affected() == 0 {
        return Err(ApiError(sentinel_core::domain::errors::DomainError::NotFound(
            "user n'a pas de role sur cette guild".into(),
        )));
    }

    Ok(Json(serde_json::json!({ "ok": true, "role": role.as_str() })))
}

/// DELETE /api/rbac/guilds/{guild_id}/users/{user_id}
///
/// Revoque le role d'un user. Gated `Owner`. Garde-fou : on ne peut pas
/// supprimer le dernier owner d'une guild.
pub async fn revoke_role(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&ctx, Role::Owner).map_err(|s| status_to_err(s, "owner requis pour revoke"))?;
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &user_id).map_err(ApiError)?;

    // Garde-fou : verifier que ce n'est pas le dernier owner
    let (total_owners,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*)::bigint FROM api_user_guilds \
         WHERE guild_id = $1 AND role = 'owner'",
    )
    .bind(&guild_id)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| internal(format!("count owners: {e}")))?;

    // Si le user a supprimer est owner ET c'est le dernier → refus
    let (is_target_owner,): (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM api_user_guilds \
         WHERE discord_user_id = $1 AND guild_id = $2 AND role = 'owner')",
    )
    .bind(&user_id)
    .bind(&guild_id)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| internal(format!("check target owner: {e}")))?;

    if sentinel_core::domain::entities::system::rbac::would_revoke_last_owner(is_target_owner, total_owners) {
        return Err(ApiError(sentinel_core::domain::errors::DomainError::ValidationError(
            "impossible de revoquer le dernier owner de la guild".into(),
        )));
    }

    let res = sqlx::query(
        "DELETE FROM api_user_guilds WHERE discord_user_id = $1 AND guild_id = $2",
    )
    .bind(&user_id)
    .bind(&guild_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| internal(format!("delete role: {e}")))?;

    if res.rows_affected() == 0 {
        return Err(ApiError(sentinel_core::domain::errors::DomainError::NotFound(
            "user n'a pas de role sur cette guild".into(),
        )));
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /api/rbac/guilds/{guild_id}/users
///
/// Liste les users ayant un role sur une guild. Gated `Admin+`.
pub async fn list_guild_users(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<GuildUserEntryDto>>, ApiError> {
    require_role(&ctx, Role::Admin).map_err(|s| status_to_err(s, "admin+ requis pour lister"))?;
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;

    #[derive(sqlx::FromRow)]
    struct Row {
        discord_user_id: String,
        display_name: String,
        avatar_url: Option<String>,
        role: String,
        granted_at: chrono::DateTime<chrono::Utc>,
        granted_by: Option<String>,
    }

    let rows: Vec<Row> = sqlx::query_as::<_, Row>(
        "SELECT u.discord_user_id, u.display_name, u.avatar_url, \
                g.role, g.granted_at, g.granted_by \
         FROM api_user_guilds g \
         INNER JOIN api_users u ON u.discord_user_id = g.discord_user_id \
         WHERE g.guild_id = $1 \
         ORDER BY \
            CASE g.role \
                WHEN 'owner' THEN 0 \
                WHEN 'admin' THEN 1 \
                WHEN 'moderator' THEN 2 \
                WHEN 'viewer' THEN 3 \
            END, \
            u.display_name ASC",
    )
    .bind(&guild_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| internal(format!("list guild users: {e}")))?;

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
    Path(guild_id): Path<String>,
) -> Result<Json<MyRoleDto>, ApiError> {
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;

    // Si pas de RoleContext (middleware n'a pas pu resoudre = pas auth), 401.
    let Some(axum::Extension(ctx)) = rbac else {
        return Err(ApiError(sentinel_core::domain::errors::DomainError::Forbidden(
            "auth Discord requise".into(),
        )));
    };

    let is_super = state
        .superadmin_user_ids
        .iter()
        .any(|id| id == &ctx.discord_user_id);

    let role = match ctx.role {
        Some(r) => r,
        None if is_super => Role::Owner,
        None => {
            return Err(ApiError(sentinel_core::domain::errors::DomainError::NotFound(
                "pas de role pour ce guild (endpoint necessite X-Discord-Token)".into(),
            )));
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
        ApiError(sentinel_core::domain::errors::DomainError::ValidationError(format!(
            "role invalide: {s} (attendu: owner|admin|moderator|viewer)"
        )))
    })
}

fn internal(msg: String) -> ApiError {
    ApiError(sentinel_core::domain::errors::DomainError::Internal(msg))
}

fn status_to_err(status: StatusCode, context: &str) -> ApiError {
    match status {
        StatusCode::FORBIDDEN => ApiError(sentinel_core::domain::errors::DomainError::Forbidden(
            context.to_string(),
        )),
        _ => ApiError(sentinel_core::domain::errors::DomainError::Internal(format!(
            "rbac gate failed: {status}"
        ))),
    }
}

#[cfg(test)]
#[path = "tests/rbac.rs"]
mod tests;

