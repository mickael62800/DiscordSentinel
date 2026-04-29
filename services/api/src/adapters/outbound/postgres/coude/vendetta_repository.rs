//! Impl Postgres de `VendettaRepository` (cf. COUPE_AMELIORATIONS 5.3).

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::coude::vendetta::ActiveVendetta;
use crate::domain::entities::coude::vendetta::VendettaStatus;
use crate::domain::errors::DomainError;
use crate::ports::outbound::coude::vendetta_repository::VendettaRepository;

use super::super::pg_err_ctx;
use crate::domain::entities::system::discord_ids::GuildId;
const TBL: &str = "coude_vendettas";
fn pg_err(e: sqlx::Error) -> DomainError { pg_err_ctx(TBL, e) }

pub struct PgVendettaRepository {
    pool: PgPool,
}

impl PgVendettaRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    guild_id: GuildId,
    challenger_id: String,
    target_id: String,
    declared_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    status: String,
    resolved_at: Option<DateTime<Utc>>,
}

impl TryFrom<Row> for ActiveVendetta {
    type Error = DomainError;
    fn try_from(r: Row) -> Result<Self, DomainError> {
        let status = VendettaStatus::from_db_str(&r.status).ok_or_else(|| {
            DomainError::Internal(format!("vendetta status inconnu : {}", r.status))
        })?;
        Ok(Self {
            id: r.id,
            guild_id: r.guild_id,
            challenger_id: r.challenger_id,
            target_id: r.target_id,
            declared_at: r.declared_at,
            expires_at: r.expires_at,
            status,
            resolved_at: r.resolved_at,
        })
    }
}

#[async_trait]
impl VendettaRepository for PgVendettaRepository {
    async fn declare(
        &self,
        guild_id: &str,
        challenger_id: &str,
        target_id: &str,
        duration_hours: i64,
    ) -> Result<Uuid, DomainError> {
        let row: (Uuid,) = sqlx::query_as(
            r#"INSERT INTO coude_vendettas
                   (guild_id, challenger_id, target_id, expires_at)
               VALUES ($1, $2, $3, NOW() + make_interval(hours => $4::int))
               RETURNING id"#,
        )
        .bind(guild_id)
        .bind(challenger_id)
        .bind(target_id)
        .bind(duration_hours)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                DomainError::Conflict("Une vendetta est deja active sur ce couple.".into())
            }
            _ => pg_err(e),
        })?;
        Ok(row.0)
    }

    async fn get_active(
        &self,
        guild_id: &str,
        challenger_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveVendetta>, DomainError> {
        let row: Option<Row> = sqlx::query_as(
            r#"SELECT id, guild_id, challenger_id, target_id, declared_at,
                      expires_at, status, resolved_at
               FROM coude_vendettas
               WHERE guild_id = $1
                 AND challenger_id = $2
                 AND target_id = $3
                 AND status = 'active'
                 AND expires_at > NOW()
               LIMIT 1"#,
        )
        .bind(guild_id)
        .bind(challenger_id)
        .bind(target_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        row.map(ActiveVendetta::try_from).transpose()
    }

    async fn resolve(&self, id: Uuid, won: bool) -> Result<(), DomainError> {
        let new_status = if won { "won" } else { "lost" };
        let result = sqlx::query(
            r#"UPDATE coude_vendettas
               SET status = $2, resolved_at = NOW()
               WHERE id = $1 AND status = 'active'"#,
        )
        .bind(id)
        .bind(new_status)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        if result.rows_affected() == 0 {
            return Err(DomainError::Conflict(
                "Vendetta deja resolue ou inconnue.".into(),
            ));
        }
        Ok(())
    }

    async fn list_by_challenger(
        &self,
        guild_id: &str,
        challenger_id: &str,
    ) -> Result<Vec<ActiveVendetta>, DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            r#"SELECT id, guild_id, challenger_id, target_id, declared_at,
                      expires_at, status, resolved_at
               FROM coude_vendettas
               WHERE guild_id = $1 AND challenger_id = $2
               ORDER BY declared_at DESC
               LIMIT 50"#,
        )
        .bind(guild_id)
        .bind(challenger_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        rows.into_iter()
            .map(ActiveVendetta::try_from)
            .collect::<Result<Vec<_>, _>>()
    }
}
