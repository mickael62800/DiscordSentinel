//! Implementation du use case wallet unifie.
//!
//! Delegue la mutation atomique + log `wallet_transactions` au
//! `WalletRepository` (qui contient deja les bonnes garanties transactionnelles).
//!
//! Apres commit, detecte :
//! - **Faillite** : passage strict de `previous > 0` a `new == 0` apres debit →
//!   appel `taunts_uc.on_bankruptcy`.
//! - **Jackpot** : credit avec `amount >= threshold_from_taunts_config` →
//!   appel `taunts_uc.on_jackpot`.
//!
//! Les `TauntEvent` sont accumules dans `WalletMutation.triggered_taunts` et
//! retournes au caller pour propagation (gRPC ou Redis stream).

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::TauntEvent;
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_taunts::ManageCoudeTauntsUseCase;
use crate::ports::inbound::manage_wallet::{ManageWalletUseCase, WalletMutation};
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
}

