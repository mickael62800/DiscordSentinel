use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::LogEntry;
use crate::domain::errors::DomainError;
use crate::ports::outbound::LogRepository;

pub struct PgLogRepository {
    pool: PgPool,
}

impl PgLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct LogRow {
    id: Uuid,
    timestamp: chrono::DateTime<chrono::Utc>,
    level: String,
    bot: String,
    server: String,
    message: String,
}

impl From<LogRow> for LogEntry {
    fn from(row: LogRow) -> Self {
        Self {
            id: row.id,
            timestamp: row.timestamp,
            level: row.level,
            bot: row.bot,
            server: row.server,
            message: row.message,
        }
    }
}

#[async_trait]
impl LogRepository for PgLogRepository {
    async fn save(&self, entry: &LogEntry) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO logs (id, timestamp, level, bot, server, message)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(entry.id)
        .bind(entry.timestamp)
        .bind(&entry.level)
        .bind(&entry.bot)
        .bind(&entry.server)
        .bind(&entry.message)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn find_all(&self, limit: i64) -> Result<Vec<LogEntry>, DomainError> {
        let rows = sqlx::query_as::<_, LogRow>(
            "SELECT id, timestamp, level, bot, server, message FROM logs ORDER BY timestamp DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(LogEntry::from).collect())
    }
}
