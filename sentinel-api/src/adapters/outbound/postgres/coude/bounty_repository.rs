//! Impl Postgres de `BountyRepository` (cf. COUPE_AMELIORATIONS 5.3).

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::coude::bounty::ActiveBounty;
use sentinel_core::domain::entities::coude::bounty::BountyStatus;
use sentinel_core::domain::errors::DomainError;
use crate::ports::outbound::coude::bounty_repository::BountyRepository;

use super::super::pg_err_ctx;
const TBL: &str = "coude_bounties";
fn pg_err(e: sqlx::Error) -> DomainError { pg_err_ctx(TBL, e) }

pub struct PgBountyRepository {
    pool: PgPool,
}

impl PgBountyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    guild_id: String,
    target_id: String,
    total_amount: i64,
    status: String,
    opened_at: DateTime<Utc>,
    claimed_by: Option<String>,
    claimed_at: Option<DateTime<Utc>>,
}

impl TryFrom<Row> for ActiveBounty {
    type Error = DomainError;
    fn try_from(r: Row) -> Result<Self, DomainError> {
        let status = BountyStatus::from_db_str(&r.status).ok_or_else(|| {
            DomainError::Internal(format!("bounty status inconnu : {}", r.status))
        })?;
        Ok(Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            target_id: r.target_id.into(),
            total_amount: r.total_amount,
            status,
            opened_at: r.opened_at,
            claimed_by: r.claimed_by,
            claimed_at: r.claimed_at,
        })
    }
}

#[async_trait]
impl BountyRepository for PgBountyRepository {
    async fn open(
        &self,
        guild_id: &str,
        target_id: &str,
        initial_amount: i64,
    ) -> Result<Uuid, DomainError> {
        let row: (Uuid,) = sqlx::query_as(
            r#"INSERT INTO coude_bounties (guild_id, target_id, total_amount)
               VALUES ($1, $2, $3)
               RETURNING id"#,
        )
        .bind(guild_id)
        .bind(target_id)
        .bind(initial_amount)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                DomainError::Conflict("Une prime est deja ouverte sur cette cible.".into())
            }
            _ => pg_err(e),
        })?;
        Ok(row.0)
    }

    async fn get_open(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveBounty>, DomainError> {
        let row: Option<Row> = sqlx::query_as(
            r#"SELECT id, guild_id, target_id, total_amount, status,
                      opened_at, claimed_by, claimed_at
               FROM coude_bounties
               WHERE guild_id = $1 AND target_id = $2 AND status = 'open'
               LIMIT 1"#,
        )
        .bind(guild_id)
        .bind(target_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        row.map(ActiveBounty::try_from).transpose()
    }

    async fn contribute(
        &self,
        bounty_id: Uuid,
        contributor_id: &str,
        contributor_name: &str,
        amount: i64,
    ) -> Result<i64, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let row: Option<(i64,)> = sqlx::query_as(
            r#"UPDATE coude_bounties
               SET total_amount = total_amount + $2
               WHERE id = $1 AND status = 'open'
               RETURNING total_amount"#,
        )
        .bind(bounty_id)
        .bind(amount)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?;
        let new_total = row.ok_or_else(|| {
            DomainError::Conflict("Prime introuvable ou deja fermee.".into())
        })?.0;

        sqlx::query(
            r#"INSERT INTO coude_bounty_contributions
                   (bounty_id, contributor_id, contributor_name, amount)
               VALUES ($1, $2, $3, $4)"#,
        )
        .bind(bounty_id)
        .bind(contributor_id)
        .bind(contributor_name)
        .bind(amount)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;
        Ok(new_total)
    }

    async fn claim(
        &self,
        bounty_id: Uuid,
        claimer_id: &str,
    ) -> Result<i64, DomainError> {
        let row: Option<(i64,)> = sqlx::query_as(
            r#"UPDATE coude_bounties
               SET status = 'claimed', claimed_by = $2, claimed_at = NOW()
               WHERE id = $1 AND status = 'open'
               RETURNING total_amount"#,
        )
        .bind(bounty_id)
        .bind(claimer_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        row.map(|r| r.0).ok_or_else(|| {
            DomainError::Conflict("Prime introuvable ou deja claimed.".into())
        })
    }
}
