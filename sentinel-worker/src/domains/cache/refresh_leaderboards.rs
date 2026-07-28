use sqlx::PgPool;
use tracing::{info, warn};

/// Phase 2 A.2 — Refresh CONCURRENT des vues materialisees leaderboards.
///
/// `REFRESH MATERIALIZED VIEW CONCURRENTLY` ne pose qu'un verrou ROW EXCLUSIVE
/// (pas ACCESS EXCLUSIVE), ce qui permet aux lectures de continuer pendant
/// le refresh. Necessite l'index UNIQUE cree dans la migration 102.
///
/// Cible toutes les MV de leaderboard une par une. Si une echoue, on log
/// et on continue les autres (best-effort).
pub async fn run(pool: &PgPool) -> Result<(), String> {
    const VIEWS: &[&str] = &[
        "mv_wallet_leaderboard",
        "mv_level_leaderboard",
    ];

    let mut refreshed = 0u32;
    for view in VIEWS {
        let sql = format!("REFRESH MATERIALIZED VIEW CONCURRENTLY {view}");
        match sqlx::query(&sql).execute(pool).await {
            Ok(_) => refreshed += 1,
            Err(e) => warn!(view, error = %e, "REFRESH MATERIALIZED VIEW failed"),
        }
    }

    if refreshed > 0 {
        info!(
            refreshed,
            total = VIEWS.len(),
            "Leaderboards materialized views refreshed"
        );
    }

    Ok(())
}
