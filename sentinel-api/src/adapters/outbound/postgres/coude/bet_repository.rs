use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::coude::bet::BetPayoutOutcome;
use sentinel_core::domain::entities::coude::bet::BetResolutionPlan;
use sentinel_core::domain::entities::coude::bet::Bet;
use sentinel_core::domain::entities::coude::bet::NewCoudeBet;
use sentinel_core::domain::entities::coude::bet::RefundSummary;
use sentinel_core::domain::entities::coude::taunt::TauntEvent;
use sentinel_core::domain::errors::DomainError;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;

use super::super::pg_err;
use crate::ports::outbound::coude::bet_repository::BetRepository;

/// Refund "neutre" dans une tx en cours : credite les coins sans toucher
/// `total_earned` (l'argent revient, ce n'est pas un gain) et log la tx wallet.
/// Partage entre `apply_resolution` (egalite) et `refund_unresolved`.
async fn refund_wallet_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    guild_id: &str,
    user_id: &str,
    amount: i64,
    source: &str,
    description: &str,
) -> Result<(), DomainError> {
    let balance_after: i64 = sqlx::query_scalar(
        "UPDATE user_wallets SET coins = coins + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2
         RETURNING coins",
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(amount)
    .fetch_one(&mut **tx)
    .await
    .map_err(pg_err)?;
    super::super::casino::wallet_tx_log::log_wallet_tx(
        tx,
        guild_id,
        user_id,
        amount,
        balance_after,
        source,
        description,
    )
    .await
}

/// Double-comptabilite legacy : `coude_players.total_earned` suit les gains
/// wallet issus de paris et bonus combattants.
async fn bump_player_earned_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    guild_id: &str,
    user_id: &str,
    amount: i64,
) -> Result<(), DomainError> {
    sqlx::query(
        "UPDATE coude_players SET total_earned = total_earned + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(amount)
    .execute(&mut **tx)
    .await
    .map_err(pg_err)?;
    Ok(())
}

pub struct PgBetRepository {
    pool: PgPool,
    wallet_uc: Arc<dyn ManageWalletUseCase>,
}

impl PgBetRepository {
    pub fn new(pool: PgPool, wallet_uc: Arc<dyn ManageWalletUseCase>) -> Self {
        Self { pool, wallet_uc }
    }
}

#[derive(sqlx::FromRow)]
struct BetRow {
    id: Uuid,
    guild_id: String,
    combat_id: Uuid,
    bettor_id: String,
    bettor_name: String,
    backed_id: String,
    amount: i64,
    won: Option<bool>,
    payout: Option<i64>,
}

impl From<BetRow> for Bet {
    fn from(r: BetRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
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


#[async_trait]
impl BetRepository for PgBetRepository {
    async fn list_for_combat(&self, combat_id: Uuid) -> Result<Vec<Bet>, DomainError> {
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

    async fn place(&self, new: NewCoudeBet) -> Result<Vec<TauntEvent>, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Lock la row combat et re-verifie le status AVANT tout debit. Evite
        // la race : service-layer check, puis worker passe en 'resolving',
        // puis on debite le parieur sans que le pari puisse etre resolu.
        // Si worker detient deja le lock sur coude_combats (FOR UPDATE SKIP
        // LOCKED), on attend — sauf que worker utilise SKIP LOCKED donc on
        // n'est JAMAIS bloque (c'est le worker qui skip si on lock).
        let combat_status: Option<(String,)> = sqlx::query_as(
            "SELECT status FROM coude_combats WHERE id = $1 FOR UPDATE",
        )
        .bind(new.combat_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?;
        let combat_status = combat_status
            .ok_or_else(|| DomainError::NotFound("Combat introuvable".into()))?
            .0;
        if combat_status != "betting" {
            return Err(DomainError::ValidationError(format!(
                "Les paris ne sont pas ouverts pour ce combat (status: {combat_status})"
            )));
        }

        // Migration #7 : debit via wallet_uc.debit_tx (lock + solde + UPDATE
        // user_wallets + INSERT wallet_transactions atomique, dans notre tx).
        // Faillite detectee apres commit via post_commit_taunts.
        let desc = format!("Pari combat {} sur {}", new.combat_id, new.backed_id);
        let debit_mut = self
            .wallet_uc
            .debit_tx(&mut tx, &new.guild_id, &new.bettor_id, new.amount, "coude_bet_place", &desc)
            .await?;

        // Insert bet.
        sqlx::query(
            r#"INSERT INTO coude_bets
                 (guild_id, combat_id, bettor_id, bettor_name, backed_id, amount)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(new.guild_id.as_str())
        .bind(new.combat_id)
        .bind(&new.bettor_id)
        .bind(&new.bettor_name)
        .bind(&new.backed_id)
        .bind(new.amount)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;

        // Taunts declenches apres commit (faillite si solde a zero).
        let taunts = self
            .wallet_uc
            .post_commit_taunts(&new.guild_id, &new.bettor_id, &debit_mut)
            .await;
        Ok(taunts)
    }

    async fn apply_resolution(
        &self,
        guild_id: &str,
        plan: BetResolutionPlan,
    ) -> Result<Vec<TauntEvent>, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // On collecte (user_id, TxWalletMutation) pour detecter les taunts
        // APRES commit (jackpot credit sur gros payout).
        let mut pending_taunts: Vec<(
            String,
            crate::ports::inbound::casino::manage_wallet::TxWalletMutation,
        )> = Vec::new();

        for payout in &plan.payouts {
            // Dispatch sur la semantique metier explicite (cf. BetPayout::outcome).
            match payout.outcome() {
                BetPayoutOutcome::Win { amount } => {
                    let desc = format!("Pari gagne combat {}", payout.bet_id);
                    let m = self
                        .wallet_uc
                        .credit_tx(&mut tx, guild_id, &payout.bettor_id, amount, "coude_bet_win", &desc)
                        .await?;
                    pending_taunts.push((payout.bettor_id.clone(), m));
                    bump_player_earned_in_tx(&mut tx, guild_id, &payout.bettor_id, amount).await?;
                }
                BetPayoutOutcome::Refund { amount } => {
                    // Egalite : credit sans total_earned, pas de taunt.
                    let desc = format!("Pari egalite - remboursement combat {}", payout.bet_id);
                    refund_wallet_in_tx(
                        &mut tx,
                        guild_id,
                        &payout.bettor_id,
                        amount,
                        "coude_bet_refund",
                        &desc,
                    )
                    .await?;
                }
                BetPayoutOutcome::Loss => {
                    // Pari perdu : aucune mutation wallet.
                }
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
                let m = self
                    .wallet_uc
                    .credit_tx(
                        &mut tx,
                        guild_id,
                        &bonus.winner_id,
                        bonus.winner_bonus,
                        "coude_bet_fighter_bonus_win",
                        "Bonus combat gagne",
                    )
                    .await?;
                pending_taunts.push((bonus.winner_id.clone(), m));
                bump_player_earned_in_tx(&mut tx, guild_id, &bonus.winner_id, bonus.winner_bonus).await?;
            }
            if bonus.loser_bonus > 0 {
                let m = self
                    .wallet_uc
                    .credit_tx(
                        &mut tx,
                        guild_id,
                        &bonus.loser_id,
                        bonus.loser_bonus,
                        "coude_bet_fighter_bonus_lose",
                        "Consolation combat perdu",
                    )
                    .await?;
                pending_taunts.push((bonus.loser_id.clone(), m));
                bump_player_earned_in_tx(&mut tx, guild_id, &bonus.loser_id, bonus.loser_bonus).await?;
            }
        }

        tx.commit().await.map_err(pg_err)?;

        // Taunts apres commit (jackpot detecte par wallet_uc si amount > seuil).
        let mut taunts = Vec::new();
        for (user_id, mutation) in &pending_taunts {
            let evs = self
                .wallet_uc
                .post_commit_taunts(guild_id, user_id, mutation)
                .await;
            taunts.extend(evs);
        }
        Ok(taunts)
    }

    async fn refund_unresolved(
        &self,
        guild_id: &str,
        combat_id: Uuid,
    ) -> Result<RefundSummary, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        let bets: Vec<(Uuid, String, i64)> = sqlx::query_as(
            "SELECT id, bettor_id, amount FROM coude_bets
             WHERE combat_id = $1 AND won IS NULL",
        )
        .bind(combat_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(pg_err)?;

        let mut refunded_total = 0i64;
        for (bet_id, bettor_id, amount) in &bets {
            // Refund neutre : credite les coins sans toucher total_earned,
            // pas de taunt. Helper partage avec apply_resolution (cas egalite).
            let desc = format!("Remboursement pari combat {}", combat_id);
            refund_wallet_in_tx(
                &mut tx,
                guild_id,
                bettor_id,
                *amount,
                "coude_bet_unresolved_refund",
                &desc,
            )
            .await?;

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
