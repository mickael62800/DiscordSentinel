use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::{
    BetResolutionPlan, CoudeBet, NewCoudeBet, RefundSummary, TauntEvent,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_wallet::ManageWalletUseCase;

use super::pg_err;
use crate::ports::outbound::CoudeBetRepository;

pub struct PgCoudeBetRepository {
    pool: PgPool,
    wallet_uc: Arc<dyn ManageWalletUseCase>,
}

impl PgCoudeBetRepository {
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
            crate::ports::inbound::manage_wallet::TxWalletMutation,
        )> = Vec::new();

        for payout in &plan.payouts {
            // Crédite le joueur si gagné (inclut total_earned via credit_tx) OU
            // si remboursement égalité (payout = mise mais won = false : on rembourse
            // SANS toucher total_earned, semantique legacy preservee via UPDATE direct).
            if payout.won && payout.payout > 0 {
                // Credit wallet unifie (credit_tx alimente total_earned + log tx)
                let desc = format!("Pari gagne combat {}", payout.bet_id);
                let m = self
                    .wallet_uc
                    .credit_tx(
                        &mut tx,
                        guild_id,
                        &payout.bettor_id,
                        payout.payout,
                        "coude_bet_win",
                        &desc,
                    )
                    .await?;
                pending_taunts.push((payout.bettor_id.clone(), m));

                // Stats coude_players.total_earned (double comptabilite legacy
                // preservee : wallet + coude_players).
                sqlx::query(
                    "UPDATE coude_players SET total_earned = total_earned + $3, updated_at = NOW()
                     WHERE guild_id = $1 AND user_id = $2",
                )
                .bind(guild_id)
                .bind(&payout.bettor_id)
                .bind(payout.payout)
                .execute(&mut *tx)
                .await
                .map_err(pg_err)?;
            } else if !payout.won && payout.payout > 0 {
                // Remboursement égalité : juste les coins (wallet), pas total_earned,
                // pas de taunt jackpot (c'est un remboursement pas un gain).
                // On reste sur un UPDATE direct + log_wallet_tx parce que credit_tx
                // incremente forcement total_earned (ce qu'on ne veut pas ici).
                let balance_after: i64 = sqlx::query_scalar(
                    "UPDATE user_wallets SET coins = coins + $3, updated_at = NOW()
                     WHERE guild_id = $1 AND user_id = $2
                     RETURNING coins",
                )
                .bind(guild_id)
                .bind(&payout.bettor_id)
                .bind(payout.payout)
                .fetch_one(&mut *tx)
                .await
                .map_err(pg_err)?;
                let desc = format!("Pari egalite - remboursement combat {}", payout.bet_id);
                super::wallet_tx_log::log_wallet_tx(
                    &mut tx,
                    guild_id,
                    &payout.bettor_id,
                    payout.payout,
                    balance_after,
                    "coude_bet_refund",
                    &desc,
                )
                .await?;
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

                sqlx::query(
                    "UPDATE coude_players SET total_earned = total_earned + $3, updated_at = NOW()
                     WHERE guild_id = $1 AND user_id = $2",
                )
                .bind(guild_id)
                .bind(&bonus.winner_id)
                .bind(bonus.winner_bonus)
                .execute(&mut *tx)
                .await
                .map_err(pg_err)?;
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

                sqlx::query(
                    "UPDATE coude_players SET total_earned = total_earned + $3, updated_at = NOW()
                     WHERE guild_id = $1 AND user_id = $2",
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
            // Refund : pas de total_earned (argent qui revient, pas un gain).
            // On reste sur UPDATE direct + log (cf. apply_resolution, cas
            // egalite). Pas de taunt non plus (remboursement = neutre).
            let balance_after: i64 = sqlx::query_scalar(
                "UPDATE user_wallets SET coins = coins + $3, updated_at = NOW()
                 WHERE guild_id = $1 AND user_id = $2
                 RETURNING coins",
            )
            .bind(guild_id)
            .bind(bettor_id)
            .bind(*amount)
            .fetch_one(&mut *tx)
            .await
            .map_err(pg_err)?;

            sqlx::query("UPDATE coude_bets SET won = false, payout = $2 WHERE id = $1")
                .bind(bet_id)
                .bind(*amount)
                .execute(&mut *tx)
                .await
                .map_err(pg_err)?;

            let desc = format!("Remboursement pari combat {}", combat_id);
            super::wallet_tx_log::log_wallet_tx(
                &mut tx,
                guild_id,
                bettor_id,
                *amount,
                balance_after,
                "coude_bet_unresolved_refund",
                &desc,
            )
            .await?;

            refunded_total += amount;
        }

        tx.commit().await.map_err(pg_err)?;
        Ok(RefundSummary {
            refunded_count: bets.len(),
            refunded_total,
        })
    }
}
