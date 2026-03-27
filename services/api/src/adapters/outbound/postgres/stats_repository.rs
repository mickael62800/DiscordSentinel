use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::UserStats;
use crate::domain::errors::DomainError;
use crate::ports::outbound::StatsRepository;

pub struct PgStatsRepository {
    pool: PgPool,
}

impl PgStatsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct StatsRow {
    id: Uuid,
    guild_id: String,
    user_id: String,
    username: String,
    message_count: i64,
    voice_seconds: i64,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<StatsRow> for UserStats {
    fn from(row: StatsRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id,
            user_id: row.user_id,
            username: row.username,
            message_count: row.message_count as u64,
            voice_seconds: row.voice_seconds as u64,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl StatsRepository for PgStatsRepository {
    async fn upsert(&self, stats: &UserStats) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO user_stats (id, guild_id, user_id, username, message_count, voice_seconds, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (guild_id, user_id) DO UPDATE SET
                username = EXCLUDED.username,
                message_count = EXCLUDED.message_count,
                voice_seconds = EXCLUDED.voice_seconds,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(stats.id)
        .bind(&stats.guild_id)
        .bind(&stats.user_id)
        .bind(&stats.username)
        .bind(stats.message_count as i64)
        .bind(stats.voice_seconds as i64)
        .bind(stats.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn find_by_user(&self, guild_id: &str, user_id: &str) -> Result<Option<UserStats>, DomainError> {
        let row = sqlx::query_as::<_, StatsRow>(
            "SELECT * FROM user_stats WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(UserStats::from))
    }

    async fn find_by_guild(&self, guild_id: &str, limit: u32) -> Result<Vec<UserStats>, DomainError> {
        let rows = sqlx::query_as::<_, StatsRow>(
            "SELECT * FROM user_stats WHERE guild_id = $1 ORDER BY message_count DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(UserStats::from).collect())
    }

    async fn increment_messages(&self, guild_id: &str, user_id: &str, username: &str, count: u64) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO user_stats (id, guild_id, user_id, username, message_count, voice_seconds, updated_at)
            VALUES ($1, $2, $3, $4, $5, 0, NOW())
            ON CONFLICT (guild_id, user_id) DO UPDATE SET
                username = EXCLUDED.username,
                message_count = user_stats.message_count + EXCLUDED.message_count,
                updated_at = NOW()
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(guild_id)
        .bind(user_id)
        .bind(username)
        .bind(count as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn add_voice_seconds(&self, guild_id: &str, user_id: &str, username: &str, seconds: u64) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO user_stats (id, guild_id, user_id, username, message_count, voice_seconds, updated_at)
            VALUES ($1, $2, $3, $4, 0, $5, NOW())
            ON CONFLICT (guild_id, user_id) DO UPDATE SET
                username = EXCLUDED.username,
                voice_seconds = user_stats.voice_seconds + EXCLUDED.voice_seconds,
                updated_at = NOW()
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(guild_id)
        .bind(user_id)
        .bind(username)
        .bind(seconds as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn count_distinct_guilds(&self) -> Result<u64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT guild_id) FROM user_stats",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.0 as u64)
    }

    async fn count_distinct_users(&self) -> Result<u64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT user_id) FROM user_stats",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.0 as u64)
    }
}
