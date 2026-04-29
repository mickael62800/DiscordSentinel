//! Impl Postgres de `UltimateRepository`.

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;

use crate::domain::entities::coude::ultimate::UltimateKind;
use crate::domain::entities::coude::ultimate::UltimateState;
use crate::domain::errors::DomainError;
use crate::ports::outbound::coude::ultimate_repository::UltimateRepository;

use super::super::pg_err_ctx;
const TBL: &str = "coude_ultimate_states";
fn pg_err(e: sqlx::Error) -> DomainError { pg_err_ctx(TBL, e) }

pub struct PgUltimateRepository {
    pool: PgPool,
}

impl PgUltimateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UltimateRepository for PgUltimateRepository {
    async fn activate(
        &self,
        guild_id: &str,
        user_id: &str,
        kind: UltimateKind,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO coude_ultimate_states
                   (guild_id, user_id, pending_kind, last_used_at, activated_at)
               VALUES ($1, $2, $3, NOW(), NOW())
               ON CONFLICT (guild_id, user_id) DO UPDATE
                   SET pending_kind = EXCLUDED.pending_kind,
                       last_used_at = EXCLUDED.last_used_at,
                       activated_at = EXCLUDED.activated_at"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(kind.as_db_str())
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn get(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<UltimateState, DomainError> {
        let row: Option<(Option<String>, Option<DateTime<Utc>>, Option<DateTime<Utc>>)> =
            sqlx::query_as(
                r#"SELECT pending_kind, last_used_at, activated_at
                   FROM coude_ultimate_states
                   WHERE guild_id = $1 AND user_id = $2"#,
            )
            .bind(guild_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        let (pending_kind, last_used_at, activated_at) = match row {
            Some((p, l, a)) => {
                let kind = p.as_deref().and_then(UltimateKind::from_db_str);
                (kind, l, a)
            }
            None => (None, None, None),
        };
        Ok(UltimateState {
            guild_id: guild_id.into(),
            user_id: user_id.into(),
            pending_kind,
            last_used_at,
            activated_at,
        })
    }

    async fn consume_pending(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UltimateKind>, DomainError> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            r#"UPDATE coude_ultimate_states
               SET pending_kind = NULL
               WHERE guild_id = $1 AND user_id = $2 AND pending_kind IS NOT NULL
               RETURNING pending_kind"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        // Le RETURNING renvoie l ancienne valeur ? Non, postgres RETURNING
        // donne la nouvelle (NULL apres update). Pour avoir l ancienne il
        // faut un CTE. Utilisons un select prealable a la place pour
        // simplifier.
        // -> on ignore le row, on a deja vide. On reflechit.
        // Implementation 2 : CTE avec select avant update.
        let _ = row;
        // Re-run avec la bonne semantique :
        let row: Option<(String,)> = sqlx::query_as(
            r#"WITH old AS (
                   SELECT pending_kind FROM coude_ultimate_states
                   WHERE guild_id = $1 AND user_id = $2
                   FOR UPDATE
               )
               UPDATE coude_ultimate_states
               SET pending_kind = NULL
               WHERE guild_id = $1 AND user_id = $2
                 AND pending_kind IS NOT NULL
               RETURNING (SELECT pending_kind FROM old)"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.and_then(|(s,)| UltimateKind::from_db_str(&s)))
    }
}
