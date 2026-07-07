//! GET/PUT /api/rbac/component-visibility/{guild_id} — gestion des overrides
//! de visibilite des composants UI par role (adaptateur ENTRANT mince).
//!
//! Lecture : ouverte a tout role authentifie (chaque utilisateur recupere
//! la liste pour appliquer les overrides cote front).
//! Ecriture : Owner+ (modifie ce que les modos/admins voient).
//!
//! Ici : parse/RBAC/validation -> use case -> map. Le SQL (dont la transaction
//! batch) vit dans `ComponentVisibilityRepository`.

use crate::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::require_role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::entities::system::component_visibility::VisibilityEntry;
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

/// GET — liste tous les overrides de visibilite pour la guild.
pub async fn list_visibility(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<VisibilityEntryDto>>, ApiError> {
    let entries = state.component_visibility_uc.list(&guild_id).await?;

    let out: Vec<VisibilityEntryDto> = entries
        .into_iter()
        .map(|e| VisibilityEntryDto {
            component_key: e.component_key,
            role: e.role,
            visible: e.visible,
        })
        .collect();
    Ok(Json(out))
}

/// PUT — upsert batch de tous les overrides envoyes. Les entrees absentes
/// du payload sont conservees telles quelles (delete explicite via visibility
/// par defaut : on garde simple, l'UI envoie tout l'etat affiche).
pub async fn upsert_visibility(
    State(state): State<AppState>,
    Extension(ctx): Extension<RoleContext>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(body): Json<VisibilityBatchDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_role(&ctx, Role::Owner).map_err(|_| {
        ApiError(DomainError::Forbidden(
            "owner+ requis pour modifier la visibilite".into(),
        ))
    })?;

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

    let entries: Vec<VisibilityEntry> = body
        .entries
        .into_iter()
        .map(|e| VisibilityEntry {
            component_key: e.component_key,
            role: e.role,
            visible: e.visible,
        })
        .collect();

    let count = state
        .component_visibility_uc
        .upsert_batch(&guild_id, entries, &ctx.discord_user_id)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true, "count": count })))
}
