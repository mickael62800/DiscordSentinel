use sqlx::PgPool;
use tokio::sync::watch;

use crate::config::WorkerConfig;
use crate::jobs;
use sentinel_worker_common::spawn_periodic;

pub fn start(config: &WorkerConfig, pool: PgPool, shutdown: watch::Receiver<bool>) {
    let api_url = config.api_url.clone();

    spawn_periodic(
        "drain_export_jobs",
        config.scan_interval_secs,
        pool,
        shutdown,
        api_url,
        "export-worker",
        move |pool| Box::pin(async move { jobs::drain_export_jobs::run(&pool).await }),
    );
}
