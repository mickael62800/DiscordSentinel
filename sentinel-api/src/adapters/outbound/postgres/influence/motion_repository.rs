//! Impl Postgres de `MotionRepository` et `VoteRepository`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::influence::motion::{Motion, MotionStatus};
use sentinel_core::domain::entities::influence::vote::{Tally, VoteChoice};
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::influence::motion_repository::{
    MotionRepository, VoteRepository,
};

use super::super::pg_err_ctx;

const TBL_M: &str = "influence_motions";
const TBL_V: &str = "influence_votes";

pub struct PgMotionRepository {
    pool: PgPool,
}

impl PgMotionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct MotionRow {
    id: Uuid,
    guild_id: String,
    org_id: Option<Uuid>,
    title: String,
    status: String,
    created_by: Uuid,
    created_at: DateTime<Utc>,
    closes_at: Option<DateTime<Utc>>,
}

impl TryFrom<MotionRow> for Motion {
    type Error = DomainError;
    fn try_from(r: MotionRow) -> Result<Self, DomainError> {
        let status = MotionStatus::from_str_lossy(&r.status)
            .ok_or_else(|| DomainError::Internal(format!("statut motion inconnu : {}", r.status)))?;
        let org_id = r
            .org_id
            .ok_or_else(|| DomainError::Internal("motion sans organisation".into()))?;
        Ok(Self {
            id: r.id,
            guild_id: r.guild_id,
            org_id,
            title: r.title,
            status,
            created_by: r.created_by,
            created_at: r.created_at,
            closes_at: r.closes_at,
        })
    }
}

const M_COLS: &str = "id, guild_id, org_id, title, status, created_by, created_at, closes_at";

#[async_trait]
impl MotionRepository for PgMotionRepository {
    async fn create(
        &self,
        guild_id: &str,
        org_id: Uuid,
        title: &str,
        created_by: Uuid,
    ) -> Result<Motion, DomainError> {
        let sql = format!(
            "INSERT INTO influence_motions (guild_id, org_id, title, created_by) \
             VALUES ($1, $2, $3, $4) RETURNING {M_COLS}"
        );
        let row: MotionRow = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(org_id)
            .bind(title)
            .bind(created_by)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| pg_err_ctx(TBL_M, e))?;
        row.try_into()
    }

    async fn get(&self, id: Uuid) -> Result<Option<Motion>, DomainError> {
        let sql = format!("SELECT {M_COLS} FROM influence_motions WHERE id = $1");
        let row: Option<MotionRow> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| pg_err_ctx(TBL_M, e))?;
        row.map(TryInto::try_into).transpose()
    }

    async fn set_status(&self, id: Uuid, status: MotionStatus) -> Result<(), DomainError> {
        sqlx::query("UPDATE influence_motions SET status = $2 WHERE id = $1")
            .bind(id)
            .bind(status.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| pg_err_ctx(TBL_M, e))?;
        Ok(())
    }
}

pub struct PgVoteRepository {
    pool: PgPool,
}

impl PgVoteRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl VoteRepository for PgVoteRepository {
    async fn upsert(
        &self,
        subject_id: Uuid,
        voter_id: Uuid,
        choice: VoteChoice,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO influence_votes (subject_type, subject_id, voter_id, choice) \
             VALUES ('motion', $1, $2, $3) \
             ON CONFLICT (subject_id, voter_id) DO UPDATE SET choice = EXCLUDED.choice",
        )
        .bind(subject_id)
        .bind(voter_id)
        .bind(choice.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| pg_err_ctx(TBL_V, e))?;
        Ok(())
    }

    async fn tally(&self, subject_id: Uuid) -> Result<Tally, DomainError> {
        // Compte brut + poids du vote = influence (plancher 1) + NOTORIETE du
        // votant (une figure connue pese davantage). LEFT JOIN sur les
        // dimensions de reputation (0 si aucune ligne).
        let rows: Vec<(String, i64, i64)> = sqlx::query_as(
            "SELECT v.choice, COUNT(*), \
                    COALESCE(SUM(GREATEST(c.influence, 1) + GREATEST(COALESCE(rd.notoriety, 0), 0)), 0) \
             FROM influence_votes v \
             JOIN influence_citizens c ON c.id = v.voter_id \
             LEFT JOIN influence_reputation_dims rd ON rd.citizen_id = c.id \
             WHERE v.subject_id = $1 GROUP BY v.choice",
        )
        .bind(subject_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| pg_err_ctx(TBL_V, e))?;

        let mut tally = Tally::default();
        for (choice, count, weight) in rows {
            match choice.as_str() {
                "pour" => {
                    tally.pour = count;
                    tally.pour_weight = weight;
                }
                "contre" => {
                    tally.contre = count;
                    tally.contre_weight = weight;
                }
                "abstention" => tally.abstention = count,
                _ => {}
            }
        }
        Ok(tally)
    }
}
