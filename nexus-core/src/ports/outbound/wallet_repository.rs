//! Port outbound : persistance du wallet Nexus.

use async_trait::async_trait;

use crate::domain::entities::wallet::Wallet;
use crate::domain::entities::wallet::WalletMutation;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait WalletRepository: Send + Sync {
    /// Charge le wallet (guild, user), ou un wallet vierge s'il n'existe pas.
    async fn get_or_default(&self, guild_id: &str, user_id: &str) -> Result<Wallet, DomainError>;

    /// Persiste l'etat du wallet ET journalise la mutation dans
    /// `nexus_wallet_transactions` (upsert du wallet + insert transaction).
    async fn save_with_transaction(
        &self,
        wallet: &Wallet,
        mutation: &WalletMutation,
    ) -> Result<(), DomainError>;
}
