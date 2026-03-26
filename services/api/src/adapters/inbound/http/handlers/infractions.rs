use axum::extract::{Path, Query, State};
use axum::Json;

use crate::adapters::inbound::http::dto::infractions::{InfractionQueryParams, InfractionResponseDto};
use crate::adapters::inbound::http::errors::ApiError;
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
        limit: params.limit.unwrap_or(50).min(200),
        offset: params.offset.unwrap_or(0),
    };

    let infractions = state.infractions_uc.list_infractions(&guild_id, filters).await?;
    let dtos: Vec<InfractionResponseDto> = infractions
        .into_iter()
        .map(InfractionResponseDto::from)
        .collect();
    Ok(Json(dtos))
}
