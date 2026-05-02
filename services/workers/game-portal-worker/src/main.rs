//! Worker DiscordSentinel — Game Portal.
//!
//! Trois jobs en task tokio independantes :
//!   - health_check    : POST /api/games/internal/jobs/health-check (30s)
//!   - idle_shutdown   : POST /api/games/internal/jobs/idle-shutdown (1h)
//!   - reconciler      : POST /api/games/internal/jobs/reconcile (1h)
//!
//! Chaque job a son propre interval, lit sa cadence depuis bot_config
//! `game-portal-worker` (lazy reload toutes les 5 minutes pour eviter de
//! taper la DB a chaque tick).

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use std::time::Duration;

use serde::Deserialize;
use tracing::{error, info, warn};

use sentinel_worker_common as common;

const WORKER_NAME: &str = "game-portal-worker";

#[derive(Debug, Deserialize, Default)]
struct JobReport {
    #[serde(default)]
    job: String,
    #[serde(default)]
    processed: usize,
    #[serde(default)]
    errors: usize,
    #[serde(default)]
    details: serde_json::Value,
}

#[tokio::main]
async fn main() {
    common::init_tracing("info,game_portal_worker=info");
    let api_url = common::load_api_url();
    let api_key = std::env::var("API_KEY").unwrap_or_default();

    info!(api_url = %api_url, "Starting {WORKER_NAME}");
    common::send_lifecycle_log(&api_url, WORKER_NAME, "info", "Worker started").await;
    common::start_heartbeat(api_url.clone(), WORKER_NAME);

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("HTTP client init");

    // 3 tasks paralleles, chacune avec son timer.
    let h1 = spawn_job(
        http.clone(),
        api_url.clone(),
        api_key.clone(),
        "health-check",
        Duration::from_secs(30),
    );
    let h2 = spawn_job(
        http.clone(),
        api_url.clone(),
        api_key.clone(),
        "idle-shutdown",
        Duration::from_secs(3600),
    );
    let h3 = spawn_job(
        http.clone(),
        api_url.clone(),
        api_key.clone(),
        "reconcile",
        Duration::from_secs(3600),
    );
    // Image cleanup : 1 fois par jour. Supprime les images Docker des
    // templates non utilises depuis unused_image_grace_days (config bot
    // game-portal, defaut 7j).
    let h4 = spawn_job(
        http,
        api_url,
        api_key,
        "image-cleanup",
        Duration::from_secs(86400),
    );

    let _ = tokio::join!(h1, h2, h3, h4);
}

fn spawn_job(
    http: reqwest::Client,
    api_url: String,
    api_key: String,
    job: &'static str,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            match call_job(&http, &api_url, &api_key, job).await {
                Ok(report) => {
                    info!(
                        job = job,
                        processed = report.processed,
                        errors = report.errors,
                        "tick OK"
                    );
                    if report.errors > 0 {
                        let _ = common::send_worker_log(
                            &api_url,
                            WORKER_NAME,
                            "warn",
                            job,
                            &format!("job {} : {} erreurs", job, report.errors),
                            serde_json::json!({
                                "event_type": format!("game_portal.{}.errors", job),
                                "report": report.details,
                            }),
                        )
                        .await;
                    }
                }
                Err(e) => {
                    error!(error = %e, job, "tick failed");
                    let _ = common::send_worker_log(
                        &api_url,
                        WORKER_NAME,
                        "error",
                        job,
                        &format!("job {} echec: {}", job, e),
                        serde_json::json!({
                            "event_type": format!("game_portal.{}.error", job),
                            "error": e,
                        }),
                    )
                    .await;
                }
            }
        }
    })
}

async fn call_job(
    http: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    job: &str,
) -> Result<JobReport, String> {
    let url = format!("{api_url}/api/games/internal/jobs/{job}");
    let mut req = http.post(&url);
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("HTTP send: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {status}: {body}"));
    }
    resp.json::<JobReport>()
        .await
        .map_err(|e| format!("decode JobReport: {e}"))
}

#[allow(dead_code)]
fn _force_warn() {
    warn!("link warn");
}
