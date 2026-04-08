use sqlx::PgPool;
use tokio::sync::watch;
use tracing::info;

use sentinel_worker_common::spawn_periodic;

use crate::config::{CleanupConfig, WorkerConfig};
use crate::jobs;

pub fn start(config: &WorkerConfig, pool: PgPool, shutdown: watch::Receiver<bool>) {
    let cleanup_config = CleanupConfig::from(config);
    let api_url = config.api_url.clone();

    // ── Cleanup old data (periodic) ──
    {
        let cfg = cleanup_config.clone();
        spawn_periodic(
            "cleanup_old_data",
            config.cleanup_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cleanup-worker",
            move |pool| {
                let cfg = cfg.clone();
                Box::pin(async move { jobs::cleanup_old_data::run(&pool, &cfg).await })
            },
        );
    }

    // ── VACUUM ANALYZE (periodic, if enabled) ──
    if config.vacuum_enabled {
        spawn_periodic(
            "vacuum_tables",
            config.vacuum_interval_secs,
            pool,
            shutdown,
            api_url,
            "cleanup-worker",
            |pool| Box::pin(async move { jobs::vacuum_tables::run(&pool).await }),
        );
    } else {
        info!("VACUUM desactive par configuration");
    }
}
