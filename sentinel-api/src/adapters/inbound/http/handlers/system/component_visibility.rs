//! GET/PUT /api/rbac/component-visibility/{guild_id} â€” gestion des overrides
//! de visibilite des composants UI par role.
//!
//! Lecture : ouverte a tout role authentifie (chaque utilisateur recupere
//! la liste pour appliquer les overrides cote front).
//! Ecriture : Owner+ (modifie ce que les modos/admins voient).

use crate::adapters::inbound::http::errors_helpers::sqlx_internal;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::require_role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::errors::DomainError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VisibilityEntryDto {
    pub component_key: String,
    pub role: String,
    pub visible: bool,
}

#[derive(Debug, Deserialize)]
pub struct VisibilityBatchDto {
    pub entries: Vec<VisibilityEntryDto>,
}

fn forbid(s: StatusCode, msg: &str) -> ApiError {
    ApiError(if s == StatusCode::FORBIDDEN {
        DomainError::Forbidden(msg.into())
    } else {
        DomainError::Internal(msg.into())
    })
}

/// GET â€” liste tous les overrides de visibilite pour la guild.
pub async fn list_visibility(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<VisibilityEntryDto>>, ApiError> {
    let rows = sqlx::query_as::<_, (String, String, bool)>(
        "SELECT component_key, role, visible \
         FROM rbac_component_visibility \
         WHERE guild_id = $1",
    )
    .bind(&guild_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(sqlx_internal("db"))?;

    let out: Vec<VisibilityEntryDto> = rows
        .into_iter()
        .map(|(component_key, role, visible)| VisibilityEntryDto {
            component_key,
            role,
            visible,
        })
        .collect();
    Ok(Json(out))
}

/// PUT â€” upsert batch de tous les overrides envoyes. Les entrees absentes
/// du payload sont conservees telles quelles (delete explicite via visibility
/// par defaut : on garde simple, l'UI envoie tout l'etat affiche).
pub async fn upsert_visibility(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(body): Json<VisibilityBatchDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&ctx, Role::Owner)
        .map_err(|s| forbid(s, "owner+ requis pour modifier la visibilite"))?;

    let valid_roles = ["viewer", "moderator", "admin", "owner"];
    for e in &body.entries {
        if !valid_roles.contains(&e.role.as_str()) {
            return Err(ApiError(DomainError::ValidationError(format!(
                "role invalide: {}",
                e.role
            ))));
        }
        if e.component_key.is_empty() || e.component_key.len() > 100 {
            return Err(ApiError(DomainError::ValidationError(
                "component_key vide ou trop long".into(),
            )));
        }
    }

    let mut tx = state.pg_pool.begin().await.map_err(sqlx_internal("tx"))?;

    for e in &body.entries {
        sqlx::query(
            "INSERT INTO rbac_component_visibility \
                 (guild_id, component_key, role, visible, updated_at, updated_by) \
             VALUES ($1, $2, $3, $4, NOW(), $5) \
             ON CONFLICT (guild_id, component_key, role) DO UPDATE SET \
                 visible = EXCLUDED.visible, \
                 updated_at = NOW(), \
                 updated_by = EXCLUDED.updated_by",
        )
        .bind(&guild_id)
        .bind(&e.component_key)
        .bind(&e.role)
        .bind(e.visible)
        .bind(&ctx.discord_user_id)
        .execute(&mut *tx)
        .await
        .map_err(sqlx_internal("upsert"))?;
    }

    tx.commit().await.map_err(sqlx_internal("commit"))?;

    Ok(Json(
        serde_json::json!({ "ok": true, "count": body.entries.len() }),
    ))
}
