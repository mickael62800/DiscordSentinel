use sqlx::PgPool;
use tracing::{debug, info};

#[derive(sqlx::FromRow)]
struct GuildRow {
    guild_id: String,
}

/// Enregistre un snapshot d'activité quotidienne pour chaque guild
pub async fn run(pool: &PgPool) -> Result<(), String> {
    let guilds: Vec<GuildRow> =
        sqlx::query_as::<_, GuildRow>("SELECT guild_id FROM guilds ORDER BY name")
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Query guilds: {e}"))?;

    if guilds.is_empty() {
        debug!("Aucune guild, snapshot ignoré");
        return Ok(());
    }

    let mut count = 0u64;

    for guild in &guilds {
        let result = sqlx::query(
            "INSERT INTO daily_activity (guild_id, day, messages, voice_minutes, active_members, infractions, warns, mutes, bans) \
             SELECT \
               $1, \
               CURRENT_DATE, \
               COALESCE((SELECT SUM(message_count) FROM user_stats WHERE guild_id = $1), 0), \
               COALESCE((SELECT SUM(voice_seconds) / 60 FROM user_stats WHERE guild_id = $1), 0), \
               COALESCE((SELECT COUNT(DISTINCT user_id) FROM user_stats WHERE guild_id = $1 AND updated_at >= CURRENT_DATE), 0)::integer, \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= CURRENT_DATE)::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= CURRENT_DATE AND action = 'warn')::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= CURRENT_DATE AND action = 'mute')::integer, 0), \
               COALESCE((SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND created_at >= CURRENT_DATE AND action = 'ban')::integer, 0) \
             ON CONFLICT (guild_id, day) DO UPDATE SET \
               messages = EXCLUDED.messages, \
               voice_minutes = EXCLUDED.voice_minutes, \
               active_members = EXCLUDED.active_members, \
               infractions = EXCLUDED.infractions, \
               warns = EXCLUDED.warns, \
               mutes = EXCLUDED.mutes, \
               bans = EXCLUDED.bans",
        )
        .bind(&guild.guild_id)
        .execute(pool)
        .await;

        if result.is_ok() {
            count += 1;
        } else if let Err(e) = result {
            debug!(error = %e, guild = %guild.guild_id, "Erreur snapshot activité");
        }
    }

    if count > 0 {
        info!(guilds = count, "Snapshots activité quotidienne enregistrés");
    }

    Ok(())
}
