use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::SecurityEvent;
use crate::domain::errors::DomainError;
use crate::ports::outbound::SecurityEventRepository;

pub struct PgSecurityEventRepository {
    pool: PgPool,
}

impl PgSecurityEventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: Uuid,
    guild_id: String,
    event_type: String,
    severity: String,
    description: String,
    user_ids: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<EventRow> for SecurityEvent {
    fn from(row: EventRow) -> Self {
        let user_ids: Vec<String> =
            serde_json::from_value(row.user_ids).unwrap_or_default();
        Self {
            id: row.id,
            guild_id: row.guild_id,
            event_type: row.event_type,
            severity: row.severity,
            description: row.description,
            user_ids,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl SecurityEventRepository for PgSecurityEventRepository {
    async fn save(&self, event: &SecurityEvent) -> Result<(), DomainError> {
        let user_ids_json = serde_json::to_value(&event.user_ids)
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO security_events (id, guild_id, event_type, severity, description, user_ids, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(event.id)
        .bind(&event.guild_id)
        .bind(&event.event_type)
        .bind(&event.severity)
        .bind(&event.description)
        .bind(user_ids_json)
        .bind(event.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn find_all(&self) -> Result<Vec<SecurityEvent>, DomainError> {
        let rows = sqlx::query_as::<_, EventRow>(
            "SELECT * FROM security_events ORDER BY created_at DESC LIMIT 200",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(SecurityEvent::from).collect())
    }

    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<SecurityEvent>, DomainError> {
        let rows = sqlx::query_as::<_, EventRow>(
            "SELECT * FROM security_events WHERE guild_id = $1 ORDER BY created_at DESC LIMIT 200",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(SecurityEvent::from).collect())
    }
}
