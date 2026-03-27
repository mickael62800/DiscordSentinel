use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::Infraction;
use crate::domain::errors::DomainError;
use crate::domain::value_objects::{Action, DetectionFlags};
use crate::ports::inbound::InfractionFilters;
use crate::ports::outbound::InfractionRepository;

pub struct PgInfractionRepository {
    pool: PgPool,
}

impl PgInfractionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct InfractionRow {
    id: Uuid,
    guild_id: String,
    channel_id: String,
    user_id: String,
    username: String,
    message_id: String,
    content: String,
    flags: serde_json::Value,
    score: f64,
    action: String,
    reason: String,
    duration: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<InfractionRow> for Infraction {
    fn from(row: InfractionRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id,
            channel_id: row.channel_id,
            user_id: row.user_id,
            username: row.username,
            message_id: row.message_id,
            content: row.content,
            flags: serde_json::from_value(row.flags).unwrap_or(DetectionFlags {
                spam: false,
                insult: false,
                link: false,
                phishing: false,
            }),
            score: row.score,
            action: Action::from_str_lossy(&row.action),
            reason: row.reason,
            duration: row.duration.map(|d| d as u64),
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl InfractionRepository for PgInfractionRepository {
    async fn save(&self, infraction: &Infraction) -> Result<(), DomainError> {
        let flags_json =
            serde_json::to_value(&infraction.flags).map_err(|e| DomainError::Internal(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO infractions (id, guild_id, channel_id, user_id, username, message_id, content, flags, score, action, reason, duration, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
        )
        .bind(infraction.id)
        .bind(&infraction.guild_id)
        .bind(&infraction.channel_id)
        .bind(&infraction.user_id)
        .bind(&infraction.username)
        .bind(&infraction.message_id)
        .bind(&infraction.content)
        .bind(flags_json)
        .bind(infraction.score)
        .bind(infraction.action.as_str())
        .bind(&infraction.reason)
        .bind(infraction.duration.map(|d| d as i64))
        .bind(infraction.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn find_by_guild(
        &self,
        guild_id: &str,
        filters: &InfractionFilters,
    ) -> Result<Vec<Infraction>, DomainError> {
        let mut query = String::from("SELECT * FROM infractions WHERE guild_id = $1");
        let mut param_idx = 2u32;

        if filters.user_id.is_some() {
            query.push_str(&format!(" AND user_id = ${}", param_idx));
            param_idx += 1;
        }
        if filters.action.is_some() {
            query.push_str(&format!(" AND action = ${}", param_idx));
            param_idx += 1;
        }

        query.push_str(&format!(" ORDER BY created_at DESC LIMIT ${} OFFSET ${}", param_idx, param_idx + 1));

        let mut q = sqlx::query_as::<_, InfractionRow>(&query).bind(guild_id);

        if let Some(ref user_id) = filters.user_id {
            q = q.bind(user_id);
        }
        if let Some(ref action) = filters.action {
            q = q.bind(action);
        }

        q = q.bind(filters.limit).bind(filters.offset);

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(Infraction::from).collect())
    }

    async fn find_all(&self, limit: i64, offset: i64) -> Result<Vec<Infraction>, DomainError> {
        let rows = sqlx::query_as::<_, InfractionRow>(
            "SELECT * FROM infractions ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(Infraction::from).collect())
    }

    async fn count_today(&self) -> Result<u64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM infractions WHERE created_at >= CURRENT_DATE",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.0 as u64)
    }
}
