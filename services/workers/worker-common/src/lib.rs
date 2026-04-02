//! Infrastructure partagee entre les workers DiscordSentinel.
//!
//! Elimine la duplication de : shutdown signal, lifecycle logging,
//! heartbeat, scheduler, pool creation.

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::signal;
use tracing::{error, info, warn};

// ── Constantes ──

/// Nombre max de connexions PostgreSQL par defaut.
const DEFAULT_PG_MAX_CONNECTIONS: u32 = 5;
/// Timeout d'acquisition de connexion PostgreSQL par defaut (secondes).
const DEFAULT_PG_ACQUIRE_TIMEOUT_SECS: u64 = 5;
/// Intervalle de heartbeat par defaut (secondes).
const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 30;

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
/// Retourne une erreur au lieu de panic si la connexion echoue.
pub async fn create_pg_pool(database_url: &str) -> PgPool {
    let max_connections: u32 = std::env::var("PG_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PG_MAX_CONNECTIONS);

    let acquire_timeout: u64 = std::env::var("PG_ACQUIRE_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PG_ACQUIRE_TIMEOUT_SECS);

    match PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(acquire_timeout))
        .connect(database_url)
        .await
    {
        Ok(pool) => pool,
        Err(e) => {
            error!(error = %e, "Impossible de se connecter a PostgreSQL");
            std::process::exit(1);
        }
    }
}

// ── Lifecycle Logging ──

/// Envoie un log de cycle de vie a l'API.
pub async fn send_lifecycle_log(api_url: &str, worker_name: &str, level: &str, message: &str) {
    if let Err(e) = reqwest::Client::new()
        .post(format!("{}/api/logs", api_url))
        .json(&serde_json::json!({
            "level": level,
            "bot": worker_name,
            "server": "",
            "message": message,
            "category": "worker",
        }))
        .send()
        .await
    {
        warn!(error = %e, worker = worker_name, "Erreur envoi log lifecycle");
    }
}

// ── Heartbeat ──

/// Demarre un heartbeat periodique vers l'API.
pub fn start_heartbeat(api_url: String, worker_name: &'static str) {
    let interval: u64 = std::env::var("HEARTBEAT_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_HEARTBEAT_INTERVAL_SECS);

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let url = format!("{}/api/bots/heartbeat", api_url);

        loop {
            if let Err(e) = client
                .post(&url)
                .json(&serde_json::json!({ "name": worker_name }))
                .send()
                .await
            {
                warn!(error = %e, worker = worker_name, "Heartbeat echoue");
            }

            tokio::time::sleep(Duration::from_secs(interval)).await;
        }
    });
}

// ── Shutdown Signal ──

/// Attend un signal d'arret (Ctrl+C ou SIGTERM).
pub async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = signal::ctrl_c().await {
            error!(error = %e, "Impossible d'ecouter Ctrl+C");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => { sig.recv().await; }
            Err(e) => error!(error = %e, "Impossible d'ecouter SIGTERM"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Signal Ctrl+C recu"),
        _ = terminate => info!("Signal SIGTERM recu"),
    }
}

// ── Worker Enabled Check ──

/// Verifie si un worker est active pour une guild donnee.
/// Retourne `true` par defaut si la cle n'est pas definie (comportement inclusif).
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

/// Constantes de temps utilitaires.
pub const SECS_PER_MINUTE: u64 = 60;
pub const SECS_PER_HOUR: u64 = 3600;

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
                if let Err(log_err) = client
                    .post(format!("{}/api/logs", api_url))
                    .json(&serde_json::json!({
                        "level": "error",
                        "bot": worker_name,
                        "message": format!("Erreur job {} : {}", name, e),
                        "category": "worker",
                        "details": {"job": name, "error": e.to_string()},
                    }))
                    .send()
                    .await
                {
                    warn!(error = %log_err, task = name, "Erreur envoi log d'erreur a l'API");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_constants_are_reasonable() {
        assert_eq!(DEFAULT_PG_MAX_CONNECTIONS, 5);
        assert_eq!(DEFAULT_PG_ACQUIRE_TIMEOUT_SECS, 5);
        assert_eq!(DEFAULT_HEARTBEAT_INTERVAL_SECS, 30);
        assert_eq!(SECS_PER_MINUTE, 60);
        assert_eq!(SECS_PER_HOUR, 3600);
    }

    #[test]
    fn time_constants_coherent() {
        assert_eq!(SECS_PER_HOUR, SECS_PER_MINUTE * 60);
    }
}
