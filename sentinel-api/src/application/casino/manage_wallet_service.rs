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
use sqlx::Postgres;
use sqlx::Transaction;
use uuid::Uuid;

use sentinel_core::ports::uow::DbTx;
use crate::adapters::outbound::postgres::uow::as_pg;

use sentinel_core::domain::entities::casino::wallet::resolve_reset_balance;
use sentinel_core::domain::entities::casino::wallet::resolve_starting_coins;
use sentinel_core::domain::entities::coude::taunt::TauntEvent;
use sentinel_core::domain::entities::casino::wallet::Wallet;
use sentinel_core::domain::entities::casino::wallet::WalletTransaction;
use sentinel_core::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::casino::manage_wallet::TxWalletMutation;
use crate::ports::inbound::casino::manage_wallet::WalletMutation;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;

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

    /// Lit le solde courant. Propage les erreurs DB plutot que de retourner
    /// 0 silencieusement (qui causait des faux positifs faillite et des
    /// payouts incorrects en cas de connection timeout).
    async fn read_balance(&self, guild_id: &str, user_id: &str) -> Result<i64, DomainError> {
        Ok(self
            .repo
            .get(guild_id, user_id)
            .await?
            .map(|w| w.coins)
            .unwrap_or(0))
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

        let previous = self.read_balance(guild_id, user_id).await?;
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

        let previous = self.read_balance(guild_id, user_id).await?;
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

        let sender_before = self.read_balance(guild_id, from_user).await?;

        self.repo
            .transfer(guild_id, from_user, to_user, amount, source, description)
            .await?;

        let sender_after = self.read_balance(guild_id, from_user).await?;

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
        self.read_balance(guild_id, user_id).await
    }

    // ── Mode "tx en cours" ────────────────────────────────────────────────

    async fn credit_tx(
        &self,
        tx: &mut dyn DbTx,
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
        let tx = as_pg(tx);

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
        tx: &mut dyn DbTx,
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
        let tx = as_pg(tx);

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

    // ── Lectures + admin ─────────────────────────────────────────────────

    async fn get_or_create(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Wallet, DomainError> {
        let env = std::env::var("WALLET_STARTING_COINS").ok();
        let starting = resolve_starting_coins(env.as_deref());
        self.repo
            .get_or_create(guild_id, user_id, user_id, starting)
            .await
    }

    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<Wallet>, DomainError> {
        self.repo.list_by_guild(guild_id).await
    }

    async fn leaderboard(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<Wallet>, DomainError> {
        self.repo.leaderboard(guild_id, limit).await
    }

    async fn get_transactions(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<WalletTransaction>, DomainError> {
        self.repo.get_transactions(guild_id, user_id, limit).await
    }

    async fn reset_wallet(
        &self,
        guild_id: &str,
        user_id: &str,
        new_balance_input: Option<i64>,
    ) -> Result<(Wallet, i64), DomainError> {
        let new_balance = resolve_reset_balance(new_balance_input);
        let wallet = self.repo.reset_wallet(guild_id, user_id, new_balance).await?;
        Ok((wallet, new_balance))
    }

    async fn reset_all_wallets(
        &self,
        guild_id: &str,
        new_balance_input: Option<i64>,
    ) -> Result<(u64, i64), DomainError> {
        let new_balance = resolve_reset_balance(new_balance_input);
        let affected = self.repo.reset_all_wallets(guild_id, new_balance).await?;
        Ok((affected, new_balance))
    }
}


#[cfg(test)]
#[path = "tests/manage_wallet.rs"]
mod tests;
