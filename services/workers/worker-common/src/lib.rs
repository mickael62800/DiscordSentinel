//! Infrastructure partagee entre les workers DiscordSentinel.
//!
//! Elimine la duplication de : shutdown signal, lifecycle logging,
//! heartbeat, scheduler, pool creation.

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::signal;
use tracing::{error, info, warn};

// ── Init ──

/// Initialise dotenvy + tracing avec un filtre par defaut.
pub fn init_tracing(default_filter: &str) {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| default_filter.into()),
        )
        .init();
}

// ── PostgreSQL ──

/// Cree un pool PostgreSQL avec des parametres configurables via env.
pub async fn create_pg_pool(database_url: &str) -> PgPool {
    let max_connections: u32 = std::env::var("PG_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);

    let acquire_timeout: u64 = std::env::var("PG_ACQUIRE_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);

    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(acquire_timeout))
        .connect(database_url)
        .await
        .expect("Impossible de se connecter a PostgreSQL")
}

// ── Lifecycle Logging ──

/// Envoie un log de cycle de vie a l'API.
pub async fn send_lifecycle_log(api_url: &str, worker_name: &str, level: &str, message: &str) {
    let _ = reqwest::Client::new()
        .post(format!("{}/api/logs", api_url))
        .json(&serde_json::json!({
            "level": level,
            "bot": worker_name,
            "server": "",
            "message": message,
            "category": "worker",
        }))
        .send()
        .await;
}

// ── Heartbeat ──

/// Demarre un heartbeat periodique vers l'API.
pub fn start_heartbeat(api_url: String, worker_name: &'static str) {
    let interval: u64 = std::env::var("HEARTBEAT_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let url = format!("{}/api/bots/heartbeat", api_url);

        loop {
            let _ = client
                .post(&url)
                .json(&serde_json::json!({ "name": worker_name }))
                .send()
                .await
                .map_err(|e| warn!(error = %e, "Heartbeat echoue"));

            tokio::time::sleep(Duration::from_secs(interval)).await;
        }
    });
}

// ── Shutdown Signal ──

/// Attend un signal d'arret (Ctrl+C ou SIGTERM).
pub async fn shutdown_signal() {
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

// ── Worker Enabled Check ──

/// Vérifie si un worker est activé pour une guild donnée.
/// Retourne `true` par défaut si la clé n'est pas définie (comportement inclusif).
pub async fn is_worker_enabled(pool: &PgPool, guild_id: &str, worker_name: &str) -> bool {
    let result: Option<String> = sqlx::query_scalar(
        "SELECT config_value FROM bot_guild_config \
         WHERE guild_id = $1 AND bot_name = $2 AND config_key = 'enabled'",
    )
    .bind(guild_id)
    .bind(worker_name)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    result.map(|v| v != "false").unwrap_or(true)
}

// ── Periodic Scheduler ──

/// Lance une tache periodique avec gestion du shutdown et reporting d'erreurs.
pub fn spawn_periodic<F>(
    name: &'static str,
    interval_secs: u64,
    pool: PgPool,
    shutdown: tokio::sync::watch::Receiver<bool>,
    api_url: String,
    worker_name: &'static str,
    task_fn: F,
) where
    F: Fn(PgPool) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>>
        + Send
        + 'static,
{
    info!(task = name, interval_secs, "Tache periodique planifiee");

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let interval = Duration::from_secs(interval_secs);

        loop {
            tokio::time::sleep(interval).await;

            if *shutdown.borrow() {
                info!(task = name, "Tache periodique arretee (shutdown)");
                break;
            }

            if let Err(e) = task_fn(pool.clone()).await {
                error!(task = name, error = %e, "Erreur tache periodique");
                let _ = client
                    .post(format!("{}/api/logs", api_url))
                    .json(&serde_json::json!({
                        "level": "error",
                        "bot": worker_name,
                        "message": format!("Erreur job {} : {}", name, e),
                        "category": "worker",
                        "details": {"job": name, "error": e.to_string()},
                    }))
                    .send()
                    .await;
            }
        }
    });
}
