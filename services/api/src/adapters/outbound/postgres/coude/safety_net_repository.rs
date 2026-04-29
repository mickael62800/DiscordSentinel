//! Impl Postgres de `SafetyNetRepository` (cf. COUPE_AMELIORATIONS 4.4).

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::coude::safety_net::ActiveSafetyNet;
use crate::domain::errors::DomainError;
use crate::ports::outbound::coude::safety_net_repository::SafetyNetRepository;

use super::super::pg_err_ctx;
use crate::domain::entities::system::discord_ids::UserId;
const TBL: &str = "coude_safety_nets";
fn pg_err(e: sqlx::Error) -> DomainError { pg_err_ctx(TBL, e) }

pub struct PgSafetyNetRepository {
    pool: PgPool,
}

impl PgSafetyNetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    guild_id: String,
    user_id: UserId,
    activated_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl From<Row> for ActiveSafetyNet {
    fn from(r: Row) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            user_id: r.user_id,
            activated_at: r.activated_at,
            expires_at: r.expires_at,
        }
    }
}

#[async_trait]
impl SafetyNetRepository for PgSafetyNetRepository {
    async fn activate(
        &self,
        guild_id: &str,
        user_id: &str,
        duration_hours: i64,
    ) -> Result<Uuid, DomainError> {
        let row: (Uuid,) = sqlx::query_as(
            r#"INSERT INTO coude_safety_nets (guild_id, user_id, expires_at)
               VALUES ($1, $2, NOW() + make_interval(hours => $3::int))
               RETURNING id"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(duration_hours)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.0)
    }

    async fn get_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<ActiveSafetyNet>, DomainError> {
        let row: Option<Row> = sqlx::query_as(
            r#"SELECT id, guild_id, user_id, activated_at, expires_at
               FROM coude_safety_nets
               WHERE guild_id = $1 AND user_id = $2 AND expires_at > NOW()
               ORDER BY expires_at DESC
               LIMIT 1"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn list_active(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ActiveSafetyNet>, DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            r#"SELECT id, guild_id, user_id, activated_at, expires_at
               FROM coude_safety_nets
               WHERE guild_id = $1 AND expires_at > NOW()
               ORDER BY expires_at DESC"#,
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
