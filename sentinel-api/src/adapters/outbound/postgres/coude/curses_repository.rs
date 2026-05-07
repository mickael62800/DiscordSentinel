//! Impl Postgres de `CursesRepository` (cf. COUPE_AMELIORATIONS 5.1).

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::coude::curse::ActiveCurse;
use sentinel_core::domain::entities::coude::curse::CurseKind;
use sentinel_core::domain::errors::DomainError;
use crate::ports::outbound::coude::curses_repository::CursesRepository;

use super::super::pg_err_ctx;
const TBL: &str = "coude_curses";
fn pg_err(e: sqlx::Error) -> DomainError { pg_err_ctx(TBL, e) }

pub struct PgCursesRepository {
    pool: PgPool,
}

impl PgCursesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct CurseRow {
    id: Uuid,
    guild_id: String,
    target_id: String,
    source_id: String,
    kind: String,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    lifted_at: Option<DateTime<Utc>>,
    lifted_by: Option<String>,
    uses_remaining: Option<i32>,
}

impl TryFrom<CurseRow> for ActiveCurse {
    type Error = DomainError;
    fn try_from(r: CurseRow) -> Result<Self, DomainError> {
        let kind = CurseKind::from_db_str(&r.kind).ok_or_else(|| {
            DomainError::Internal(format!("kind de curse inconnu en DB : {}", r.kind))
        })?;
        Ok(Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            target_id: r.target_id.into(),
            source_id: r.source_id,
            kind,
            created_at: r.created_at,
            expires_at: r.expires_at,
            lifted_at: r.lifted_at,
            lifted_by: r.lifted_by,
            uses_remaining: r.uses_remaining,
        })
    }
}

#[async_trait]
impl CursesRepository for PgCursesRepository {
    async fn cast(
        &self,
        guild_id: &str,
        target_id: &str,
        source_id: &str,
        kind: CurseKind,
        duration_hours: i64,
    ) -> Result<Uuid, DomainError> {
        let row: (Uuid,) = sqlx::query_as(
            r#"INSERT INTO coude_curses (guild_id, target_id, source_id, kind, expires_at)
               VALUES ($1, $2, $3, $4, NOW() + make_interval(hours => $5::int))
               RETURNING id"#,
        )
        .bind(guild_id)
        .bind(target_id)
        .bind(source_id)
        .bind(kind.as_db_str())
        .bind(duration_hours)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                DomainError::Conflict("Une malediction est deja active sur cette cible.".into())
            }
            _ => pg_err(e),
        })?;
        Ok(row.0)
    }

    async fn get_active_for_target(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveCurse>, DomainError> {
        let row: Option<CurseRow> = sqlx::query_as(
            r#"SELECT id, guild_id, target_id, source_id, kind,
                      created_at, expires_at, lifted_at, lifted_by, uses_remaining
               FROM coude_curses
               WHERE guild_id = $1 AND target_id = $2
                 AND lifted_at IS NULL
                 AND expires_at > NOW()
               LIMIT 1"#,
        )
        .bind(guild_id)
        .bind(target_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        row.map(ActiveCurse::try_from).transpose()
    }

    async fn cast_with_uses(
        &self,
        guild_id: &str,
        target_id: &str,
        source_id: &str,
        kind: CurseKind,
        duration_hours: i64,
        uses_remaining: Option<i32>,
    ) -> Result<Uuid, DomainError> {
        let row: (Uuid,) = sqlx::query_as(
            r#"INSERT INTO coude_curses
                   (guild_id, target_id, source_id, kind, expires_at, uses_remaining)
               VALUES ($1, $2, $3, $4, NOW() + make_interval(hours => $5::int), $6)
               RETURNING id"#,
        )
        .bind(guild_id)
        .bind(target_id)
        .bind(source_id)
        .bind(kind.as_db_str())
        .bind(duration_hours)
        .bind(uses_remaining)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                DomainError::Conflict("Une malediction est deja active sur cette cible.".into())
            }
            _ => pg_err(e),
        })?;
        Ok(row.0)
    }

    async fn consume_one_use(&self, id: Uuid) -> Result<Option<i32>, DomainError> {
        // Decremente uses_remaining si > 0. Si tombe a 0, l update est
        // accompagne d un lifted_at = NOW() pour fermer la curse.
        // Atomic via CTE.
        let row: Option<(Option<i32>,)> = sqlx::query_as(
            r#"UPDATE coude_curses
               SET uses_remaining = uses_remaining - 1,
                   lifted_at = CASE
                       WHEN uses_remaining - 1 <= 0 THEN NOW()
                       ELSE lifted_at
                   END
               WHERE id = $1
                 AND lifted_at IS NULL
                 AND uses_remaining IS NOT NULL
                 AND uses_remaining > 0
               RETURNING uses_remaining"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.and_then(|r| r.0))
    }

    async fn lift(&self, id: Uuid, lifted_by: &str) -> Result<(), DomainError> {
        let result = sqlx::query(
            r#"UPDATE coude_curses
               SET lifted_at = NOW(), lifted_by = $2
               WHERE id = $1 AND lifted_at IS NULL"#,
        )
        .bind(id)
        .bind(lifted_by)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        if result.rows_affected() == 0 {
            return Err(DomainError::Conflict(
                "Malediction introuvable ou deja levee.".into(),
            ));
        }
        Ok(())
    }

    async fn list_active_by_source(
        &self,
        guild_id: &str,
        source_id: &str,
    ) -> Result<Vec<ActiveCurse>, DomainError> {
        let rows: Vec<CurseRow> = sqlx::query_as(
            r#"SELECT id, guild_id, target_id, source_id, kind,
                      created_at, expires_at, lifted_at, lifted_by, uses_remaining
               FROM coude_curses
               WHERE guild_id = $1 AND source_id = $2
                 AND lifted_at IS NULL AND expires_at > NOW()
               ORDER BY created_at DESC"#,
        )
        .bind(guild_id)
        .bind(source_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        rows.into_iter()
            .map(ActiveCurse::try_from)
            .collect::<Result<Vec<_>, _>>()
    }
}
