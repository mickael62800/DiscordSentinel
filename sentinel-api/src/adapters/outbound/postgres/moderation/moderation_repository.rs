use crate::adapters::outbound::postgres::pg_ctx;
use crate::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

use crate::ports::outbound::moderation::moderation_repository::ModerationRepository;
use sentinel_core::domain::entities::moderation::action::applied::ModerationAction;
use sentinel_core::domain::enums::moderation::moderation_gravity::ModerationGravity;
use sentinel_core::domain::errors::DomainError;

/// Phase 2 helper : reconstruit une ModerationAction a partir d'une ligne
/// audit_logs (event_type `mod_*`).
#[derive(sqlx::FromRow)]
struct AuditModRow {
    id: Uuid,
    guild_id: String,
    event_type: String,
    actor_id: Option<String>,
    actor_name: Option<String>,
    target_id: Option<String>,
    target_name: Option<String>,
    /// Lu uniquement par les queries qui font le LEFT JOIN guild_members.
    /// Default = None pour les autres queries (l'aliasing est explicite cote SQL).
    #[sqlx(default)]
    target_display_name: Option<String>,
    channel_id: Option<String>,
    details: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<AuditModRow> for ModerationAction {
    fn from(row: AuditModRow) -> Self {
        let action_type = row
            .event_type
            .strip_prefix("mod_")
            .unwrap_or(&row.event_type)
            .to_string();
        let reason = row
            .details
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let gravity = row
            .details
            .get("gravity")
            .and_then(|v| v.as_str())
            .and_then(ModerationGravity::from_str_lossy);
        // Negative duration → None (ne wrap pas sur u64::MAX).
        let duration = row
            .details
            .get("duration_secs")
            .and_then(|v| v.as_i64())
            .and_then(|d| u64::try_from(d).ok());
        // Si details.action_id existe, on l'utilise pour conserver l'identite
        // historique (Phase 4 : sera l'id audit_log lui-meme).
        let id = row
            .details
            .get("action_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::from_str(s).ok())
            .unwrap_or(row.id);
        Self {
            id,
            guild_id: row.guild_id.into(),
            channel_id: row.channel_id.unwrap_or_default().into(),
            moderator_id: row.actor_id.unwrap_or_default(),
            moderator_name: row.actor_name.unwrap_or_default(),
            target_id: row.target_id.unwrap_or_default(),
            target_name: row.target_name.unwrap_or_default(),
            target_display_name: row.target_display_name,
            action_type,
            reason,
            gravity,
            duration,
            created_at: row.created_at,
        }
    }
}

const AUDIT_MOD_SELECT: &str =
    "SELECT id, guild_id, event_type, actor_id, actor_name, target_id, target_name, channel_id, details, created_at FROM audit_logs";

pub struct PgModerationRepository {
    pool: PgPool,
}

impl PgModerationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ActionRow {
    id: Uuid,
    guild_id: String,
    channel_id: String,
    moderator_id: String,
    moderator_name: String,
    target_id: String,
    target_name: String,
    action_type: String,
    reason: String,
    gravity: Option<crate::adapters::outbound::postgres::types::PgModerationGravity>,
    duration: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<ActionRow> for ModerationAction {
    fn from(row: ActionRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id.into(),
            channel_id: row.channel_id.into(),
            moderator_id: row.moderator_id,
            moderator_name: row.moderator_name,
            target_id: row.target_id,
            target_name: row.target_name,
            target_display_name: None,
            action_type: row.action_type,
            reason: row.reason,
            gravity: row.gravity.map(Into::into),
            duration: row.duration.and_then(|d| u64::try_from(d).ok()),
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl ModerationRepository for PgModerationRepository {
    async fn save(&self, _action: &ModerationAction) -> Result<(), DomainError> {
        // Phase 4 : on n'ecrit plus dans `moderation_actions`. Le dual-write
        // dans `audit_logs` est gere par ManageModerationService::log_action
        // via audit_logs_uc. Cette methode est conservee pour ne pas casser
        // l'interface ModerationRepository, mais devient un no-op.
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<ModerationAction>, DomainError> {
        // Phase 4 : on cherche par details->>'action_id' dans audit_logs.
        let row = sqlx::query_as::<_, AuditModRow>(
            "SELECT id, guild_id, event_type, actor_id, actor_name, target_id, target_name, channel_id, details, created_at \
             FROM audit_logs \
             WHERE event_type LIKE 'mod_%' AND details->>'action_id' = $1 \
             LIMIT 1",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(row.map(ModerationAction::from))
    }

    async fn find_by_target(
        &self,
        guild_id: &str,
        target_id: &str,
        limit: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        let limit = limit.clamp(1, 1000);
        let sql = format!(
            "{AUDIT_MOD_SELECT} WHERE guild_id = $1 AND target_id = $2 AND event_type LIKE 'mod_%' ORDER BY created_at DESC LIMIT {limit}"
        );
        let rows = sqlx::query_as::<_, AuditModRow>(&sql)
            .bind(guild_id)
            .bind(target_id)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(rows.into_iter().map(ModerationAction::from).collect())
    }

    async fn find_bans(
        &self,
        guild_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        // Phase 2 : lecture depuis audit_logs.
        // Pour chaque (guild_id, target_id), on prend la derniere action ban*/unban
        // et on ne garde que celles dont l'event_type final commence par 'mod_ban'.
        // LEFT JOIN guild_members pour enrichir avec le pseudo serveur
        // (target_display_name) — affiche dans la liste "Bannis actifs" cote web.
        let rows = match guild_id {
            Some(gid) => {
                sqlx::query_as::<_, AuditModRow>(
                    "SELECT latest.id, latest.guild_id, latest.event_type, latest.actor_id, latest.actor_name, \
                            latest.target_id, latest.target_name, gm.display_name AS target_display_name, \
                            latest.channel_id, latest.details, latest.created_at \
                     FROM ( \
                        SELECT DISTINCT ON (guild_id, target_id) \
                            id, guild_id, event_type, actor_id, actor_name, target_id, target_name, channel_id, details, created_at \
                        FROM audit_logs \
                        WHERE guild_id = $1 \
                          AND target_id IS NOT NULL \
                          AND (event_type LIKE 'mod_ban%' OR event_type = 'mod_unban') \
                        ORDER BY guild_id, target_id, created_at DESC \
                     ) latest \
                     LEFT JOIN guild_members gm \
                         ON gm.guild_id = latest.guild_id AND gm.user_id = latest.target_id \
                     WHERE latest.event_type LIKE 'mod_ban%' \
                     ORDER BY latest.created_at DESC \
                     LIMIT $2 OFFSET $3",
                )
                .bind(gid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, AuditModRow>(
                    "SELECT latest.id, latest.guild_id, latest.event_type, latest.actor_id, latest.actor_name, \
                            latest.target_id, latest.target_name, gm.display_name AS target_display_name, \
                            latest.channel_id, latest.details, latest.created_at \
                     FROM ( \
                        SELECT DISTINCT ON (guild_id, target_id) \
                            id, guild_id, event_type, actor_id, actor_name, target_id, target_name, channel_id, details, created_at \
                        FROM audit_logs \
                        WHERE target_id IS NOT NULL \
                          AND (event_type LIKE 'mod_ban%' OR event_type = 'mod_unban') \
                        ORDER BY guild_id, target_id, created_at DESC \
                     ) latest \
                     LEFT JOIN guild_members gm \
                         ON gm.guild_id = latest.guild_id AND gm.user_id = latest.target_id \
                     WHERE latest.event_type LIKE 'mod_ban%' \
                     ORDER BY latest.created_at DESC \
                     LIMIT $1 OFFSET $2",
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(ModerationAction::from).collect())
    }

    async fn find_all_for_guild(
        &self,
        guild_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        // Phase 2 : lecture depuis audit_logs.
        let rows = match guild_id {
            Some(gid) => {
                let sql = format!(
                    "{AUDIT_MOD_SELECT} WHERE guild_id = $1 AND event_type LIKE 'mod_%' ORDER BY created_at DESC LIMIT $2"
                );
                sqlx::query_as::<_, AuditModRow>(&sql)
                    .bind(gid)
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await
            }
            None => {
                let sql = format!(
                    "{AUDIT_MOD_SELECT} WHERE event_type LIKE 'mod_%' ORDER BY created_at DESC LIMIT $1"
                );
                sqlx::query_as::<_, AuditModRow>(&sql)
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await
            }
        }
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(ModerationAction::from).collect())
    }

    async fn delete_bans_for_user(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<(), DomainError> {
        // Phase 4 : on supprime depuis audit_logs.
        sqlx::query(
            "DELETE FROM audit_logs WHERE guild_id = $1 AND target_id = $2 AND event_type LIKE 'mod_ban%'",
        )
        .bind(guild_id)
        .bind(target_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn delete_action(&self, id: uuid::Uuid) -> Result<bool, DomainError> {
        // Phase 4 : on supprime depuis audit_logs en matchant details->>'action_id'.
        let result = sqlx::query(
            "DELETE FROM audit_logs WHERE event_type LIKE 'mod_%' AND details->>'action_id' = $1",
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn action_guild_id(&self, action_id: Uuid) -> Result<Option<String>, DomainError> {
        // Phase 4 : `moderation_actions` n'est plus ecrite (save() est un
        // no-op). Lookup dans `audit_logs` (event_type 'mod_%') ou l'action_id
        // est stocke dans details->>'action_id' — identique a delete_action /
        // find_action_for_reversal.
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT guild_id FROM audit_logs \
             WHERE event_type LIKE 'mod_%' AND details->>'action_id' = $1 \
             LIMIT 1",
        )
        .bind(action_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("fetch action guild_id"))?;
        Ok(row.map(|(g,)| g))
    }

    async fn count_recent_mod_actions(
        &self,
        guild_id: &str,
        moderator_id: &str,
        window_secs: i64,
    ) -> Result<i64, DomainError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM moderation_actions \
             WHERE guild_id = $1 AND moderator_id = $2 \
               AND created_at > NOW() - ($3::double precision * INTERVAL '1 second')",
        )
        .bind(guild_id)
        .bind(moderator_id)
        .bind(window_secs as f64)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_ctx("count recent mod actions"))?;
        Ok(count)
    }

    async fn find_action_for_reversal(
        &self,
        action_id: Uuid,
    ) -> Result<
        Option<sentinel_core::domain::entities::moderation::action::reversal::ActionReversalInfo>,
        DomainError,
    > {
        // Phase 4 : lookup dans `audit_logs` (event_type='mod_*') ; action_id est
        // stocke dans `details->>'action_id'`. action_type derive de event_type
        // en strippant le prefixe 'mod_'.
        let row: Option<(String, Option<String>, Option<String>, String)> = sqlx::query_as(
            "SELECT guild_id, target_id, target_name, event_type \
             FROM audit_logs \
             WHERE event_type LIKE 'mod_%' AND details->>'action_id' = $1 \
             LIMIT 1",
        )
        .bind(action_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("fetch action"))?;

        Ok(
            row.map(|(guild_id, target_id_opt, target_name_opt, event_type)| {
                let action_type = event_type
                    .strip_prefix("mod_")
                    .unwrap_or(&event_type)
                    .to_string();
                sentinel_core::domain::entities::moderation::action::reversal::ActionReversalInfo {
                    guild_id,
                    target_id: target_id_opt.unwrap_or_default(),
                    target_name: target_name_opt.unwrap_or_default(),
                    action_type,
                }
            }),
        )
    }
}
