use axum::extract::State;
use axum::Json;

use crate::adapters::inbound::http::state::AppState;
use crate::adapters::outbound::redis_cache::CacheStats;

/// GET /api/cache/stats — Retourne les statistiques hit/miss du cache Redis.
pub async fn get_cache_stats(State(state): State<AppState>) -> Json<CacheStats> {
    match &state.cache {
        Some(cache) => Json(cache.stats()),
        None => Json(CacheStats {
            hits: 0,
            misses: 0,
            total: 0,
            hit_rate_percent: 0.0,
        }),
    }
}
