use axum::extract::{Path, Query, State};
use axum::{Extension, Json};

use crate::adapters::inbound::http::dto::infractions::{InfractionQueryParams, InfractionResponseDto};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, normalize_limit, ok_response};
use crate::adapters::inbound::http::middleware::rbac::{require_role_for_guild, Role, RoleContext};
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use crate::domain::errors::DomainError;
use crate::ports::inbound::InfractionFilters;

pub async fn list_infractions(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<InfractionQueryParams>,
) -> Result<Json<Vec<InfractionResponseDto>>, ApiError> {
    // Validation
    validation::validate_guild_id_path(&guild_id).map_err(ApiError)?;

    let filters = InfractionFilters {
        user_id: params.user_id,
        action: params.action,
        limit: normalize_limit(params.limit, 50, 200),
        offset: params.offset.unwrap_or(0),
    };

    let infractions = state.infractions_uc.list_infractions(&guild_id, filters).await?;
    Ok(map_to_dtos(infractions))
}

pub async fn delete_infraction(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Recuperer l'infraction avant suppression pour le DM ET pour le gate RBAC
    let infraction = state.infractions_uc.find_by_id(&id).await?;

    // Phase 7 B — Gate RBAC : moderator+ requis. L'infraction porte son
    // propre guild_id, donc on fetch d'abord puis on verifie le role
    // via require_role_for_guild (pattern "ressource-id-based").
    if let (Some(Extension(ctx)), Some(ref inf)) = (rbac.as_ref(), infraction.as_ref()) {
        require_role_for_guild(&state, ctx, &inf.guild_id, Role::Moderator)
            .await
            .map_err(|_| ApiError(DomainError::Forbidden("moderator+ requis pour supprimer une infraction".into())))?;
    }

    let deleted = state.infractions_uc.delete_infraction(&id).await?;
    if !deleted {
        return Err(crate::domain::errors::DomainError::NotFound("Infraction introuvable".into()).into());
    }

    // Envoyer un DM a l'utilisateur pour l'informer de la grace
    if let Some(inf) = infraction {
        let message = format!(
            "Bonne nouvelle ! Votre avertissement a ete annule.\n\
            **Raison initiale** : {}\n\
            **Type** : {}\n\n\
            Cette infraction a ete retiree de votre dossier.",
            inf.reason,
            inf.action.as_str(),
        );
        if let Err(e) = state.discord_api.send_dm(&inf.user_id, &message).await {
            tracing::warn!("Echec envoi DM de grace a {}: {e}", inf.user_id);
        }
    }

    Ok(ok_response())
}
