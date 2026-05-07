//! Impl Postgres de `StealProtectionRepository` (Phase 9 Part B).

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::coude::steal::protection::StealProtection;
use sentinel_core::domain::errors::DomainError;
use crate::ports::outbound::coude::steal_protection_repository::StealProtectionRepository;

use super::super::pg_err_ctx;
const TBL: &str = "steal_protection";
fn pg_err(e: sqlx::Error) -> DomainError { pg_err_ctx(TBL, e) }

pub struct PgStealProtectionRepository {
    pool: PgPool,
}

impl PgStealProtectionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ProtectionRow {
    id: Uuid,
    guild_id: String,
    user_id: String,
    item_key: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl From<ProtectionRow> for StealProtection {
    fn from(r: ProtectionRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            user_id: r.user_id.into(),
            item_key: r.item_key,
            expires_at: r.expires_at,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl StealProtectionRepository for PgStealProtectionRepository {
    async fn list_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<StealProtection>, DomainError> {
        let rows: Vec<ProtectionRow> = sqlx::query_as(
            r#"SELECT id, guild_id, user_id, item_key, expires_at, created_at
               FROM coude_steal_protections
               WHERE guild_id = $1 AND user_id = $2 AND expires_at > NOW()
               ORDER BY expires_at DESC"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn upsert(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        days_to_add: i64,
    ) -> Result<DateTime<Utc>, DomainError> {
        // Cumul : si une protection active existe deja pour ce couple,
        // on etend son expiration a partir de la valeur actuelle. Sinon
        // on insere a partir de NOW.
        //
        // GREATEST(expires_at, NOW()) gere proprement le cas ou la ligne
        // existait mais est deja expiree (on repart de NOW, pas de la
        // vieille date).
        let row: (DateTime<Utc>,) = sqlx::query_as(
            r#"INSERT INTO coude_steal_protections (guild_id, user_id, item_key, expires_at)
               VALUES ($1, $2, $3, NOW() + make_interval(days => $4::int))
               ON CONFLICT (guild_id, user_id, item_key) DO UPDATE
                   SET expires_at = GREATEST(
                           coude_steal_protections.expires_at,
                           NOW()
                       ) + make_interval(days => $4::int)
               RETURNING expires_at"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(item_key)
        .bind(days_to_add)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.0)
    }

    async fn purge_expired(&self) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM coude_steal_protections WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(result.rows_affected())
    }
}
