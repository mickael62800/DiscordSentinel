//! Job : VACUUM ANALYZE periodique sur les tables les plus volumineuses.
//! Porte de cleanup-worker (Phase 1 fusion).

use sqlx::PgPool;
use tracing::{info, warn};

const TABLES: &[&str] = &[
    "voice_sessions",
    "audit_logs",
    "infractions",
    "ticket_messages",
    "user_activity_log",
    "logs",
];

pub async fn run(pool: &PgPool) -> Result<(), String> {
    let mut errors = Vec::new();

    for table in TABLES {
        let start = std::time::Instant::now();
        let query = format!("VACUUM ANALYZE {table}");
        match sqlx::query(&query).execute(pool).await {
            Ok(_) => {
                let elapsed = start.elapsed();
                info!(table, duration_ms = elapsed.as_millis() as u64, "VACUUM ANALYZE termine");
            }
            Err(e) => {
                warn!(table, error = %e, "Erreur VACUUM ANALYZE");
                errors.push(format!("{table}: {e}"));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Erreurs VACUUM partielles: {}", errors.join("; ")))
    }
}
