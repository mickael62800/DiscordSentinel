mod config;
mod heartbeat;
mod jobs;
mod scheduler;

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tokio::signal;
use tokio::sync::watch;
use tracing::info;

use crate::config::WorkerConfig;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sentinel_analytics_worker=info".into()),
        )
        .init();

    let config = WorkerConfig::from_env();

    info!("Demarrage de Sentinel Analytics Worker");

    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await
        .expect("Impossible de se connecter a PostgreSQL");

    info!("PostgreSQL connecte");

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    scheduler::start(&config, pg_pool.clone(), shutdown_rx);
    heartbeat::start(config.api_url.clone(), "analytics-worker");

    send_lifecycle_log(&config.api_url, "info", "Analytics Worker demarre").await;

    info!("Sentinel Analytics Worker pret");

    shutdown_signal().await;

    send_lifecycle_log(&config.api_url, "warn", "Analytics Worker en cours d'arret").await;

    info!("Arret en cours...");
    let _ = shutdown_tx.send(true);

    pg_pool.close().await;
    info!("Sentinel Analytics Worker arrete proprement");
}

async fn send_lifecycle_log(api_url: &str, level: &str, message: &str) {
    let _ = reqwest::Client::new()
        .post(format!("{}/api/logs", api_url))
        .json(&serde_json::json!({
            "level": level,
            "bot": "analytics-worker",
            "server": "",
            "message": message,
            "category": "worker",
        }))
        .send()
        .await;
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Impossible d'ecouter Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Impossible d'ecouter SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Signal Ctrl+C recu"),
        _ = terminate => info!("Signal SIGTERM recu"),
    }
}
