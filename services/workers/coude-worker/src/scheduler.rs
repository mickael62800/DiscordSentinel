use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::watch;
use tracing::{error, info};

use crate::config::WorkerConfig;
use crate::jobs;

pub fn start(config: &WorkerConfig, pool: PgPool, shutdown: watch::Receiver<bool>) {
    let interval = config.combat_expiry_check_secs;

    info!(interval_secs = interval, "Tache periodique planifiee: expire_combats");

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(interval)).await;

            if *shutdown.borrow() {
                info!("Tache expire_combats arretee (shutdown)");
                break;
            }

            if let Err(e) = jobs::expire_combats::run(&pool).await {
                error!(error = %e, "Erreur tache expire_combats");
            }
        }
    });
}
