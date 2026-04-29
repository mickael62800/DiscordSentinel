use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::entities::audit::user_activity::UserActivity;
use crate::domain::errors::DomainError;

use super::super::pg_err;
use crate::ports::outbound::audit::user_activity_repository::UserActivityRepository;
use crate::domain::entities::system::discord_ids::UserId;

pub struct PgUserActivityRepository {
    pool: PgPool,
}

impl PgUserActivityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}


#[derive(sqlx::FromRow)]
struct ActivityRow {
    id: uuid::Uuid,
    guild_id: String,
    user_id: UserId,
    event_type: String,
    channel_id: Option<String>,
    channel_name: Option<String>,
    content: Option<String>,
    metadata: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<ActivityRow> for UserActivity {
    fn from(r: ActivityRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            user_id: r.user_id,
            event_type: r.event_type,
            channel_id: r.channel_id,
            channel_name: r.channel_name,
            content: r.content,
            metadata: r.metadata,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl UserActivityRepository for PgUserActivityRepository {
    async fn create(&self, a: &UserActivity) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO user_activity_log
                 (id, guild_id, user_id, event_type, channel_id, channel_name, content, metadata, created_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
        )
        .bind(a.id)
        .bind(&a.guild_id)
        .bind(&a.user_id)
        .bind(&a.event_type)
        .bind(&a.channel_id)
        .bind(&a.channel_name)
        .bind(&a.content)
        .bind(&a.metadata)
        .bind(a.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list(
        &self,
        guild_id: &str,
        user_id: &str,
        event_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserActivity>, DomainError> {
        let rows = if let Some(et) = event_type {
            sqlx::query_as::<_, ActivityRow>(
                r#"SELECT id, guild_id, user_id, event_type, channel_id, channel_name, content, metadata, created_at
                   FROM user_activity_log
                   WHERE guild_id = $1 AND user_id = $2 AND event_type = $3
                   ORDER BY created_at DESC
                   LIMIT $4 OFFSET $5"#,
            )
            .bind(guild_id)
            .bind(user_id)
            .bind(et)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, ActivityRow>(
                r#"SELECT id, guild_id, user_id, event_type, channel_id, channel_name, content, metadata, created_at
                   FROM user_activity_log
                   WHERE guild_id = $1 AND user_id = $2
                   ORDER BY created_at DESC
                   LIMIT $3 OFFSET $4"#,
            )
            .bind(guild_id)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
