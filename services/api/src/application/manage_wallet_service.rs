//! Implementation du use case wallet unifie.
//!
//! # Architecture
//!
//! Ce service expose DEUX modes :
//!
//! 1. **Standalone** (`credit`, `debit`, `transfer`) — le service ouvre sa
//!    propre tx, fait le UPDATE + INSERT wallet_transactions, commit, puis
//!    detecte les taunts (faillite/jackpot) et les retourne dans la
//!    `WalletMutation`. Point d'entree simple pour les call sites qui n'ont
//!    pas de tx composite.
//!
//! 2. **Tx en cours** (`credit_tx`, `debit_tx`, `post_commit_taunts`) — le
//!    call site passe sa propre tx sqlx. Le service fait l'UPDATE + INSERT
//!    sur cette tx, sans commit. Le champ `maybe_bankruptcy` /
//!    `maybe_jackpot_amount` de la `TxWalletMutation` permet au call site
//!    de rappeler `post_commit_taunts` APRES son `tx.commit()` pour
//!    recuperer la liste de `TauntEvent` a propager.
//!
//! Ce double mode permet de migrer progressivement les ~25 call sites de
//! mutations `user_wallets` sans casser leurs tx composites (mise pari +
//! insert bet_row, record_casino_win + coude_players + casino_log, etc.).
//!
//! # Detection de taunts
//!
//! - **Faillite** : debit dont le passage strict `previous > 0 → new == 0`.
//! - **Jackpot** : credit avec `amount >= threshold_from_taunts_config`
//!   (default 10_000, configurable par `jackpot_threshold`).

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::domain::entities::TauntEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_taunts::ManageCoudeTauntsUseCase;
use crate::ports::inbound::manage_wallet::{
    ManageWalletUseCase, TxWalletMutation, WalletMutation,
};
use crate::ports::outbound::WalletRepository;

pub struct ManageWalletService {
    repo: Arc<dyn WalletRepository>,
    taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
}

impl ManageWalletService {
    pub fn new(
        repo: Arc<dyn WalletRepository>,
        taunts_uc: Arc<dyn ManageCoudeTauntsUseCase>,
    ) -> Self {
        Self { repo, taunts_uc }
    }

    async fn read_balance(&self, guild_id: &str, user_id: &str) -> i64 {
        self.repo
            .get(guild_id, user_id)
            .await
            .ok()
            .flatten()
            .map(|w| w.coins)
            .unwrap_or(0)
    }
}

/// Ecrit une ligne dans `wallet_transactions` au sein de la tx fournie.
async fn insert_wallet_tx(
    tx: &mut Transaction<'_, Postgres>,
    guild_id: &str,
    user_id: &str,
    signed_amount: i64,
    balance_after: i64,
    source: &str,
    description: &str,
) -> Result<(), DomainError> {
    sqlx::query(
        "INSERT INTO wallet_transactions (id, guild_id, user_id, amount, balance_after, source, description, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(guild_id)
    .bind(user_id)
    .bind(signed_amount)
    .bind(balance_after)
    .bind(source)
    .bind(description)
    .execute(&mut **tx)
    .await
    .map_err(|e| DomainError::Internal(format!("insert wallet_transactions: {e}")))?;
    Ok(())
}

#[async_trait]
impl ManageWalletUseCase for ManageWalletService {
    async fn credit(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<WalletMutation, DomainError> {
        if amount <= 0 {
            return Err(DomainError::ValidationError(
                "Montant credit doit etre positif".into(),
            ));
        }

        let previous = self.read_balance(guild_id, user_id).await;
        let wallet = self
            .repo
            .credit(guild_id, user_id, amount, source, description)
            .await?;

        let mut triggered_taunts = Vec::new();

        // Jackpot detection : le palier est porte par la taunts config (default
        // 10_000 cote service taunts).
        if let Ok(Some(evt)) = self
            .taunts_uc
            .on_jackpot(guild_id, user_id, amount)
            .await
        {
            triggered_taunts.push(evt);
        }

        Ok(WalletMutation {
            new_balance: wallet.coins,
            previous_balance: previous,
            triggered_taunts,
        })
    }

    async fn debit(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<WalletMutation, DomainError> {
        if amount <= 0 {
            return Err(DomainError::ValidationError(
                "Montant debit doit etre positif".into(),
            ));
        }

        let previous = self.read_balance(guild_id, user_id).await;
        let wallet = self
            .repo
            .debit(guild_id, user_id, amount, source, description)
            .await?;

        let mut triggered_taunts = Vec::new();

        // Faillite : passage strict de >0 a 0.
        if previous > 0 && wallet.coins == 0 {
            if let Ok(Some(evt)) = self.taunts_uc.on_bankruptcy(guild_id, user_id).await {
                triggered_taunts.push(evt);
            }
        }

        Ok(WalletMutation {
            new_balance: wallet.coins,
            previous_balance: previous,
            triggered_taunts,
        })
    }

    async fn transfer(
        &self,
        guild_id: &str,
        from_user: &str,
        to_user: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<Vec<TauntEvent>, DomainError> {
        if amount <= 0 {
            return Err(DomainError::ValidationError(
                "Montant transfer doit etre positif".into(),
            ));
        }
        if from_user == to_user {
            return Err(DomainError::ValidationError(
                "Impossible de se transferer a soi-meme".into(),
            ));
        }

        let sender_before = self.read_balance(guild_id, from_user).await;

        self.repo
            .transfer(guild_id, from_user, to_user, amount, source, description)
            .await?;

        let sender_after = self.read_balance(guild_id, from_user).await;

        let mut triggered = Vec::new();

        // Faillite cote emetteur.
        if sender_before > 0 && sender_after == 0 {
            if let Ok(Some(evt)) = self.taunts_uc.on_bankruptcy(guild_id, from_user).await {
                triggered.push(evt);
            }
        }
        // Jackpot cote recepteur.
        if let Ok(Some(evt)) = self.taunts_uc.on_jackpot(guild_id, to_user, amount).await {
            triggered.push(evt);
        }

        Ok(triggered)
    }

    async fn get_balance(&self, guild_id: &str, user_id: &str) -> Result<i64, DomainError> {
        Ok(self.read_balance(guild_id, user_id).await)
    }

    // ── Mode "tx en cours" ────────────────────────────────────────────────

    async fn credit_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<TxWalletMutation, DomainError> {
        if amount <= 0 {
            return Err(DomainError::ValidationError(
                "Montant credit doit etre positif".into(),
            ));
        }

        // Lock le wallet et lit le solde actuel dans la tx. On fait un
        // SELECT FOR UPDATE prealable pour exposer `previous_balance`
        // fiable (utile pour detection jackpot meme si on pourrait s'en
        // passer sur un simple credit).
        let previous: Option<i64> = sqlx::query_scalar(
            "SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| DomainError::Internal(format!("credit_tx select: {e}")))?;

        let previous = previous
            .ok_or_else(|| DomainError::NotFound("Portefeuille introuvable".into()))?;

        let balance_after: i64 = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = coins + $1, total_earned = total_earned + $1, updated_at = NOW() \
             WHERE guild_id = $2 AND user_id = $3 RETURNING coins",
        )
        .bind(amount)
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| DomainError::Internal(format!("credit_tx update: {e}")))?;

        insert_wallet_tx(tx, guild_id, user_id, amount, balance_after, source, description).await?;

        Ok(TxWalletMutation {
            new_balance: balance_after,
            previous_balance: previous,
            maybe_bankruptcy: false,
            maybe_jackpot_amount: Some(amount),
        })
    }

    async fn debit_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<TxWalletMutation, DomainError> {
        if amount <= 0 {
            return Err(DomainError::ValidationError(
                "Montant debit doit etre positif".into(),
            ));
        }

        // Lock + verifie solde suffisant.
        let previous: Option<i64> = sqlx::query_scalar(
            "SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| DomainError::Internal(format!("debit_tx select: {e}")))?;

        let previous = previous
            .ok_or_else(|| DomainError::NotFound("Portefeuille introuvable".into()))?;

        if previous < amount {
            return Err(DomainError::ValidationError(format!(
                "Solde insuffisant : tu as {} coins, il en faut {} (manque {}).",
                previous,
                amount,
                amount - previous
            )));
        }

        let balance_after: i64 = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = coins - $1, total_spent = total_spent + $1, updated_at = NOW() \
             WHERE guild_id = $2 AND user_id = $3 RETURNING coins",
        )
        .bind(amount)
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| DomainError::Internal(format!("debit_tx update: {e}")))?;

        insert_wallet_tx(tx, guild_id, user_id, -amount, balance_after, source, description).await?;

        Ok(TxWalletMutation {
            new_balance: balance_after,
            previous_balance: previous,
            maybe_bankruptcy: previous > 0 && balance_after == 0,
            maybe_jackpot_amount: None,
        })
    }

    async fn post_commit_taunts(
        &self,
        guild_id: &str,
        user_id: &str,
        mutation: &TxWalletMutation,
    ) -> Vec<TauntEvent> {
        let mut out = Vec::new();
        if mutation.maybe_bankruptcy {
            if let Ok(Some(evt)) = self.taunts_uc.on_bankruptcy(guild_id, user_id).await {
                out.push(evt);
            }
        }
        if let Some(amount) = mutation.maybe_jackpot_amount {
            if let Ok(Some(evt)) = self.taunts_uc.on_jackpot(guild_id, user_id, amount).await {
                out.push(evt);
            }
        }
        out
    }
}


#[cfg(test)]
#[path = "tests/manage_wallet.rs"]
mod tests;
