mod config;
mod monitor;

use tracing::info;

use crate::config::MonitorConfig;
use sentinel_worker_common as common;

const WORKER_NAME: &str = "monitoring-worker";

#[tokio::main]
async fn main() {
    common::init_tracing("sentinel_monitoring_worker=info");

    let config = MonitorConfig::from_env();

    info!("Demarrage de Sentinel Monitoring Worker");

    let redis_client =
        redis::Client::open(config.redis_url.as_str()).expect("URL Redis invalide");

    match redis_client.get_multiplexed_async_connection().await {
        Ok(_) => info!("Redis connecte"),
        Err(e) => {
            tracing::error!("Redis indisponible: {e}");
            std::process::exit(1);
        }
    }

    common::start_heartbeat(config.api_url.clone(), WORKER_NAME);
    monitor::start(redis_client, config.clone());

    common::send_lifecycle_log(&config.api_url, WORKER_NAME, "info", "Monitoring Worker demarre").await;

    info!("Sentinel Monitoring Worker pret");

    common::shutdown_signal().await;

    common::send_lifecycle_log(&config.api_url, WORKER_NAME, "warn", "Monitoring Worker en cours d'arret").await;

    info!("Sentinel Monitoring Worker arrete");
}
