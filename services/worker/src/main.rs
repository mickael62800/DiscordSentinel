mod config;
mod jobs;
mod queue;
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
                .unwrap_or_else(|_| "sentinel_worker=info".into()),
        )
        .init();

    let config = WorkerConfig::from_env();

    info!("Démarrage de Sentinel Worker");

    // ── PostgreSQL ──
    let pg_pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await
        .expect("Impossible de se connecter à PostgreSQL");

    info!("PostgreSQL connecté");

    // ── Redis ──
    let redis_client =
        redis::Client::open(config.redis_url.as_str()).expect("URL Redis invalide");

    match redis_client.get_multiplexed_async_connection().await {
        Ok(_) => info!("Redis connecté"),
        Err(e) => {
            tracing::error!("Redis indisponible: {e}");
            std::process::exit(1);
        }
    }

    // ── Shutdown signal ──
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // ── Tâches périodiques ──
    scheduler::start(&config, pg_pool.clone(), shutdown_rx.clone());

    // ── Consumer de jobs (queue Redis) ──
    let consumer_handle = tokio::spawn(queue::consume_loop(
        redis_client,
        config.queue_key.clone(),
        pg_pool.clone(),
        shutdown_rx,
    ));

    info!(queue = %config.queue_key, "Sentinel Worker prêt");

    // ── Attente signal d'arrêt ──
    shutdown_signal().await;

    info!("Arrêt en cours...");
    let _ = shutdown_tx.send(true);

    // Attendre que le consumer se termine
    let timeout = Duration::from_secs(config.shutdown_timeout_secs);
    let _ = tokio::time::timeout(timeout, consumer_handle).await;

    pg_pool.close().await;
    info!("Sentinel Worker arrêté proprement");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Impossible d'écouter Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Impossible d'écouter SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Signal Ctrl+C reçu"),
        _ = terminate => info!("Signal SIGTERM reçu"),
    }
}
