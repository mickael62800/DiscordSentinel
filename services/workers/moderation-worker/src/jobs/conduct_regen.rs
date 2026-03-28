use sqlx::PgPool;
use tracing::{debug, info};
use uuid::Uuid;

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

/// Régénère les points de conduite pour les utilisateurs éligibles (weekly + monthly)
pub async fn run(pool: &PgPool) -> Result<(), String> {
    let mut total = 0u64;

    for interval in &["weekly", "monthly"] {
        let interval_expr = match *interval {
            "weekly" => "7 days",
            "monthly" => "30 days",
            _ => continue,
        };

        // Récupérer les configs qui utilisent cet intervalle
        let configs: Vec<RegenConfig> = sqlx::query_as::<_, RegenConfig>(
            "SELECT guild_id, max_points, regen_amount FROM conduct_config WHERE regen_interval = $1",
        )
        .bind(interval)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Query configs: {e}"))?;

        for config in &configs {
            // Trouver les utilisateurs dont la regen est due
            let query = format!(
                "SELECT guild_id, user_id, points FROM user_conduct_points \
                 WHERE guild_id = $1 AND last_regen_at + INTERVAL '{}' <= NOW()",
                interval_expr
            );

            let users: Vec<UserPoints> = sqlx::query_as::<_, UserPoints>(&query)
                .bind(&config.guild_id)
                .fetch_all(pool)
                .await
                .map_err(|e| format!("Query users: {e}"))?;

            for user in &users {
                let new_points = user.points + config.regen_amount;

                if new_points >= config.max_points {
                    // L'utilisateur est "clean", supprimer son entrée
                    log_regen(
                        pool,
                        &user.guild_id,
                        &user.user_id,
                        config.regen_amount,
                        user.points,
                        config.max_points,
                    )
                    .await;

                    sqlx::query("DELETE FROM user_conduct_points WHERE guild_id = $1 AND user_id = $2")
                        .bind(&user.guild_id)
                        .bind(&user.user_id)
                        .execute(pool)
                        .await
                        .ok();
                } else {
                    // Mettre à jour les points + timestamp regen
                    sqlx::query(
                        "UPDATE user_conduct_points SET points = $1, last_regen_at = NOW(), updated_at = NOW() \
                         WHERE guild_id = $2 AND user_id = $3",
                    )
                    .bind(new_points)
                    .bind(&user.guild_id)
                    .bind(&user.user_id)
                    .execute(pool)
                    .await
                    .ok();

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
        info!(count = total, "Points de conduite régénérés");
    } else {
        debug!("Aucun point de conduite à régénérer");
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
    let _ = sqlx::query(
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
    .await;
}
