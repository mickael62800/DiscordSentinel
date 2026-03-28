use sqlx::PgPool;
use tokio::sync::watch;
use tracing::{error, info};

use crate::config::WorkerConfig;
use crate::jobs;

pub fn start(config: &WorkerConfig, pool: PgPool, shutdown: watch::Receiver<bool>) {
    let api_url = config.api_url.clone();

    spawn_periodic(
        "daily_snapshot",
        config.daily_snapshot_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        |pool| Box::pin(async move { jobs::daily_snapshot::run(&pool).await }),
    );

    spawn_periodic(
        "hourly_snapshot",
        config.hourly_snapshot_interval_secs,
        pool,
        shutdown,
        api_url,
        |pool| Box::pin(async move { jobs::hourly_snapshot::run(&pool).await }),
    );
}

fn spawn_periodic<F>(
    name: &'static str,
    interval_secs: u64,
    pool: PgPool,
    shutdown: watch::Receiver<bool>,
    api_url: String,
    task_fn: F,
) where
    F: Fn(PgPool) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>>
        + Send
        + 'static,
{
    info!(task = name, interval_secs, "Tache periodique planifiee");

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let interval = tokio::time::Duration::from_secs(interval_secs);

        loop {
            tokio::time::sleep(interval).await;

            if *shutdown.borrow() {
                info!(task = name, "Tache periodique arretee (shutdown)");
                break;
            }

            if let Err(e) = task_fn(pool.clone()).await {
                error!(task = name, error = %e, "Erreur tache periodique");
                let _ = client.post(format!("{}/api/logs", api_url))
                    .json(&serde_json::json!({
                        "level": "error",
                        "bot": "analytics-worker",
                        "message": format!("Erreur job {} : {}", name, e),
                        "category": "worker",
                        "details": {"job": name, "error": e.to_string()},
                    }))
                    .send().await;
            }
        }
    });
}
