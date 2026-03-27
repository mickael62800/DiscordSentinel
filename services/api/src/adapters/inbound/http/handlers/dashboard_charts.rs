use axum::extract::{Query, State};
use axum::Json;

use crate::adapters::inbound::http::dto::dashboard_charts::{ChartQueryParams, DailyActivityDto};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

pub async fn get_activity_trend(
    State(state): State<AppState>,
    Query(params): Query<ChartQueryParams>,
) -> Result<Json<Vec<DailyActivityDto>>, ApiError> {
    let days = params.days.unwrap_or(30).min(90);
    let activity = state
        .daily_activity_repo
        .get_activity(params.guild_id.as_deref(), days)
        .await?;
    let dtos: Vec<DailyActivityDto> = activity.into_iter().map(DailyActivityDto::from).collect();
    Ok(Json(dtos))
}
