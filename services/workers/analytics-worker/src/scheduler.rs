use sqlx::PgPool;
use tokio::sync::watch;

use crate::config::WorkerConfig;
use crate::jobs;
use sentinel_worker_common::spawn_periodic;

pub fn start(config: &WorkerConfig, pool: PgPool, shutdown: watch::Receiver<bool>) {
    let api_url = config.api_url.clone();

    spawn_periodic(
        "daily_snapshot",
        config.daily_snapshot_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "analytics-worker",
        |pool| Box::pin(async move { jobs::daily_snapshot::run(&pool).await }),
    );

    spawn_periodic(
        "hourly_snapshot",
        config.hourly_snapshot_interval_secs,
        pool,
        shutdown,
        api_url,
        "analytics-worker",
        |pool| Box::pin(async move { jobs::hourly_snapshot::run(&pool).await }),
    );
}
