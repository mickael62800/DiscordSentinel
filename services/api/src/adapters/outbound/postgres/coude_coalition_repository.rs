//! Impl Postgres de `CoudeCoalitionRepository`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::{
    ActiveCoalition, CoalitionMember, CoalitionStatus, COALITION_MIN_MEMBERS,
};
use crate::domain::errors::DomainError;
use crate::ports::outbound::CoudeCoalitionRepository;

use super::pg_err_ctx;
const TBL: &str = "coude_coalitions";
fn pg_err(e: sqlx::Error) -> DomainError { pg_err_ctx(TBL, e) }

pub struct PgCoudeCoalitionRepository {
    pool: PgPool,
}

impl PgCoudeCoalitionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct CoalitionRow {
    id: Uuid,
    guild_id: String,
    target_id: String,
    opened_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    status: String,
    broken_by: Option<String>,
    broken_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct MemberRow {
    member_id: String,
    member_name: String,
    joined_at: DateTime<Utc>,
}

async fn load_coalition(
    pool: &PgPool,
    row: CoalitionRow,
) -> Result<ActiveCoalition, DomainError> {
    let status = CoalitionStatus::from_db_str(&row.status).ok_or_else(|| {
        DomainError::Internal(format!("coalition status inconnu : {}", row.status))
    })?;
    let members: Vec<MemberRow> = sqlx::query_as(
        r#"SELECT member_id, member_name, joined_at
           FROM coude_coalition_members
           WHERE coalition_id = $1
           ORDER BY joined_at"#,
    )
    .bind(row.id)
    .fetch_all(pool)
    .await
    .map_err(pg_err)?;
    Ok(ActiveCoalition {
        id: row.id,
        guild_id: row.guild_id,
        target_id: row.target_id,
        opened_at: row.opened_at,
        expires_at: row.expires_at,
        status,
        broken_by: row.broken_by,
        broken_at: row.broken_at,
        members: members
            .into_iter()
            .map(|m| CoalitionMember {
                member_id: m.member_id,
                member_name: m.member_name,
                joined_at: m.joined_at,
            })
            .collect(),
    })
}

#[async_trait]
impl CoudeCoalitionRepository for PgCoudeCoalitionRepository {
    async fn create_with_first_member(
        &self,
        guild_id: &str,
        target_id: &str,
        first_member_id: &str,
        first_member_name: &str,
        duration_hours: i64,
    ) -> Result<Uuid, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let row: (Uuid,) = sqlx::query_as(
            r#"INSERT INTO coude_coalitions (guild_id, target_id, expires_at)
               VALUES ($1, $2, NOW() + make_interval(hours => $3::int))
               RETURNING id"#,
        )
        .bind(guild_id)
        .bind(target_id)
        .bind(duration_hours)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                DomainError::Conflict("Une coalition est deja active sur cette cible.".into())
            }
            _ => pg_err(e),
        })?;
        sqlx::query(
            r#"INSERT INTO coude_coalition_members (coalition_id, member_id, member_name)
               VALUES ($1, $2, $3)"#,
        )
        .bind(row.0)
        .bind(first_member_id)
        .bind(first_member_name)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)?;
        Ok(row.0)
    }

    async fn add_member(
        &self,
        coalition_id: Uuid,
        member_id: &str,
        member_name: &str,
    ) -> Result<ActiveCoalition, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        // Insert ignore-on-conflict pour idempotence.
        sqlx::query(
            r#"INSERT INTO coude_coalition_members (coalition_id, member_id, member_name)
               VALUES ($1, $2, $3)
               ON CONFLICT (coalition_id, member_id) DO NOTHING"#,
        )
        .bind(coalition_id)
        .bind(member_id)
        .bind(member_name)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        // Compte des membres actuels pour decider de la transition status.
        let cnt: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*)::BIGINT FROM coude_coalition_members WHERE coalition_id = $1"#,
        )
        .bind(coalition_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(pg_err)?;
        if cnt.0 as i32 >= COALITION_MIN_MEMBERS {
            sqlx::query(
                r#"UPDATE coude_coalitions
                   SET status = 'active'
                   WHERE id = $1 AND status = 'forming'"#,
            )
            .bind(coalition_id)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;
        }

        let row: CoalitionRow = sqlx::query_as(
            r#"SELECT id, guild_id, target_id, opened_at, expires_at, status,
                      broken_by, broken_at
               FROM coude_coalitions
               WHERE id = $1"#,
        )
        .bind(coalition_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)?;
        load_coalition(&self.pool, row).await
    }

    async fn get_active(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveCoalition>, DomainError> {
        let row: Option<CoalitionRow> = sqlx::query_as(
            r#"SELECT id, guild_id, target_id, opened_at, expires_at, status,
                      broken_by, broken_at
               FROM coude_coalitions
               WHERE guild_id = $1 AND target_id = $2
                 AND status IN ('forming', 'active')
                 AND expires_at > NOW()
               LIMIT 1"#,
        )
        .bind(guild_id)
        .bind(target_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        match row {
            Some(r) => Ok(Some(load_coalition(&self.pool, r).await?)),
            None => Ok(None),
        }
    }

    async fn mark_broken(
        &self,
        coalition_id: Uuid,
        breaker_id: &str,
    ) -> Result<(), DomainError> {
        let result = sqlx::query(
            r#"UPDATE coude_coalitions
               SET status = 'broken', broken_by = $2, broken_at = NOW()
               WHERE id = $1 AND status IN ('forming', 'active')"#,
        )
        .bind(coalition_id)
        .bind(breaker_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        if result.rows_affected() == 0 {
            return Err(DomainError::Conflict(
                "Coalition introuvable ou deja fermee.".into(),
            ));
        }
        Ok(())
    }

    async fn is_member_of_active_coalition_against(
        &self,
        guild_id: &str,
        member_id: &str,
        target_id: &str,
    ) -> Result<bool, DomainError> {
        let row: Option<(i32,)> = sqlx::query_as(
            r#"SELECT 1
               FROM coude_coalitions c
               JOIN coude_coalition_members m ON m.coalition_id = c.id
               WHERE c.guild_id = $1
                 AND c.target_id = $2
                 AND c.status IN ('forming', 'active')
                 AND c.expires_at > NOW()
                 AND m.member_id = $3
               LIMIT 1"#,
        )
        .bind(guild_id)
        .bind(target_id)
        .bind(member_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.is_some())
    }
}
