pub mod cleanup_bans;
pub mod conduct_regen;
pub mod daily_snapshot;

use crate::queue::Job;

/// Dispatch un job vers le handler approprié
pub async fn dispatch(job: &Job, pool: &sqlx::PgPool) -> Result<(), String> {
    match job.job_type.as_str() {
        "conduct_regen" => conduct_regen::run(pool).await,
        "cleanup_bans" => cleanup_bans::run(pool).await,
        "daily_snapshot" => daily_snapshot::run(pool).await,
        other => {
            tracing::warn!(job_type = %other, "Type de job inconnu, ignoré");
            Ok(())
        }
    }
}
