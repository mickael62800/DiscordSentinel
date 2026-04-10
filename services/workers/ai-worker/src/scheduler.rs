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
    let job_timeout = config.job_timeout_secs;

    // Phase 4 A — Job principal : depile la file ai_jobs et dispatch vers
    // l'inference de l'API. Tourne tres souvent (poll_interval_secs).
    let redis_for_drain = redis_client.clone();
    let api_for_drain = api_url.clone();
    spawn_periodic(
        "drain_ai_jobs",
        config.poll_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url,
        "ai-worker",
        move |pool| {
            let redis = redis_for_drain.clone();
            let api = api_for_drain.clone();
            Box::pin(async move {
                jobs::drain_ai_jobs::run(&pool, &redis, &api, job_timeout).await
            })
        },
    );
}
