use sqlx::PgPool;
use tokio::sync::watch;
use tracing::{error, info};

use crate::config::WorkerConfig;
use crate::jobs;

/// Lance les tâches périodiques en background
pub fn start(config: &WorkerConfig, pool: PgPool, shutdown: watch::Receiver<bool>) {
    // Régénération des points de conduite
    spawn_periodic(
        "conduct_regen",
        config.conduct_regen_interval_secs,
        pool.clone(),
        shutdown.clone(),
        |pool| Box::pin(async move { jobs::conduct_regen::run(&pool).await }),
    );

    // Nettoyage des bans vocaux expirés
    spawn_periodic(
        "cleanup_bans",
        config.ban_cleanup_interval_secs,
        pool.clone(),
        shutdown.clone(),
        |pool| Box::pin(async move { jobs::cleanup_bans::run(&pool).await }),
    );

    // Snapshots d'activité quotidienne
    spawn_periodic(
        "daily_snapshot",
        config.daily_snapshot_interval_secs,
        pool,
        shutdown,
        |pool| Box::pin(async move { jobs::daily_snapshot::run(&pool).await }),
    );
}

fn spawn_periodic<F>(
    name: &'static str,
    interval_secs: u64,
    pool: PgPool,
    shutdown: watch::Receiver<bool>,
    task_fn: F,
) where
    F: Fn(PgPool) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>>
        + Send
        + 'static,
{
    info!(task = name, interval_secs, "Tâche périodique planifiée");

    tokio::spawn(async move {
        let interval = tokio::time::Duration::from_secs(interval_secs);

        loop {
            tokio::time::sleep(interval).await;

            if *shutdown.borrow() {
                info!(task = name, "Tâche périodique arrêtée (shutdown)");
                break;
            }

            if let Err(e) = task_fn(pool.clone()).await {
                error!(task = name, error = %e, "Erreur tâche périodique");
            }
        }
    });
}
