//! Impl Postgres de `OrganizationRepository`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::influence::organization::Organization;
use sentinel_core::domain::entities::influence::treasury::{TreasuryKind, TreasuryMovement};
use sentinel_core::domain::enums::influence::organization_kind::OrganizationKind;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::influence::organization_repository::{
    NewOrganization, OrganizationRepository,
};

use super::super::pg_err_ctx;

const TBL: &str = "influence_organizations";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

pub struct PgOrganizationRepository {
    pool: PgPool,
}

impl PgOrganizationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    guild_id: String,
    kind: String,
    name: String,
    motto: String,
    treasury: i64,
    reputation: i64,
    influence: i64,
    founder_id: Uuid,
    discord_role_id: Option<String>,
    discord_channel_id: Option<String>,
    created_at: DateTime<Utc>,
    dissolved_at: Option<DateTime<Utc>>,
}

impl TryFrom<Row> for Organization {
    type Error = DomainError;
    fn try_from(r: Row) -> Result<Self, DomainError> {
        let kind = OrganizationKind::from_str_lossy(&r.kind)
            .ok_or_else(|| DomainError::Internal(format!("kind org inconnu : {}", r.kind)))?;
        Ok(Self {
            id: r.id,
            guild_id: r.guild_id,
            kind,
            name: r.name,
            motto: r.motto,
            treasury: r.treasury,
            reputation: r.reputation,
            influence: r.influence,
            founder_id: r.founder_id,
            discord_role_id: r.discord_role_id,
            discord_channel_id: r.discord_channel_id,
            created_at: r.created_at,
            dissolved_at: r.dissolved_at,
        })
    }
}

const SELECT_COLS: &str = "id, guild_id, kind, name, motto, treasury, reputation, \
    influence, founder_id, discord_role_id, discord_channel_id, created_at, dissolved_at";

#[async_trait]
impl OrganizationRepository for PgOrganizationRepository {
    async fn create(&self, new: NewOrganization<'_>) -> Result<Organization, DomainError> {
        let sql = format!(
            "INSERT INTO influence_organizations (guild_id, kind, name, motto, founder_id) \
             VALUES ($1, $2, $3, $4, $5) RETURNING {SELECT_COLS}"
        );
        let row: Row = sqlx::query_as(&sql)
            .bind(new.guild_id)
            .bind(new.kind.as_str())
            .bind(new.name)
            .bind(new.motto)
            .bind(new.founder_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match &e {
                sqlx::Error::Database(db) if db.is_unique_violation() => {
                    DomainError::Conflict(format!("Nom d'organisation deja pris : {}", new.name))
                }
                _ => pg_err(e),
            })?;
        row.try_into()
    }

    async fn get(&self, id: Uuid) -> Result<Option<Organization>, DomainError> {
        let sql = format!("SELECT {SELECT_COLS} FROM influence_organizations WHERE id = $1");
        let row: Option<Row> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_name(
        &self,
        guild_id: &str,
        name: &str,
    ) -> Result<Option<Organization>, DomainError> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM influence_organizations \
             WHERE guild_id = $1 AND LOWER(name) = LOWER($2) AND dissolved_at IS NULL"
        );
        let row: Option<Row> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn count_active_founded_by(&self, founder_id: Uuid) -> Result<i64, DomainError> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM influence_organizations \
             WHERE founder_id = $1 AND dissolved_at IS NULL",
        )
        .bind(founder_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)
    }

    async fn list_for_guild(&self, guild_id: &str) -> Result<Vec<Organization>, DomainError> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM influence_organizations \
             WHERE guild_id = $1 AND dissolved_at IS NULL ORDER BY created_at"
        );
        let rows: Vec<Row> = sqlx::query_as(&sql)
            .bind(guild_id)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn dissolve(&self, org_id: Uuid) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE influence_organizations SET dissolved_at = NOW() \
             WHERE id = $1 AND dissolved_at IS NULL",
        )
        .bind(org_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn set_discord_role(&self, org_id: Uuid, role_id: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE influence_organizations SET discord_role_id = $2 WHERE id = $1")
            .bind(org_id)
            .bind(role_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn set_discord_channel(
        &self,
        org_id: Uuid,
        channel_id: &str,
    ) -> Result<(), DomainError> {
        sqlx::query("UPDATE influence_organizations SET discord_channel_id = $2 WHERE id = $1")
            .bind(org_id)
            .bind(channel_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn founder_user_id(&self, org_id: Uuid) -> Result<Option<String>, DomainError> {
        let uid: Option<String> = sqlx::query_scalar(
            "SELECT c.user_id FROM influence_organizations o \
             JOIN influence_citizens c ON c.id = o.founder_id WHERE o.id = $1",
        )
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(uid)
    }

    async fn collective_power(&self, org_id: Uuid) -> Result<(i64, i64), DomainError> {
        let row: (i64, i64) = sqlx::query_as(
            "SELECT COALESCE(SUM(c.influence), 0), COALESCE(SUM(c.reputation), 0) \
             FROM influence_org_members m JOIN influence_citizens c ON c.id = m.citizen_id \
             WHERE m.org_id = $1",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row)
    }

    async fn deposit_treasury(
        &self,
        org_id: Uuid,
        guild_id: &str,
        amount: i64,
        actor_user_id: &str,
        actor_username: &str,
    ) -> Result<i64, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let new_bal: i64 = sqlx::query_scalar(
            "UPDATE influence_organizations SET treasury = treasury + $2 WHERE id = $1 RETURNING treasury",
        )
        .bind(org_id)
        .bind(amount)
        .fetch_one(&mut *tx)
        .await
        .map_err(pg_err)?;
        sqlx::query(
            "INSERT INTO influence_org_treasury_movements \
             (guild_id, org_id, kind, amount, treasury_after, actor_user_id, actor_username) \
             VALUES ($1, $2, 'deposit', $3, $4, $5, $6)",
        )
        .bind(guild_id)
        .bind(org_id)
        .bind(amount)
        .bind(new_bal)
        .bind(actor_user_id)
        .bind(actor_username)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)?;
        Ok(new_bal)
    }

    async fn withdraw_treasury(
        &self,
        org_id: Uuid,
        guild_id: &str,
        amount: i64,
        actor_user_id: &str,
        actor_username: &str,
    ) -> Result<Option<i64>, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        // Garde `treasury >= amount` : jamais de solde negatif.
        let new_bal: Option<i64> = sqlx::query_scalar(
            "UPDATE influence_organizations SET treasury = treasury - $2 \
             WHERE id = $1 AND treasury >= $2 RETURNING treasury",
        )
        .bind(org_id)
        .bind(amount)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?;
        let Some(bal) = new_bal else {
            tx.rollback().await.map_err(pg_err)?;
            return Ok(None);
        };
        sqlx::query(
            "INSERT INTO influence_org_treasury_movements \
             (guild_id, org_id, kind, amount, treasury_after, actor_user_id, actor_username) \
             VALUES ($1, $2, 'withdrawal', $3, $4, $5, $6)",
        )
        .bind(guild_id)
        .bind(org_id)
        .bind(amount)
        .bind(bal)
        .bind(actor_user_id)
        .bind(actor_username)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;
        tx.commit().await.map_err(pg_err)?;
        Ok(Some(bal))
    }

    async fn list_treasury_movements(
        &self,
        org_id: Uuid,
        limit: i64,
    ) -> Result<Vec<TreasuryMovement>, DomainError> {
        let rows: Vec<(String, i64, i64, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT kind, amount, treasury_after, actor_username, created_at \
             FROM influence_org_treasury_movements \
             WHERE org_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(org_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(|(kind, amount, after, actor, created_at)| TreasuryMovement {
                kind: if kind == "withdrawal" {
                    TreasuryKind::Withdrawal
                } else {
                    TreasuryKind::Deposit
                },
                amount,
                treasury_after: after,
                actor_username: actor,
                created_at,
            })
            .collect())
    }
}
