//! Impl Postgres de `TournamentRepository` (migration 139).

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::ports::outbound::coude::tournament_repository::TournamentRepository;
use sentinel_core::domain::entities::coude::tournament::PastTournament;
use sentinel_core::domain::errors::DomainError;

use super::super::pg_ctx;

pub struct PgTournamentRepository {
    pool: PgPool,
}

impl PgTournamentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TournamentRepository for PgTournamentRepository {
    async fn weekly_net_gains(
        &self,
        guild_id: &str,
        week_start: DateTime<Utc>,
        week_end: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<(String, i64)>, DomainError> {
        sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT wt.user_id, COALESCE(SUM(wt.amount), 0)::BIGINT AS net
            FROM wallet_transactions wt
            WHERE wt.guild_id = $1
              AND wt.created_at >= $2
              AND wt.created_at <= $3
            GROUP BY wt.user_id
            ORDER BY net DESC
            LIMIT $4
            "#,
        )
        .bind(guild_id)
        .bind(week_start)
        .bind(week_end)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("tournaments weekly_net_gains"))
    }

    async fn usernames(
        &self,
        guild_id: &str,
        user_ids: &[String],
    ) -> Result<Vec<(String, String)>, DomainError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT user_id, username
            FROM user_wallets
            WHERE guild_id = $1
              AND user_id = ANY($2)
            "#,
        )
        .bind(guild_id)
        .bind(user_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("tournaments usernames"))
    }

    async fn cashbox_balance(&self, guild_id: &str) -> Result<Option<i64>, DomainError> {
        sqlx::query_scalar::<_, i64>("SELECT balance FROM coude_cashbox WHERE guild_id = $1")
            .bind(guild_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_ctx("tournaments cashbox_balance"))
    }

    async fn list_past_tournaments(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<PastTournament>, DomainError> {
        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                DateTime<Utc>,
                DateTime<Utc>,
                Option<String>,
                Option<String>,
                Option<i64>,
                i64,
                String,
                Option<DateTime<Utc>>,
            ),
        >(
            r#"
            SELECT id, guild_id, week_start, week_end, winner_user_id,
                   winner_username, winner_net_gain, prize_amount, status, resolved_at
            FROM coude_weekly_tournaments
            WHERE guild_id = $1
            ORDER BY week_start DESC
            LIMIT $2
            "#,
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("tournaments list_past_tournaments"))?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    guild_id,
                    week_start,
                    week_end,
                    winner_user_id,
                    winner_username,
                    winner_net_gain,
                    prize_amount,
                    status,
                    resolved_at,
                )| PastTournament {
                    id: id.to_string(),
                    guild_id,
                    week_start,
                    week_end,
                    winner_user_id,
                    winner_username,
                    winner_net_gain: winner_net_gain.unwrap_or(0),
                    prize_amount,
                    status,
                    resolved_at,
                },
            )
            .collect())
    }
}
