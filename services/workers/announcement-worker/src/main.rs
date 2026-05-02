//! Worker DiscordSentinel — publication automatique des annonces planifiees.
//!
//! Tick toutes les heures pile (calcul du delay au boot pour s'aligner sur
//! HH:MM:00 UTC). A chaque tick :
//! 1. GET /api/announcements/internal/due (l'API select les annonces dues,
//!    cree les runs pending et avance next_run_at).
//! 2. Pour chaque annonce rendue : XADD sur Redis stream `sentinel:events`
//!    event "announcement_publish" avec le payload (channel_ids, content,
//!    embed, mentions_prefix, run_id). Le bot Discord consume cette stream
//!    et poste les messages.
//! 3. Le bot rapporte le resultat via POST /api/announcements/internal/runs/
//!    {run_id}/result -- pas notre boulot ici.

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use std::time::Duration;

use chrono::{Timelike, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use sentinel_worker_common as common;

const WORKER_NAME: &str = "announcement-worker";
const STREAM_KEY: &str = "sentinel:events";
const STREAM_MAXLEN: usize = 10_000;
const FETCH_LIMIT: i64 = 50;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RenderedEmbed {
    title: Option<String>,
    description: String,
    color: Option<i32>,
    image_url: Option<String>,
    thumbnail_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RenderedAnnouncement {
    announcement_id: String,
    run_id: String,
    guild_id: String,
    channel_ids: Vec<String>,
    content_text: String,
    embed: Option<RenderedEmbed>,
    mentions_prefix: String,
}

#[tokio::main]
async fn main() {
    common::init_tracing("info,sentinel_announcement_worker=info");

    let api_url = common::load_api_url();
    let redis_url = common::load_redis_url();

    info!(api_url = %api_url, "Starting {WORKER_NAME}");
    common::send_lifecycle_log(&api_url, WORKER_NAME, "info", "Worker started").await;

    // Heartbeat -> /api/bots/heartbeat (pour le panel de monitoring)
    common::start_heartbeat(api_url.clone(), WORKER_NAME);

    // Calcule le delay jusqu'a la prochaine HH:00:00 (heure pile UTC)
    // pour s'aligner sur les configurations heure-precise des annonces.
    let initial_delay = compute_initial_delay();
    info!(
        delay_secs = initial_delay.as_secs(),
        "Aligning on next hour boundary, sleeping..."
    );
    tokio::time::sleep(initial_delay).await;

    // Init Redis client (connexion paresseuse, ne crash pas si Redis off)
    let redis_client = match redis::Client::open(redis_url.as_str()) {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Impossible d'ouvrir le client Redis");
            common::send_lifecycle_log(
                &api_url,
                WORKER_NAME,
                "error",
                &format!("Redis client init failed: {e}"),
            )
            .await;
            return;
        }
    };

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("HTTP client init");
    let api_key = std::env::var("API_KEY").unwrap_or_default();

    // Tick chaque heure pile
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;
        if let Err(e) =
            run_one_tick(&http_client, &api_url, &api_key, &redis_client).await
        {
            error!(error = %e, "Tick error");
            common::send_worker_log(
                &api_url,
                WORKER_NAME,
                "error",
                "tick",
                &format!("Tick error: {e}"),
                serde_json::json!({ "event_type": "announcement.tick.error", "error": e }),
            )
            .await;
        }
    }
}

fn compute_initial_delay() -> Duration {
    let now = Utc::now();
    // Secondes restantes jusqu'a la prochaine heure pile.
    let secs_in_hour = (now.minute() as u64) * 60 + now.second() as u64;
    let to_next = if secs_in_hour == 0 {
        0
    } else {
        3600 - secs_in_hour
    };
    Duration::from_secs(to_next)
}

async fn run_one_tick(
    http: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    redis_client: &redis::Client,
) -> Result<(), String> {
    let url = format!("{api_url}/api/announcements/internal/due?limit={FETCH_LIMIT}");
    let mut req = http.get(&url);
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("HTTP fetch_due: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("fetch_due returned {status}: {body}"));
    }
    let payloads: Vec<RenderedAnnouncement> = resp
        .json()
        .await
        .map_err(|e| format!("Decode fetch_due response: {e}"))?;

    if payloads.is_empty() {
        info!("No announcements due, skip tick");
        return Ok(());
    }

    info!(count = payloads.len(), "Publishing announcements via Redis stream");

    // Connexion Redis a chaque tick (cheap : pool TCP par tokio runtime).
    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("Redis conn: {e}"))?;

    for p in &payloads {
        let payload_json = serde_json::to_string(&serde_json::json!({
            "event": "announcement_publish",
            "data": p,
        }))
        .map_err(|e| format!("encode payload: {e}"))?;

        // XADD sentinel:events MAXLEN ~ 10000 * payload <json>
        let res: redis::RedisResult<String> = conn
            .xadd_maxlen(
                STREAM_KEY,
                redis::streams::StreamMaxlen::Approx(STREAM_MAXLEN),
                "*",
                &[("payload", payload_json.as_str())],
            )
            .await;
        match res {
            Ok(id) => {
                info!(stream_id = %id, run_id = %p.run_id, channels = p.channel_ids.len(), "XADD success");
            }
            Err(e) => {
                warn!(error = %e, run_id = %p.run_id, "XADD failed");
                common::send_worker_log(
                    api_url,
                    WORKER_NAME,
                    "warn",
                    "tick",
                    "XADD failed",
                    serde_json::json!({
                        "event_type": "announcement.xadd.error",
                        "run_id": p.run_id,
                        "error": e.to_string(),
                    }),
                )
                .await;
            }
        }
    }

    common::send_worker_log(
        api_url,
        WORKER_NAME,
        "info",
        "tick",
        &format!("Published {} announcement(s)", payloads.len()),
        serde_json::json!({
            "event_type": "announcement.tick.success",
            "count": payloads.len(),
        }),
    )
    .await;

    Ok(())
}
