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
    let redis = redis_client.clone();

    spawn_periodic(
        "expire_temp_roles",
        config.scan_interval_secs,
        pool,
        shutdown,
        api_url,
        "temp-roles-worker",
        move |pool| {
            let redis = redis.clone();
            Box::pin(async move { jobs::expire_temp_roles::run(&pool, &redis).await })
        },
    );
}
