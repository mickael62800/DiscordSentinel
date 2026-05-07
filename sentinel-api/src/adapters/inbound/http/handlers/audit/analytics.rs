use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use redis::AsyncCommands;
use tracing::warn;

use crate::adapters::inbound::http::dto::audit::analytics::*;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;

/// TTL du cache analytics (5 minutes).
const ANALYTICS_CACHE_TTL: u64 = 300;

/// Construit la cle de cache pour un endpoint analytics.
fn cache_key(endpoint: &str, guild_id: Option<&str>, days: i32, limit: Option<i64>) -> String {
    let gid = guild_id.unwrap_or("global");
    match limit {
        Some(l) => format!("analytics:{endpoint}:{gid}:{days}:{l}"),
        None => format!("analytics:{endpoint}:{gid}:{days}"),
    }
}

/// Tente de lire une valeur depuis le cache Redis.
async fn try_cache_get<T: serde::de::DeserializeOwned>(state: &AppState, key: &str) -> Option<T> {
    let mut conn = state.redis_client.get_multiplexed_async_connection().await.ok()?;
    let json: Option<String> = conn.get(key).await.ok()?;
    let json = json?;
    serde_json::from_str(&json).ok()
}

/// Ecrit une valeur dans le cache Redis.
async fn try_cache_set<T: serde::Serialize>(state: &AppState, key: &str, value: &T) {
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(value) {
            let result: Result<(), _> = conn.set_ex(key, json, ANALYTICS_CACHE_TTL).await;
            if let Err(e) = result {
                warn!(error = %e, key = key, "Erreur ecriture cache analytics");
            }
        }
    }
}

/// GET /api/analytics — Retourne toutes les analytics en une seule requete (cache 5min).
pub async fn get_full_analytics(
    State(state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<FullAnalyticsDto>, ApiError> {
    let days = params.days();
    let limit = params.limit();
    let guild_id = params.guild_id.as_deref();
    let key = cache_key("full", guild_id, days, Some(limit));

    // Cache-first
    if let Some(cached) = try_cache_get::<FullAnalyticsDto>(&state, &key).await {
        return Ok(Json(cached));
    }

    let (heatmap, distribution, infractors, trend, peaks) = tokio::try_join!(
        state.analytics_repo.get_heatmap(guild_id, days),
        state.analytics_repo.get_action_distribution(guild_id, days),
        state.analytics_repo.get_top_infractors(guild_id, days, limit),
        state.analytics_repo.get_moderation_trend(guild_id, days),
        state.analytics_repo.get_peak_hours(guild_id, days),
    )?;

    let dto = FullAnalyticsDto {
        heatmap: heatmap.into_iter().map(Into::into).collect(),
        action_distribution: distribution.into_iter().map(Into::into).collect(),
        top_infractors: infractors.into_iter().map(Into::into).collect(),
        moderation_trend: trend.into_iter().map(Into::into).collect(),
        peak_hours: peaks.into_iter().map(Into::into).collect(),
    };

    try_cache_set(&state, &key, &dto).await;

    Ok(Json(dto))
}

/// GET /api/analytics/heatmap (cache 5min)
pub async fn get_heatmap(
    State(state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<Vec<HeatmapPointDto>>, ApiError> {
    let key = cache_key("heatmap", params.guild_id.as_deref(), params.days(), None);

    if let Some(cached) = try_cache_get::<Vec<HeatmapPointDto>>(&state, &key).await {
        return Ok(Json(cached));
    }

    let data = state
        .analytics_repo
        .get_heatmap(params.guild_id.as_deref(), params.days())
        .await?;
    let dtos: Vec<HeatmapPointDto> = data.into_iter().map(Into::into).collect();

    try_cache_set(&state, &key, &dtos).await;

    Ok(Json(dtos))
}

/// GET /api/analytics/actions (cache 5min)
pub async fn get_action_distribution(
    State(state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<Vec<ActionDistributionDto>>, ApiError> {
    let key = cache_key("actions", params.guild_id.as_deref(), params.days(), None);

    if let Some(cached) = try_cache_get::<Vec<ActionDistributionDto>>(&state, &key).await {
        return Ok(Json(cached));
    }

    let data = state
        .analytics_repo
        .get_action_distribution(params.guild_id.as_deref(), params.days())
        .await?;
    let dtos: Vec<ActionDistributionDto> = data.into_iter().map(Into::into).collect();

    try_cache_set(&state, &key, &dtos).await;

    Ok(Json(dtos))
}

/// GET /api/analytics/top-infractors (cache 5min)
///
/// Si `limit` n'est pas fourni en query et qu'un `guild_id` est specifie,
/// fallback sur la cle `top_users_count` de bot_guild_config (module
/// `analytics`). Sinon defaut hardcode 10.
pub async fn get_top_infractors(
    State(state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<Vec<TopInfractorDto>>, ApiError> {
    let effective_limit = match params.limit {
        Some(_) => params.limit(),
        None => {
            // Pas de limit explicite : tente la cle top_users_count du guild.
            if let Some(gid) = params.guild_id.as_deref() {
                if let Ok(cfgs) = state.bot_config_repo.get_config(gid, "analytics").await {
                    if let Some(v) = cfgs
                        .iter()
                        .find(|c| c.config_key == "top_users_count")
                        .and_then(|c| c.config_value.parse::<i64>().ok())
                    {
                        v.clamp(1, 100)
                    } else {
                        params.limit()
                    }
                } else {
                    params.limit()
                }
            } else {
                params.limit()
            }
        }
    };

    let key = cache_key("infractors", params.guild_id.as_deref(), params.days(), Some(effective_limit));

    if let Some(cached) = try_cache_get::<Vec<TopInfractorDto>>(&state, &key).await {
        return Ok(Json(cached));
    }

    let data = state
        .analytics_repo
        .get_top_infractors(params.guild_id.as_deref(), params.days(), effective_limit)
        .await?;
    let dtos: Vec<TopInfractorDto> = data.into_iter().map(Into::into).collect();

    try_cache_set(&state, &key, &dtos).await;

    Ok(Json(dtos))
}

/// GET /api/analytics/moderation-trend (cache 5min)
pub async fn get_moderation_trend(
    State(state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<Vec<ModerationTrendDto>>, ApiError> {
    let key = cache_key("trend", params.guild_id.as_deref(), params.days(), None);

    if let Some(cached) = try_cache_get::<Vec<ModerationTrendDto>>(&state, &key).await {
        return Ok(Json(cached));
    }

    let data = state
        .analytics_repo
        .get_moderation_trend(params.guild_id.as_deref(), params.days())
        .await?;
    let dtos: Vec<ModerationTrendDto> = data.into_iter().map(Into::into).collect();

    try_cache_set(&state, &key, &dtos).await;

    Ok(Json(dtos))
}

/// GET /api/analytics/peak-hours (cache 5min)
pub async fn get_peak_hours(
    State(state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<Vec<PeakHourDto>>, ApiError> {
    let key = cache_key("peaks", params.guild_id.as_deref(), params.days(), None);

    if let Some(cached) = try_cache_get::<Vec<PeakHourDto>>(&state, &key).await {
        return Ok(Json(cached));
    }

    let data = state
        .analytics_repo
        .get_peak_hours(params.guild_id.as_deref(), params.days())
        .await?;
    let dtos: Vec<PeakHourDto> = data.into_iter().map(Into::into).collect();

    try_cache_set(&state, &key, &dtos).await;

    Ok(Json(dtos))
}

/// POST /api/analytics/reset?guild_id=X
///
/// Reset les compteurs d'activite (hourly_activity + daily_activity) pour
/// la guild specifiee. NE TOUCHE PAS aux infractions/audit_logs (donnees
/// d'audit reelles, conservees pour la chaine de moderation).
/// Flush egalement le cache Redis analytics:* pour la guild.
#[derive(serde::Serialize)]
pub struct ResetAnalyticsResponse {
    pub deleted_rows: u64,
}

pub async fn reset_analytics(
    State(state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<ResetAnalyticsResponse>, ApiError> {
    let guild_id = params.guild_id.as_deref()
        .ok_or_else(|| ApiError::from(sentinel_core::domain::errors::DomainError::ValidationError("guild_id requis".into())))?;

    let deleted_rows = state.analytics_repo.reset_activity(guild_id).await?;

    // Flush du cache Redis (toutes les cles analytics:*:<guild_id>:*).
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        let pattern = format!("analytics:*:{guild_id}:*");
        if let Ok(keys) = conn.keys::<_, Vec<String>>(pattern).await {
            if !keys.is_empty() {
                let _: Result<(), _> = conn.del(keys).await;
            }
        }
    }

    Ok(Json(ResetAnalyticsResponse { deleted_rows }))
}

#[cfg(test)]
#[path = "tests/analytics.rs"]
mod tests;
