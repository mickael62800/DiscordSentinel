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

    spawn_periodic(
        "conduct_regen",
        config.conduct_regen_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "moderation-worker",
        |pool| Box::pin(async move { jobs::conduct_regen::run(&pool).await }),
    );

    spawn_periodic(
        "cleanup_bans",
        config.ban_cleanup_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "moderation-worker",
        |pool| Box::pin(async move { jobs::cleanup_bans::run(&pool).await }),
    );

    spawn_periodic(
        "sync_ban_proposals",
        config.sync_ban_proposals_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "moderation-worker",
        |pool| Box::pin(async move { jobs::sync_ban_proposals::run(&pool).await }),
    );

    let redis_for_reminders = redis_client.clone();
    spawn_periodic(
        "send_reminders",
        config.send_reminders_interval_secs,
        pool,
        shutdown,
        api_url,
        "moderation-worker",
        move |pool| {
            let redis = redis_for_reminders.clone();
            Box::pin(async move { jobs::send_reminders::run(&pool, &redis).await })
        },
    );
}
