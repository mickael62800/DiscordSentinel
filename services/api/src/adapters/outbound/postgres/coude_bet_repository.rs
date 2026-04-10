use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::{BetResolutionPlan, CoudeBet, NewCoudeBet, RefundSummary};
use crate::domain::errors::DomainError;
use crate::ports::outbound::CoudeBetRepository;

pub struct PgCoudeBetRepository {
    pool: PgPool,
}

impl PgCoudeBetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct BetRow {
    id: i64,
    guild_id: String,
    combat_id: Uuid,
    bettor_id: String,
    bettor_name: String,
    backed_id: String,
    amount: i64,
    won: Option<bool>,
    payout: Option<i64>,
}

impl From<BetRow> for CoudeBet {
    fn from(r: BetRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            combat_id: r.combat_id,
            bettor_id: r.bettor_id,
            bettor_name: r.bettor_name,
            backed_id: r.backed_id,
            amount: r.amount,
            won: r.won,
            payout: r.payout,
        }
    }
}

fn pg_err(e: sqlx::Error) -> DomainError {
    DomainError::Internal(e.to_string())
}

#[async_trait]
impl CoudeBetRepository for PgCoudeBetRepository {
    async fn list_for_combat(&self, combat_id: Uuid) -> Result<Vec<CoudeBet>, DomainError> {
        let rows: Vec<BetRow> = sqlx::query_as(
            r#"SELECT id, guild_id, combat_id, bettor_id, bettor_name, backed_id, amount, won, payout
               FROM coude_bets
               WHERE combat_id = $1
               ORDER BY id"#,
        )
        .bind(combat_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn place(&self, new: NewCoudeBet) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Lock the bettor row + check balance.
        let bettor: Option<(i64,)> = sqlx::query_as(
            "SELECT coins FROM coude_players WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(&new.guild_id)
        .bind(&new.bettor_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?;

        let (bettor_coins,) = bettor
            .ok_or_else(|| DomainError::NotFound("Parieur introuvable".into()))?;

        if bettor_coins < new.amount {
            return Err(DomainError::ValidationError(format!(
                "Solde insuffisant ({} coins, {} requis)",
                bettor_coins, new.amount
            )));
        }

        // Debit.
        sqlx::query(
            "UPDATE coude_players SET coins = coins - $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(&new.guild_id)
        .bind(&new.bettor_id)
        .bind(new.amount)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        // Insert bet.
        sqlx::query(
            r#"INSERT INTO coude_bets
                 (guild_id, combat_id, bettor_id, bettor_name, backed_id, amount)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(&new.guild_id)
        .bind(new.combat_id)
        .bind(&new.bettor_id)
        .bind(&new.bettor_name)
        .bind(&new.backed_id)
        .bind(new.amount)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;
        Ok(())
    }

    async fn apply_resolution(
        &self,
        guild_id: &str,
        plan: BetResolutionPlan,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        for payout in &plan.payouts {
            // Crédite le joueur si gagné (inclut total_earned) OU si remboursement égalité
            // (payout = mise mais won = false : on rembourse mais sans toucher total_earned).
            if payout.won && payout.payout > 0 {
                sqlx::query(
                    r#"UPDATE coude_players
                       SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW()
                       WHERE guild_id = $1 AND user_id = $2"#,
                )
                .bind(guild_id)
                .bind(&payout.bettor_id)
                .bind(payout.payout)
                .execute(&mut *tx)
                .await
                .map_err(pg_err)?;
            } else if !payout.won && payout.payout > 0 {
                // Remboursement égalité : juste les coins, pas total_earned.
                sqlx::query(
                    "UPDATE coude_players SET coins = coins + $3, updated_at = NOW()
                     WHERE guild_id = $1 AND user_id = $2",
                )
                .bind(guild_id)
                .bind(&payout.bettor_id)
                .bind(payout.payout)
                .execute(&mut *tx)
                .await
                .map_err(pg_err)?;
            }

            sqlx::query("UPDATE coude_bets SET won = $2, payout = $3 WHERE id = $1")
                .bind(payout.bet_id)
                .bind(payout.won)
                .bind(payout.payout)
                .execute(&mut *tx)
                .await
                .map_err(pg_err)?;
        }

        if let Some(bonus) = &plan.fighter_bonus {
            if bonus.winner_bonus > 0 {
                sqlx::query(
                    r#"UPDATE coude_players
                       SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW()
                       WHERE guild_id = $1 AND user_id = $2"#,
                )
                .bind(guild_id)
                .bind(&bonus.winner_id)
                .bind(bonus.winner_bonus)
                .execute(&mut *tx)
                .await
                .map_err(pg_err)?;
            }
            if bonus.loser_bonus > 0 {
                sqlx::query(
                    r#"UPDATE coude_players
                       SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW()
                       WHERE guild_id = $1 AND user_id = $2"#,
                )
                .bind(guild_id)
                .bind(&bonus.loser_id)
                .bind(bonus.loser_bonus)
                .execute(&mut *tx)
                .await
                .map_err(pg_err)?;
            }
        }

        tx.commit().await.map_err(pg_err)?;
        Ok(())
    }

    async fn refund_unresolved(
        &self,
        guild_id: &str,
        combat_id: Uuid,
    ) -> Result<RefundSummary, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        let bets: Vec<(i64, String, i64)> = sqlx::query_as(
            "SELECT id, bettor_id, amount FROM coude_bets
             WHERE combat_id = $1 AND won IS NULL",
        )
        .bind(combat_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(pg_err)?;

        let mut refunded_total = 0i64;
        for (bet_id, bettor_id, amount) in &bets {
            sqlx::query(
                "UPDATE coude_players SET coins = coins + $3, updated_at = NOW()
                 WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(guild_id)
            .bind(bettor_id)
            .bind(*amount)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;

            sqlx::query("UPDATE coude_bets SET won = false, payout = $2 WHERE id = $1")
                .bind(bet_id)
                .bind(*amount)
                .execute(&mut *tx)
                .await
                .map_err(pg_err)?;

            refunded_total += amount;
        }

        tx.commit().await.map_err(pg_err)?;
        Ok(RefundSummary {
            refunded_count: bets.len(),
            refunded_total,
        })
    }
}
