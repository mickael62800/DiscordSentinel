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

/// Spawn le job periodique de cloture des lois. Ne bloque pas l'appelant.
pub fn start(api_url: String, close_laws_secs: u64) {
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

    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(close_laws_secs));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            match call_close_laws(&http, &api_url, &api_key).await {
                Ok(report) => {
                    if report.processed > 0 {
                        info!(processed = report.processed, "influence: lois cloturees");
                    }
                }
                Err(e) => {
                    error!(error = %e, "influence: close-laws failed");
                    common::send_worker_log(
                        &api_url,
                        WORKER_NAME,
                        "error",
                        "close-laws",
                        &format!("job close-laws echec: {e}"),
                        serde_json::json!({ "event_type": "influence.close_laws.error", "error": e }),
                    )
                    .await;
                }
            }
        }
    });
}

async fn call_close_laws(
    http: &reqwest::Client,
    api_url: &str,
    api_key: &str,
) -> Result<JobReport, String> {
    let url = format!("{api_url}/api/influence/internal/jobs/close-laws");
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
