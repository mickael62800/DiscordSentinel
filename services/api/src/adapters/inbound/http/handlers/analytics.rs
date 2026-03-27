use axum::extract::{Query, State};
use axum::Json;

use crate::adapters::inbound::http::dto::analytics::*;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

/// GET /api/analytics — Retourne toutes les analytics en une seule requete.
pub async fn get_full_analytics(
    State(state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<FullAnalyticsDto>, ApiError> {
    let days = params.days();
    let limit = params.limit();
    let guild_id = params.guild_id.as_deref();

    let (heatmap, distribution, infractors, trend, peaks) = tokio::try_join!(
        state.analytics_repo.get_heatmap(guild_id, days),
        state.analytics_repo.get_action_distribution(guild_id, days),
        state.analytics_repo.get_top_infractors(guild_id, days, limit),
        state.analytics_repo.get_moderation_trend(guild_id, days),
        state.analytics_repo.get_peak_hours(guild_id, days),
    )?;

    Ok(Json(FullAnalyticsDto {
        heatmap: heatmap.into_iter().map(Into::into).collect(),
        action_distribution: distribution.into_iter().map(Into::into).collect(),
        top_infractors: infractors.into_iter().map(Into::into).collect(),
        moderation_trend: trend.into_iter().map(Into::into).collect(),
        peak_hours: peaks.into_iter().map(Into::into).collect(),
    }))
}

/// GET /api/analytics/heatmap
pub async fn get_heatmap(
    State(state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<Vec<HeatmapPointDto>>, ApiError> {
    let data = state
        .analytics_repo
        .get_heatmap(params.guild_id.as_deref(), params.days())
        .await?;
    Ok(Json(data.into_iter().map(Into::into).collect()))
}

/// GET /api/analytics/actions
pub async fn get_action_distribution(
    State(state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<Vec<ActionDistributionDto>>, ApiError> {
    let data = state
        .analytics_repo
        .get_action_distribution(params.guild_id.as_deref(), params.days())
        .await?;
    Ok(Json(data.into_iter().map(Into::into).collect()))
}

/// GET /api/analytics/top-infractors
pub async fn get_top_infractors(
    State(state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<Vec<TopInfractorDto>>, ApiError> {
    let data = state
        .analytics_repo
        .get_top_infractors(params.guild_id.as_deref(), params.days(), params.limit())
        .await?;
    Ok(Json(data.into_iter().map(Into::into).collect()))
}

/// GET /api/analytics/moderation-trend
pub async fn get_moderation_trend(
    State(state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<Vec<ModerationTrendDto>>, ApiError> {
    let data = state
        .analytics_repo
        .get_moderation_trend(params.guild_id.as_deref(), params.days())
        .await?;
    Ok(Json(data.into_iter().map(Into::into).collect()))
}

/// GET /api/analytics/peak-hours
pub async fn get_peak_hours(
    State(state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<Vec<PeakHourDto>>, ApiError> {
    let data = state
        .analytics_repo
        .get_peak_hours(params.guild_id.as_deref(), params.days())
        .await?;
    Ok(Json(data.into_iter().map(Into::into).collect()))
}
