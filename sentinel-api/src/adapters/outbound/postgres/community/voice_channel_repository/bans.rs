use crate::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use uuid::Uuid;

use crate::ports::outbound::community::voice_channel_repository::VoiceBanStore;
use sentinel_core::domain::entities::community::voice_channel::VoiceChannelBan;
use sentinel_core::domain::errors::DomainError;

#[derive(sqlx::FromRow)]
struct BanRow {
    id: Uuid,
    voice_channel_id: Uuid,
    user_id: String,
    user_name: String,
    banned_by: String,
    reason: Option<String>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<BanRow> for VoiceChannelBan {
    fn from(row: BanRow) -> Self {
        Self {
            id: row.id,
            voice_channel_id: row.voice_channel_id,
            user_id: row.user_id.into(),
            user_name: row.user_name,
            banned_by: row.banned_by,
            reason: row.reason,
            expires_at: row.expires_at,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl VoiceBanStore for super::PgVoiceChannelRepository {
    async fn find_bans(&self, voice_channel_id: Uuid) -> Result<Vec<VoiceChannelBan>, DomainError> {
        let rows = sqlx::query_as::<_, BanRow>(
            "SELECT * FROM voice_channel_bans WHERE voice_channel_id = $1 ORDER BY created_at DESC",
        )
        .bind(voice_channel_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(VoiceChannelBan::from).collect())
    }

    async fn find_active_ban(
        &self,
        voice_channel_id: Uuid,
        user_id: &str,
    ) -> Result<Option<VoiceChannelBan>, DomainError> {
        let row = sqlx::query_as::<_, BanRow>(
            "SELECT * FROM voice_channel_bans WHERE voice_channel_id = $1 AND user_id = $2 AND (expires_at IS NULL OR expires_at > NOW())",
        )
        .bind(voice_channel_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(row.map(VoiceChannelBan::from))
    }

    async fn save_ban(&self, ban: &VoiceChannelBan) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO voice_channel_bans (id, voice_channel_id, user_id, user_name, banned_by, reason, expires_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (voice_channel_id, user_id) DO UPDATE SET
                banned_by = EXCLUDED.banned_by,
                reason = EXCLUDED.reason,
                expires_at = EXCLUDED.expires_at,
                created_at = EXCLUDED.created_at
            "#,
        )
        .bind(ban.id)
        .bind(ban.voice_channel_id)
        .bind(ban.user_id.as_str())
        .bind(&ban.user_name)
        .bind(&ban.banned_by)
        .bind(&ban.reason)
        .bind(ban.expires_at)
        .bind(ban.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn remove_ban(&self, voice_channel_id: Uuid, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM voice_channel_bans WHERE voice_channel_id = $1 AND user_id = $2")
            .bind(voice_channel_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(())
    }

    async fn cleanup_expired_bans(&self) -> Result<u64, DomainError> {
        let result = sqlx::query(
            "DELETE FROM voice_channel_bans WHERE expires_at IS NOT NULL AND expires_at <= NOW()",
        )
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(result.rows_affected())
    }
}
