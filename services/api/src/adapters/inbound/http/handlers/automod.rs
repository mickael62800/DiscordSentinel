//! Handler HTTP du module Automod (Phase 4).
//!
//! Pas de logique metier ici — on reutilise `ManageInfractionsUseCase`
//! (port inbound) avec un filtre `action="detection"`. La page
//! `/automod` cote web consomme ce endpoint pour la timeline des
//! detections automod.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::infractions::InfractionResponseDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, normalize_limit, normalize_offset};
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use crate::ports::inbound::InfractionFilters;

#[derive(Debug, Deserialize)]
pub struct DetectionQuery {
    /// Defaut 50, max 200.
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Optionnel : filtre par utilisateur.
    pub user_id: Option<String>,
}

/// GET /api/automod/{guild_id}/detections
pub async fn list_detections(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<DetectionQuery>,
) -> Result<Json<Vec<InfractionResponseDto>>, ApiError> {
    validation::validate_guild_id_path(&guild_id).map_err(ApiError)?;

    // Filtre `action = "detection"` : seules les detections automod, pas
    // les actions de moderation (warn/mute/ban...).
    let filters = InfractionFilters {
        user_id: params.user_id,
        action: Some("detection".to_string()),
        limit: normalize_limit(params.limit, 50, 200),
        offset: normalize_offset(params.offset),
    };

    let detections = state
        .infractions_uc
        .list_infractions(&guild_id, filters)
        .await?;
    Ok(map_to_dtos(detections))
}
