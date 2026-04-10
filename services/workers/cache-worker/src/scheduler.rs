use sqlx::PgPool;
use tokio::sync::watch;

use crate::config::WorkerConfig;
use crate::jobs;
use sentinel_worker_common::spawn_periodic;

pub fn start(
    config: &WorkerConfig,
    pool: PgPool,
    redis_client: redis::Client,
    shutdown: watch::Receiver<bool>,
) {
    let api_url = config.api_url.clone();

    let redis_analytics = redis_client.clone();
    spawn_periodic(
        "warm_analytics",
        config.analytics_refresh_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "cache-worker",
        move |pool| {
            let redis = redis_analytics.clone();
            Box::pin(async move { jobs::warm_analytics::run(&pool, &redis).await })
        },
    );

    let redis_dashboard = redis_client.clone();
    spawn_periodic(
        "warm_dashboard",
        config.dashboard_refresh_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "cache-worker",
        move |pool| {
            let redis = redis_dashboard.clone();
            Box::pin(async move { jobs::warm_dashboard::run(&pool, &redis).await })
        },
    );

    let redis_voice = redis_client;
    spawn_periodic(
        "warm_voice_stats",
        config.voice_stats_refresh_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "cache-worker",
        move |pool| {
            let redis = redis_voice.clone();
            Box::pin(async move { jobs::warm_voice_stats::run(&pool, &redis).await })
        },
    );

    // Phase 2 A.2 — Refresh des vues materialisees leaderboards.
    spawn_periodic(
        "refresh_leaderboards",
        config.leaderboards_refresh_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "cache-worker",
        move |pool| {
            Box::pin(async move { jobs::refresh_leaderboards::run(&pool).await })
        },
    );

    // Phase 2 A.2 — Sync de la table user_cache (source de verite usernames).
    spawn_periodic(
        "sync_user_cache",
        config.user_cache_sync_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "cache-worker",
        move |pool| {
            Box::pin(async move { jobs::sync_user_cache::run(&pool).await })
        },
    );

    // Phase 2 A.4 — Partition manager : cree les partitions M+1 et M+2.
    spawn_periodic(
        "manage_partitions",
        config.partition_manager_secs,
        pool,
        shutdown,
        api_url,
        "cache-worker",
        move |pool| {
            Box::pin(async move { jobs::manage_partitions::run(&pool).await })
        },
    );
}
