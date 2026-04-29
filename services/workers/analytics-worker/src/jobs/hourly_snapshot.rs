use sqlx::PgPool;
use tracing::{debug, info, warn};

use sentinel_worker_common::is_worker_enabled;
use sentinel_api::domain::entities::system::discord_ids::GuildId;

#[derive(sqlx::FromRow)]
struct GuildRow {
    guild_id: GuildId,
}

/// Enregistre un snapshot d'activite horaire pour chaque guild
pub async fn run(pool: &PgPool) -> Result<(), String> {
    let guilds: Vec<GuildRow> =
        sqlx::query_as::<_, GuildRow>("SELECT guild_id FROM guilds ORDER BY name")
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Query guilds: {e}"))?;

    if guilds.is_empty() {
        debug!("Aucune guild, snapshot horaire ignore");
        return Ok(());
    }

    let mut count = 0u64;

    for guild in &guilds {
        if !is_worker_enabled(pool, &guild.guild_id, "analytics-worker").await {
            continue;
        }

        let result = sqlx::query(
            "INSERT INTO hourly_activity (guild_id, day, hour, messages, infractions) \
             SELECT \
               $1, \
               CURRENT_DATE, \
               EXTRACT(HOUR FROM NOW())::smallint, \
               COALESCE((SELECT COUNT(DISTINCT user_id) FROM user_stats \
                 WHERE guild_id = $1 AND updated_at >= date_trunc('hour', NOW())), 0)::bigint, \
               COALESCE((SELECT COUNT(*) FROM infractions \
                 WHERE guild_id = $1 AND created_at >= date_trunc('hour', NOW()))::integer, 0) \
             ON CONFLICT (guild_id, day, hour) DO UPDATE SET \
               messages = EXCLUDED.messages, \
               infractions = EXCLUDED.infractions",
        )
        .bind(&guild.guild_id)
        .execute(pool)
        .await;

        if result.is_ok() {
            count += 1;
        } else if let Err(e) = result {
            warn!(error = %e, guild = %guild.guild_id, "Erreur snapshot horaire");
        }
    }

    if count > 0 {
        info!(guilds = count, "Snapshots horaires enregistres");
    }

    Ok(())
}
