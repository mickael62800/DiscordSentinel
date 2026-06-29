//! GET/PUT/DELETE /api/rbac/component-min-role/{guild_id} â€” gestion des
//! overrides du min_role par composant sensible (db.purge.*, db.reset.*).
//!
//! Lecture : ouverte aux Admin+ (visualiser la config).
//! Ecriture : Owner+ (la config c'est de la securite).

use crate::adapters::inbound::http::errors_helpers::sqlx_internal;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::component_gates;
use crate::adapters::inbound::http::middleware::rbac::require_role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::errors::DomainError;

#[derive(Debug, Serialize)]
pub struct GateInfoDto {
    pub component_key: String,
    pub default_role: String,
    pub floor_role: String,
    /// Role effectif applique pour cette guild (override si present, sinon default).
    pub effective_role: String,
    /// Override explicite stocke en DB (None si default).
    pub override_role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetMinRoleDto {
    pub component_key: String,
    pub min_role: String,
}

fn forbid(s: StatusCode, msg: &str) -> ApiError {
    ApiError(if s == StatusCode::FORBIDDEN {
        DomainError::Forbidden(msg.into())
    } else {
        DomainError::Internal(msg.into())
    })
}

/// GET â€” liste les gates avec leur etat effectif pour cette guild.
pub async fn list_min_roles(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<GateInfoDto>>, ApiError> {
    require_role(&ctx, Role::Admin)
        .map_err(|s| forbid(s, "admin+ requis pour lister les gates"))?;
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;

    let overrides: Vec<(String, String)> = sqlx::query_as(
        "SELECT component_key, min_role FROM rbac_component_min_role \
         WHERE guild_id = $1",
    )
    .bind(&guild_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(sqlx_internal("db"))?;

    let mut out = Vec::new();
    for (key, def) in component_gates::list_gates() {
        let override_role = overrides
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, r)| r.clone());
        let effective_role = override_role
            .as_deref()
            .and_then(Role::from_str)
            .map(|r| if r < def.floor { def.floor } else { r })
            .unwrap_or(def.default_role);

        out.push(GateInfoDto {
            component_key: key.to_string(),
            default_role: def.default_role.as_str().to_string(),
            floor_role: def.floor.as_str().to_string(),
            effective_role: effective_role.as_str().to_string(),
            override_role,
        });
    }
    Ok(Json(out))
}

/// PUT â€” upsert d'un override (component_key + min_role). Le min_role est
/// clamp au floor du registry cote API au moment du gate.
pub async fn upsert_min_role(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<SetMinRoleDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&ctx, Role::Owner)
        .map_err(|s| forbid(s, "owner+ requis pour modifier la config RBAC"))?;
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;

    if Role::from_str(&dto.min_role).is_none() {
        return Err(ApiError(DomainError::ValidationError(format!(
            "min_role invalide: {}",
            dto.min_role
        ))));
    }
    // On verifie que la cle est connue (sinon override inutile).
    let known_keys: Vec<&'static str> = component_gates::list_gates()
        .into_iter()
        .map(|(k, _)| k)
        .collect();
    if !known_keys.contains(&dto.component_key.as_str()) {
        return Err(ApiError(DomainError::ValidationError(format!(
            "component_key inconnu: {}",
            dto.component_key
        ))));
    }

    sqlx::query(
        "INSERT INTO rbac_component_min_role \
             (guild_id, component_key, min_role, updated_at, updated_by) \
         VALUES ($1, $2, $3, NOW(), $4) \
         ON CONFLICT (guild_id, component_key) DO UPDATE SET \
             min_role = EXCLUDED.min_role, \
             updated_at = NOW(), \
             updated_by = EXCLUDED.updated_by",
    )
    .bind(&guild_id)
    .bind(&dto.component_key)
    .bind(&dto.min_role)
    .bind(&ctx.discord_user_id)
    .execute(&state.pg_pool)
    .await
    .map_err(sqlx_internal("upsert"))?;

    component_gates::invalidate_cache(&state, &guild_id, &dto.component_key).await;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE â€” supprime l'override (retour au default).
pub async fn delete_min_role(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    Path((guild_id, component_key)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&ctx, Role::Owner)
        .map_err(|s| forbid(s, "owner+ requis pour supprimer l'override"))?;
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;

    sqlx::query(
        "DELETE FROM rbac_component_min_role \
         WHERE guild_id = $1 AND component_key = $2",
    )
    .bind(&guild_id)
    .bind(&component_key)
    .execute(&state.pg_pool)
    .await
    .map_err(sqlx_internal("delete"))?;

    component_gates::invalidate_cache(&state, &guild_id, &component_key).await;

    Ok(Json(serde_json::json!({ "ok": true })))
}
