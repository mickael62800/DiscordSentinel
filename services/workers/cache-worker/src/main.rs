mod config;
mod jobs;
mod scheduler;

use tokio::sync::watch;
use tracing::info;

use crate::config::WorkerConfig;
use sentinel_worker_common as common;

const WORKER_NAME: &str = "cache-worker";

#[tokio::main]
async fn main() {
    common::init_tracing("sentinel_cache_worker=info");

    let mut config = WorkerConfig::from_env();

    info!("Demarrage de Sentinel Cache Worker");

    let pg_pool = common::create_pg_pool(&config.database_url).await;
    info!("PostgreSQL connecte");

    let db_config = common::load_worker_config(&pg_pool, WORKER_NAME).await;
    if !db_config.is_empty() {
        config.apply_db_config(&db_config);
        info!(keys = db_config.len(), "Config DB chargee");
    }

    let redis_client = redis::Client::open(config.redis_url.as_str())
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "Impossible de creer le client Redis");
            std::process::exit(1);
        });
    info!("Redis client cree");

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    scheduler::start(&config, pg_pool.clone(), redis_client, shutdown_rx);
    common::start_heartbeat(config.api_url.clone(), WORKER_NAME);

    common::send_lifecycle_log(&config.api_url, WORKER_NAME, "info", "Cache Worker demarre").await;

    info!("Sentinel Cache Worker pret");

    common::shutdown_signal().await;

    common::send_lifecycle_log(&config.api_url, WORKER_NAME, "warn", "Cache Worker en cours d'arret").await;

    info!("Arret en cours...");
    let _ = shutdown_tx.send(true);

    pg_pool.close().await;
    info!("Sentinel Cache Worker arrete proprement");
}
