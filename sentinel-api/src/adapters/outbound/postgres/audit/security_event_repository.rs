use async_trait::async_trait;
use crate::adapters::outbound::postgres::pg_err;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::audit::security_event::SecurityEvent;
use sentinel_core::domain::errors::DomainError;
use crate::ports::outbound::audit::security_event_repository::SecurityEventRepository;

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
        let user_ids: Vec<String> = match serde_json::from_value(row.user_ids.clone()) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    event_id = %row.id,
                    guild_id = %row.guild_id,
                    error = %e,
                    raw = %row.user_ids,
                    "Parse user_ids JSON echoue dans security_event, fallback vec![]"
                );
                Vec::new()
            }
        };
        Self {
            id: row.id,
            guild_id: row.guild_id.into(),
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
    async fn save(&self, _event: &SecurityEvent) -> Result<(), DomainError> {
        // Phase 4 : on n'ecrit plus dans `security_events`. ManageSecurityService
        // ecrit directement dans audit_logs via audit_logs_uc. No-op pour ne pas
        // casser l'interface SecurityEventRepository.
        Ok(())
    }

    async fn find_all(&self) -> Result<Vec<SecurityEvent>, DomainError> {
        // Phase 2 : lecture depuis audit_logs (event_type 'security_*').
        let rows = sqlx::query_as::<_, AuditSecurityRow>(
            "SELECT id, guild_id, event_type, details, created_at FROM audit_logs \
             WHERE event_type LIKE 'security_%' ORDER BY created_at DESC LIMIT 200",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(SecurityEvent::from).collect())
    }

    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<SecurityEvent>, DomainError> {
        let rows = sqlx::query_as::<_, AuditSecurityRow>(
            "SELECT id, guild_id, event_type, details, created_at FROM audit_logs \
             WHERE guild_id = $1 AND event_type LIKE 'security_%' ORDER BY created_at DESC LIMIT 200",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(SecurityEvent::from).collect())
    }
}

/// Phase 2 helper : ligne audit_logs (event_type `security_*`) reconstruite
/// en SecurityEvent.
#[derive(sqlx::FromRow)]
struct AuditSecurityRow {
    id: Uuid,
    guild_id: String,
    event_type: String,
    details: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<AuditSecurityRow> for SecurityEvent {
    fn from(row: AuditSecurityRow) -> Self {
        let event_type = row
            .event_type
            .strip_prefix("security_")
            .unwrap_or(&row.event_type)
            .to_string();
        let severity = row
            .details
            .get("severity")
            .and_then(|v| v.as_str())
            .unwrap_or("info")
            .to_string();
        let description = row
            .details
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let user_ids: Vec<String> = row
            .details
            .get("user_ids")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        // Conserve l'event_id historique si present, sinon utilise l'audit id.
        let id = row
            .details
            .get("event_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or(row.id);
        Self {
            id,
            guild_id: row.guild_id.into(),
            event_type,
            severity,
            description,
            user_ids,
            created_at: row.created_at,
        }
    }
}
