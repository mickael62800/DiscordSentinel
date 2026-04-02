use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::watch;
use tracing::{error, info};

use crate::config::{CleanupConfig, WorkerConfig};
use crate::jobs;

pub fn start(config: &WorkerConfig, pool: PgPool, shutdown: watch::Receiver<bool>) {
    let cleanup_config = CleanupConfig::from(config);

    // ── Cleanup old data (periodic) ──
    {
        let pool = pool.clone();
        let shutdown = shutdown.clone();
        let cfg = cleanup_config.clone();
        let interval = config.cleanup_interval_secs;

        info!(interval_secs = interval, "Tache periodique planifiee: cleanup_old_data");

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(interval)).await;

                if *shutdown.borrow() {
                    info!("Tache cleanup_old_data arretee (shutdown)");
                    break;
                }

                if let Err(e) = jobs::cleanup_old_data::run(&pool, &cfg).await {
                    error!(error = %e, "Erreur tache cleanup_old_data");
                }
            }
        });
    }

    // ── VACUUM ANALYZE (periodic, if enabled) ──
    if config.vacuum_enabled {
        let pool = pool;
        let shutdown = shutdown;
        let interval = config.vacuum_interval_secs;

        info!(interval_secs = interval, "Tache periodique planifiee: vacuum_tables");

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(interval)).await;

                if *shutdown.borrow() {
                    info!("Tache vacuum_tables arretee (shutdown)");
                    break;
                }

                if let Err(e) = jobs::vacuum_tables::run(&pool).await {
                    error!(error = %e, "Erreur tache vacuum_tables");
                }
            }
        });
    } else {
        info!("VACUUM desactive par configuration");
    }
}
