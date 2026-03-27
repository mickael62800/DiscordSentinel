use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    #[serde(rename = "type")]
    pub job_type: String,
    #[serde(default)]
    pub payload: serde_json::Value,
    #[serde(default = "chrono::Utc::now")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Job {
    pub fn new(job_type: &str, payload: serde_json::Value) -> Self {
        Self {
            job_type: job_type.to_string(),
            payload,
            created_at: chrono::Utc::now(),
        }
    }
}

/// Enqueue un job dans la queue Redis (LPUSH)
pub async fn enqueue(
    redis: &redis::Client,
    queue_key: &str,
    job: &Job,
) -> Result<(), String> {
    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("Redis connection: {e}"))?;

    let json = serde_json::to_string(job)
        .map_err(|e| format!("Serialization: {e}"))?;

    conn.lpush::<_, _, ()>(queue_key, &json)
        .await
        .map_err(|e| format!("LPUSH: {e}"))?;

    debug!(job_type = %job.job_type, "Job enqueued");
    Ok(())
}

/// Boucle de consommation : BRPOP bloquant avec timeout
pub async fn consume_loop(
    redis: &redis::Client,
    queue_key: String,
    pg_pool: sqlx::PgPool,
    shutdown: tokio::sync::watch::Receiver<bool>,
) {
    tracing::info!(queue = %queue_key, "Consumer de jobs démarré");

    loop {
        if *shutdown.borrow() {
            tracing::info!("Consumer arrêté (shutdown)");
            break;
        }

        // BRPOP avec timeout 2s pour vérifier le shutdown régulièrement
        let result: Result<Option<(String, String)>, _> = async {
            let mut conn = redis
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| format!("Redis: {e}"))?;

            let val: Option<(String, String)> = redis::cmd("BRPOP")
                .arg(&queue_key)
                .arg(2) // timeout 2s
                .query_async(&mut conn)
                .await
                .map_err(|e| format!("BRPOP: {e}"))?;

            Ok(val)
        }
        .await;

        match result {
            Ok(Some((_key, json))) => {
                match serde_json::from_str::<Job>(&json) {
                    Ok(job) => {
                        tracing::info!(job_type = %job.job_type, "Job reçu");
                        if let Err(e) = super::jobs::dispatch(&job, &pg_pool).await {
                            error!(job_type = %job.job_type, error = %e, "Erreur traitement job");
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, raw = %json, "Job invalide, ignoré");
                    }
                }
            }
            Ok(None) => {} // timeout, pas de job
            Err(e) => {
                error!(error = %e, "Erreur BRPOP, retry dans 2s");
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }
}
