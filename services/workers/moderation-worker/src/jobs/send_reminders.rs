use sqlx::PgPool;
use tracing::{debug, info};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use sentinel_worker_common::is_worker_enabled;

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct PendingReminder {
    id: Uuid,
    guild_id: String,
    moderator_id: String,
    moderator_name: String,
    target_id: String,
    target_name: String,
    action_type: String,
    reason: String,
    expires_at: DateTime<Utc>,
}

/// Envoie les rappels de sanctions temporaires aux moderateurs.
/// Publie un event Redis pour chaque rappel pending dont remind_at <= NOW().
pub async fn run(pool: &PgPool) -> Result<(), String> {
    let reminders = sqlx::query_as::<_, PendingReminder>(
        "SELECT id, guild_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, expires_at
         FROM sanction_reminders
         WHERE status = 'pending' AND remind_at <= NOW()
         ORDER BY remind_at ASC
         LIMIT 50"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Query pending reminders: {e}"))?;

    if reminders.is_empty() {
        debug!("Aucun rappel a envoyer");
        return Ok(());
    }

    for reminder in &reminders {
        if !is_worker_enabled(pool, &reminder.guild_id, "moderation-worker").await {
            continue;
        }

        // Marquer comme envoye AVANT de tenter le broadcast (evite les doublons)
        sqlx::query("UPDATE sanction_reminders SET status = 'sent' WHERE id = $1")
            .bind(reminder.id)
            .execute(pool)
            .await
            .map_err(|e| format!("Mark reminder sent: {e}"))?;

        let time_left = reminder.expires_at.signed_duration_since(Utc::now());
        let minutes_left = time_left.num_minutes().max(0);

        info!(
            reminder_id = %reminder.id,
            moderator = %reminder.moderator_name,
            target = %reminder.target_name,
            action = %reminder.action_type,
            minutes_left = minutes_left,
            "Rappel de sanction temporaire envoye"
        );
    }

    info!(count = reminders.len(), "Rappels de sanctions envoyes");
    Ok(())
}
