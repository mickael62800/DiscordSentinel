//! Jeu Influence — job « monde vivant » : cloture des lois arrivees a echeance.
//!
//! POST periodique sur /api/influence/internal/jobs/close-laws. L'API cloture
//! les lois et diffuse `influence_law_closed` (le bot edite le message).

use std::time::Duration;

use serde::Deserialize;
use tracing::{error, info};

use crate::common;

const WORKER_NAME: &str = "influence";

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
struct JobReport {
    #[serde(default)]
    job: String,
    #[serde(default)]
    processed: usize,
    #[serde(default)]
    errors: usize,
}

/// Spawn les jobs periodiques Influence (cloture des lois + resolution des
/// enquetes). Ne bloque pas l'appelant.
pub fn start(api_url: String, interval_secs: u64) {
    let api_key = std::env::var("API_KEY").unwrap_or_default();
    let http = match reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "influence: HTTP client init failed");
            return;
        }
    };

    spawn_job(http.clone(), api_url.clone(), api_key.clone(), "close-laws", interval_secs);
    spawn_job(http, api_url, api_key, "resolve-investigations", interval_secs);
}

fn spawn_job(
    http: reqwest::Client,
    api_url: String,
    api_key: String,
    job: &'static str,
    interval_secs: u64,
) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(interval_secs));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            match call_job(&http, &api_url, &api_key, job).await {
                Ok(report) => {
                    if report.processed > 0 {
                        info!(job, processed = report.processed, "influence: tick OK");
                    }
                }
                Err(e) => {
                    error!(error = %e, job, "influence: job failed");
                    common::send_worker_log(
                        &api_url,
                        WORKER_NAME,
                        "error",
                        job,
                        &format!("job {job} echec: {e}"),
                        serde_json::json!({ "event_type": format!("influence.{job}.error"), "error": e }),
                    )
                    .await;
                }
            }
        }
    });
}

async fn call_job(
    http: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    job: &str,
) -> Result<JobReport, String> {
    let url = format!("{api_url}/api/influence/internal/jobs/{job}");
    let mut req = http.post(&url);
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    let resp = req.send().await.map_err(|e| format!("HTTP send: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {status}: {body}"));
    }
    resp.json::<JobReport>()
        .await
        .map_err(|e| format!("decode JobReport: {e}"))
}
