mod config;
mod monitor;

use tokio::signal;
use tracing::info;

use crate::config::MonitorConfig;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sentinel_monitoring_worker=info".into()),
        )
        .init();

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

    // Heartbeat pour se signaler en ligne
    let api_url = config.api_url.clone();
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let url = format!("{}/api/bots/heartbeat", api_url);
        loop {
            let _ = client
                .post(&url)
                .json(&serde_json::json!({ "name": "monitoring-worker" }))
                .send()
                .await;
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });

    // Lancer le monitoring
    let api_url2 = config.api_url.clone();
    monitor::start(redis_client, config);

    send_lifecycle_log(&api_url2, "info", "Monitoring Worker demarre").await;

    info!("Sentinel Monitoring Worker pret");

    shutdown_signal().await;

    send_lifecycle_log(&api_url2, "warn", "Monitoring Worker en cours d'arret").await;

    info!("Sentinel Monitoring Worker arrete");
}

async fn send_lifecycle_log(api_url: &str, level: &str, message: &str) {
    let _ = reqwest::Client::new()
        .post(format!("{}/api/logs", api_url))
        .json(&serde_json::json!({
            "level": level,
            "bot": "monitoring-worker",
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
