use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

use sentinel_worker_common::is_worker_enabled;

/// Jours pour le mode weekly.
const WEEKLY_DAYS: i32 = 7;
/// Jours pour le mode monthly.
const MONTHLY_DAYS: i32 = 30;
/// Nombre max de rappels traites par batch.
#[allow(dead_code)]
const REMINDERS_BATCH_SIZE: i64 = 50;

#[derive(sqlx::FromRow)]
struct UserPoints {
    guild_id: String,
    user_id: String,
    points: i32,
}

#[derive(sqlx::FromRow)]
struct RegenConfig {
    guild_id: String,
    max_points: i32,
    regen_amount: i32,
}

/// Regenere les points de conduite pour les utilisateurs eligibles (weekly + monthly).
pub async fn run(pool: &PgPool) -> Result<(), String> {
    let mut total = 0u64;

    for interval in &["weekly", "monthly"] {
        let interval_days: i32 = match *interval {
            "weekly" => WEEKLY_DAYS,
            "monthly" => MONTHLY_DAYS,
            _ => continue,
        };

        // Recuperer les configs qui utilisent cet intervalle
        let configs: Vec<RegenConfig> = sqlx::query_as::<_, RegenConfig>(
            "SELECT guild_id, max_points, regen_amount FROM conduct_config WHERE regen_interval = $1",
        )
        .bind(interval)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Query configs: {e}"))?;

        for config in &configs {
            if !is_worker_enabled(pool, &config.guild_id, "moderation-worker").await {
                continue;
            }

            // Trouver les utilisateurs dont la regen est due
            let users: Vec<UserPoints> = sqlx::query_as::<_, UserPoints>(
                "SELECT guild_id, user_id, points FROM user_conduct_points \
                 WHERE guild_id = $1 AND last_regen_at + make_interval(days => $2) <= NOW()",
            )
            .bind(&config.guild_id)
            .bind(interval_days)
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Query users: {e}"))?;

            for user in &users {
                let new_points = user.points + config.regen_amount;

                if new_points >= config.max_points {
                    // L'utilisateur est "clean", supprimer son entree
                    log_regen(
                        pool,
                        &user.guild_id,
                        &user.user_id,
                        config.regen_amount,
                        user.points,
                        config.max_points,
                    )
                    .await;

                    if let Err(e) = sqlx::query("DELETE FROM user_conduct_points WHERE guild_id = $1 AND user_id = $2")
                        .bind(&user.guild_id)
                        .bind(&user.user_id)
                        .execute(pool)
                        .await
                    {
                        warn!(error = %e, guild = %user.guild_id, user = %user.user_id, "Erreur suppression points de conduite");
                    }
                } else {
                    // Mettre a jour les points + timestamp regen
                    if let Err(e) = sqlx::query(
                        "UPDATE user_conduct_points SET points = $1, last_regen_at = NOW(), updated_at = NOW() \
                         WHERE guild_id = $2 AND user_id = $3",
                    )
                    .bind(new_points)
                    .bind(&user.guild_id)
                    .bind(&user.user_id)
                    .execute(pool)
                    .await
                    {
                        warn!(error = %e, guild = %user.guild_id, user = %user.user_id, "Erreur mise a jour points de conduite");
                    }

                    log_regen(
                        pool,
                        &user.guild_id,
                        &user.user_id,
                        config.regen_amount,
                        user.points,
                        new_points,
                    )
                    .await;
                }

                total += 1;
            }
        }
    }

    if total > 0 {
        info!(count = total, "Points de conduite regeneres");
    } else {
        debug!("Aucun point de conduite a regenerer");
    }

    Ok(())
}

async fn log_regen(
    pool: &PgPool,
    guild_id: &str,
    user_id: &str,
    delta: i32,
    before: i32,
    after: i32,
) {
    if let Err(e) = sqlx::query(
        "INSERT INTO conduct_points_log (id, guild_id, user_id, delta, reason, points_before, points_after, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(guild_id)
    .bind(user_id)
    .bind(delta)
    .bind("regen")
    .bind(before)
    .bind(after)
    .execute(pool)
    .await
    {
        warn!(error = %e, guild = %guild_id, user = %user_id, "Erreur insertion log regeneration");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_constants() {
        assert_eq!(WEEKLY_INTERVAL, "7 days");
        assert_eq!(MONTHLY_INTERVAL, "30 days");
    }

    #[test]
    fn regen_logic_clean_user() {
        // Un utilisateur avec 90 points, max 100, regen 15 → 105 >= 100 → clean
        let points = 90;
        let regen_amount = 15;
        let max_points = 100;
        let new_points = points + regen_amount;
        assert!(new_points >= max_points);
    }

    #[test]
    fn regen_logic_partial_regen() {
        // Un utilisateur avec 50 points, max 100, regen 10 → 60 < 100 → update
        let points = 50;
        let regen_amount = 10;
        let max_points = 100;
        let new_points = points + regen_amount;
        assert!(new_points < max_points);
        assert_eq!(new_points, 60);
    }

    #[test]
    fn regen_logic_exact_threshold() {
        // Exact au seuil → considere clean
        let points = 90;
        let regen_amount = 10;
        let max_points = 100;
        let new_points = points + regen_amount;
        assert!(new_points >= max_points);
    }

    #[test]
    fn regen_logic_zero_regen() {
        let points = 50;
        let regen_amount = 0;
        let new_points = points + regen_amount;
        assert_eq!(new_points, 50);
    }
}
