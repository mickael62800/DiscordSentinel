//! Handlers capitaux (vue detaillee + conversions).

use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use sentinel_core::domain::entities::influence::conversion::ConversionKind;
use sentinel_core::domain::errors::DomainError;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use crate::adapters::inbound::http::handlers::influence::dto::{
    CapitalOverviewDto, ConversionOutcomeDto,
};
use crate::adapters::inbound::http::state::AppState;

#[derive(Debug, Deserialize)]
pub struct UserDto {
    pub user_id: String,
    #[serde(default)]
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct ConvertDto {
    pub user_id: String,
    #[serde(default)]
    pub username: String,
    pub kind: String,
    pub budget: i64,
}

/// POST /api/influence/{guild_id}/capital
pub async fn view_capital(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<UserDto>,
) -> Result<Json<CapitalOverviewDto>, ApiError> {
    let overview = state
        .influence_capital_uc
        .view(&guild_id, &dto.user_id, &dto.username)
        .await?;
    Ok(Json(overview.into()))
}

/// POST /api/influence/{guild_id}/capital/convert
pub async fn convert_capital(
    State(state): State<AppState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<ConvertDto>,
) -> Result<Json<ConversionOutcomeDto>, ApiError> {
    let kind = ConversionKind::from_str_lossy(&dto.kind).ok_or_else(|| {
        ApiError(DomainError::ValidationError(format!(
            "Conversion invalide : {}",
            dto.kind
        )))
    })?;
    let outcome = state
        .influence_capital_uc
        .convert(&guild_id, &dto.user_id, &dto.username, kind, dto.budget)
        .await?;
    Ok(Json(outcome.into()))
}
