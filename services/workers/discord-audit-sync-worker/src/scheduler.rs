use sqlx::PgPool;
use tokio::sync::watch;

use crate::config::WorkerConfig;
use crate::jobs;
use sentinel_worker_common::spawn_periodic;

pub fn start(config: WorkerConfig, pool: PgPool, shutdown: watch::Receiver<bool>) {
    let api_url = config.api_url.clone();
    let interval = config.sync_interval_secs;
    let token = config.discord_bot_token.clone();

    spawn_periodic(
        "sync_discord_audit_logs",
        interval,
        pool,
        shutdown,
        api_url,
        "discord-audit-sync-worker",
        move |pool| {
            let token = token.clone();
            Box::pin(async move { jobs::sync_discord_audit_logs::run(&pool, &token).await })
        },
    );
}
