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
    category: String,
    details: serde_json::Value,
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
            category: row.category,
            details: row.details,
        }
    }
}

#[async_trait]
impl LogRepository for PgLogRepository {
    async fn save(&self, entry: &LogEntry) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO logs (id, timestamp, level, bot, server, message, category, details)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(entry.id)
        .bind(entry.timestamp)
        .bind(&entry.level)
        .bind(&entry.bot)
        .bind(&entry.server)
        .bind(&entry.message)
        .bind(&entry.category)
        .bind(&entry.details)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn find_all(&self, limit: i64) -> Result<Vec<LogEntry>, DomainError> {
        let rows = sqlx::query_as::<_, LogRow>(
            "SELECT id, timestamp, level, bot, server, message, category, details FROM logs ORDER BY timestamp DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(LogEntry::from).collect())
    }

    async fn delete_by_category(&self, category: &str) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM logs WHERE category = $1")
            .bind(category)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(result.rows_affected())
    }

    async fn delete_older_than_days(&self, days: i32) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM logs WHERE timestamp < NOW() - make_interval(days => $1)")
            .bind(days)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(format!("delete_logs_older: {e}")))?;
        Ok(result.rows_affected())
    }
}
