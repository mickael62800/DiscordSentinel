use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::watch;
use tracing::{error, info};

use crate::config::WorkerConfig;
use crate::jobs;

pub fn start(config: &WorkerConfig, pool: PgPool, shutdown: watch::Receiver<bool>) {
    let expiry_interval = config.combat_expiry_check_secs;
    let betting_interval = config.betting_check_secs;
    let api_url = config.api_url.clone();
    let bot_token = config.discord_bot_token.clone();

    // Job 1 : expiration des combats pending (toutes les 24h)
    let pool_expiry = pool.clone();
    let shutdown_expiry = shutdown.clone();
    info!(interval_secs = expiry_interval, "Tache planifiee: expire_combats");
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(expiry_interval)).await;
            if *shutdown_expiry.borrow() { break; }
            if let Err(e) = jobs::expire_combats::run(&pool_expiry).await {
                error!(error = %e, "Erreur tache expire_combats");
            }
        }
    });

    // Job 2 : resolution des combats betting (toutes les 30s)
    info!(interval_secs = betting_interval, "Tache planifiee: resolve_betting");
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(betting_interval)).await;
            if *shutdown.borrow() { break; }
            if let Err(e) = jobs::resolve_betting::run(&pool, &api_url, &bot_token).await {
                error!(error = %e, "Erreur tache resolve_betting");
            }
        }
    });
}
