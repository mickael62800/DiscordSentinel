#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod config;
mod jobs;
mod scheduler;

use tokio::sync::watch;
use tracing::info;

use crate::config::WorkerConfig;
use sentinel_worker_common as common;

const WORKER_NAME: &str = "discord-audit-sync-worker";

#[tokio::main]
async fn main() {
    common::init_tracing("sentinel_discord_audit_sync_worker=info");
    common::metrics::init_observability(WORKER_NAME);

    let mut config = WorkerConfig::from_env();

    info!("Demarrage de Sentinel Discord Audit Sync Worker");

    if config.discord_bot_token.is_empty() {
        tracing::error!(
            "DISCORD_BOT_TOKEN manquant. Le worker a besoin d'un bot token \
             avec VIEW_AUDIT_LOG pour appeler l'API Discord. Exit."
        );
        std::process::exit(1);
    }

    let pg_pool = common::create_pg_pool(&config.database_url).await;
    info!("PostgreSQL connecte");

    let db_config = common::load_worker_config(&pg_pool, WORKER_NAME).await;
    if !db_config.is_empty() {
        config.apply_db_config(&db_config);
    }

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    scheduler::start(config.clone(), pg_pool.clone(), shutdown_rx);
    common::start_heartbeat(config.api_url.clone(), WORKER_NAME);

    common::send_lifecycle_log(
        &config.api_url,
        WORKER_NAME,
        "info",
        "Discord Audit Sync Worker demarre",
    )
    .await;

    info!("Sentinel Discord Audit Sync Worker pret");

    common::shutdown_signal().await;

    common::send_lifecycle_log(
        &config.api_url,
        WORKER_NAME,
        "warn",
        "Discord Audit Sync Worker en cours d'arret",
    )
    .await;

    info!("Arret en cours...");
    let _ = shutdown_tx.send(true);

    pg_pool.close().await;
    info!("Sentinel Discord Audit Sync Worker arrete proprement");
}
