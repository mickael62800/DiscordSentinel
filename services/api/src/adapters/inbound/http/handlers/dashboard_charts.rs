use axum::extract::{Query, State};
use axum::Json;

use crate::adapters::inbound::http::dto::dashboard_charts::{ChartQueryParams, DailyActivityDto};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::{map_to_dtos, normalize_days};
use crate::adapters::inbound::http::state::AppState;

pub async fn get_activity_trend(
    State(state): State<AppState>,
    Query(params): Query<ChartQueryParams>,
) -> Result<Json<Vec<DailyActivityDto>>, ApiError> {
    let days = normalize_days(params.days, 30, 90);
    let activity = state
        .daily_activity_repo
        .get_activity(params.guild_id.as_deref(), days)
        .await?;
    Ok(map_to_dtos(activity))
}
