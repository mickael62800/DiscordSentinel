//! Infrastructure partagee entre les workers DiscordSentinel.
//!
//! Elimine la duplication de : shutdown signal, lifecycle logging,
//! heartbeat, scheduler, pool creation, observabilité Prometheus.

pub mod metrics;

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
///
/// La route `/api/bots/heartbeat` cote API est protegee par l'auth_middleware,
/// on doit donc envoyer l'`API_KEY` en header `Authorization: Bearer` — sinon
/// l'API retourne 401 silencieusement (reqwest::send() considere un 401 comme
/// un succes reseau, donc le worker ne log meme pas l'erreur). L'API_KEY est
/// lue depuis l'env au demarrage du heartbeat.
pub fn start_heartbeat(api_url: String, worker_name: &'static str) {
    let interval: u64 = std::env::var("HEARTBEAT_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_HEARTBEAT_INTERVAL_SECS);

    let api_key = std::env::var("API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        warn!(
            worker = worker_name,
            "API_KEY non definie — les heartbeats seront rejetes avec 401"
        );
    }

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let url = format!("{}/api/bots/heartbeat", api_url);

        loop {
            let req = client
                .post(&url)
                .bearer_auth(&api_key)
                .json(&serde_json::json!({ "name": worker_name }));

            match req.send().await {
                Ok(resp) if resp.status().is_success() => {}
                Ok(resp) => {
                    warn!(
                        status = %resp.status(),
                        worker = worker_name,
                        "Heartbeat rejete par l'API"
                    );
                }
                Err(e) => {
                    warn!(error = %e, worker = worker_name, "Heartbeat echoue");
                }
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

/// Verifie si le worker est active pour au moins une guild.
/// Retourne true si:
/// - Aucune entree `enabled` trouvee (defaut = active)
/// - Au moins une entree `enabled = true`
/// Retourne false si toutes les entrees trouvees sont `enabled = false`.
pub async fn is_worker_globally_enabled(pool: &PgPool, worker_name: &str) -> bool {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT config_value FROM bot_guild_config \
         WHERE bot_name = $1 AND config_key = 'enabled'",
    )
    .bind(worker_name)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return true; // Pas de config = active par defaut
    }
    rows.iter().any(|(v,)| v == "true" || v == "1")
}

/// Constantes de temps utilitaires.
pub const SECS_PER_MINUTE: u64 = 60;
pub const SECS_PER_HOUR: u64 = 3600;

// ── Config Helpers ──

/// Charge DATABASE_URL depuis l'environnement. Exit si absent.
pub fn load_database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        error!("DATABASE_URL non defini");
        std::process::exit(1);
    })
}

/// Charge API_URL depuis l'environnement avec fallback localhost.
pub fn load_api_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:3000".into())
}

/// Charge REDIS_URL depuis l'environnement avec fallback localhost.
pub fn load_redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into())
}

// ── DB Config Loading ──

/// Charge toute la config d'un worker depuis la table bot_guild_config.
/// Retourne un HashMap<config_key, config_value> (toutes les guilds mergees, global).
/// Les workers n'ont pas de guild_id specifique — on charge la config "globale"
/// (premiere valeur trouvee pour chaque cle).
pub async fn load_worker_config(pool: &PgPool, worker_name: &str) -> std::collections::HashMap<String, String> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT config_key, config_value FROM bot_guild_config WHERE bot_name = $1 ORDER BY updated_at DESC",
    )
    .bind(worker_name)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut map = std::collections::HashMap::new();
    for (key, value) in rows {
        map.entry(key).or_insert(value);
    }
    map
}

/// Lit une valeur depuis la config DB, sinon env var, sinon defaut.
pub fn config_or_env<T: std::str::FromStr>(
    db_config: &std::collections::HashMap<String, String>,
    db_key: &str,
    env_key: &str,
    default: T,
) -> T {
    // Priorite 1 : config DB
    if let Some(val) = db_config.get(db_key) {
        if let Ok(parsed) = val.parse() {
            return parsed;
        }
    }
    // Priorite 2 : env var
    if let Ok(val) = std::env::var(env_key) {
        if let Ok(parsed) = val.parse() {
            return parsed;
        }
    }
    // Priorite 3 : defaut
    default
}

/// Version bool de config_or_env.
pub fn config_or_env_bool(
    db_config: &std::collections::HashMap<String, String>,
    db_key: &str,
    env_key: &str,
    default: bool,
) -> bool {
    if let Some(val) = db_config.get(db_key) {
        return val == "true" || val == "1";
    }
    match std::env::var(env_key) {
        Ok(v) => v == "true" || v == "1",
        Err(_) => default,
    }
}

/// Charge une variable d'environnement avec un fallback par defaut.
pub fn load_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Charge une variable d'environnement booleenne (accepte "true"/"1").
pub fn load_env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => v == "true" || v == "1",
        Err(_) => default,
    }
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

            // Verifie le flag enabled en DB avant chaque tick.
            // Si toutes les guilds ont desactive ce worker, on skip la tache.
            if !is_worker_globally_enabled(&pool, worker_name).await {
                tracing::debug!(
                    task = name,
                    worker = worker_name,
                    "Worker desactive via config, skip tick"
                );
                continue;
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

    // ── config_or_env tests ──

    #[test]
    fn config_or_env_db_takes_priority() {
        let mut db = std::collections::HashMap::new();
        db.insert("my_key".into(), "42".into());
        let result: u64 = config_or_env(&db, "my_key", "NONEXISTENT_ENV_VAR_XYZ", 99);
        assert_eq!(result, 42);
    }

    #[test]
    fn config_or_env_falls_back_to_default() {
        let db = std::collections::HashMap::new();
        let result: u64 = config_or_env(&db, "missing", "NONEXISTENT_ENV_VAR_XYZ", 99);
        assert_eq!(result, 99);
    }

    #[test]
    fn config_or_env_invalid_db_value_falls_back() {
        let mut db = std::collections::HashMap::new();
        db.insert("key".into(), "not_a_number".into());
        let result: u64 = config_or_env(&db, "key", "NONEXISTENT_ENV_VAR_XYZ", 50);
        assert_eq!(result, 50);
    }

    #[test]
    fn config_or_env_bool_db_true() {
        let mut db = std::collections::HashMap::new();
        db.insert("flag".into(), "true".into());
        assert!(config_or_env_bool(&db, "flag", "NONEXISTENT_ENV_VAR_XYZ", false));
    }

    #[test]
    fn config_or_env_bool_db_false() {
        let mut db = std::collections::HashMap::new();
        db.insert("flag".into(), "false".into());
        assert!(!config_or_env_bool(&db, "flag", "NONEXISTENT_ENV_VAR_XYZ", true));
    }

    #[test]
    fn config_or_env_bool_db_one() {
        let mut db = std::collections::HashMap::new();
        db.insert("flag".into(), "1".into());
        assert!(config_or_env_bool(&db, "flag", "NONEXISTENT_ENV_VAR_XYZ", false));
    }

    #[test]
    fn config_or_env_bool_missing_uses_default() {
        let db = std::collections::HashMap::new();
        assert!(config_or_env_bool(&db, "missing", "NONEXISTENT_ENV_VAR_XYZ", true));
        assert!(!config_or_env_bool(&db, "missing", "NONEXISTENT_ENV_VAR_XYZ", false));
    }
}
