use axum::extract::{Path, Query, State};
use axum::Json;

use crate::adapters::inbound::http::dto::infractions::{InfractionQueryParams, InfractionResponseDto};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, normalize_limit, ok_response};
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::InfractionFilters;

pub async fn list_infractions(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<InfractionQueryParams>,
) -> Result<Json<Vec<InfractionResponseDto>>, ApiError> {
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
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Recuperer l'infraction avant suppression pour le DM
    let infraction = state.infractions_uc.find_by_id(&id).await?;

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
