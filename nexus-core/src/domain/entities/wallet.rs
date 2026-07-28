//! Wallet minimal Nexus — coins par (guild, user).
//!
//! Regles PURES de credit/debit. Le debit est clampe au solde : un joueur ne
//! peut jamais passer en negatif (regle reprise du wallet Sentinel,
//! cf. ancienne migration `080_create_user_wallets.sql` et le clamp du payout
//! negatif dans `manage_wheel_service`).

use crate::domain::errors::DomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wallet {
    pub guild_id: String,
    pub user_id: String,
    pub coins: i64,
    pub total_earned: i64,
    pub total_spent: i64,
}

impl Wallet {
    /// Wallet vierge (nouveau joueur) : 0 coins.
    pub fn new(guild_id: impl Into<String>, user_id: impl Into<String>) -> Self {
        Self {
            guild_id: guild_id.into(),
            user_id: user_id.into(),
            coins: 0,
            total_earned: 0,
            total_spent: 0,
        }
    }

    /// Credite `amount` coins (strictement positif).
    pub fn credit(&mut self, amount: i64) -> Result<(), DomainError> {
        if amount <= 0 {
            return Err(DomainError::Validation(
                "montant de credit invalide (doit etre > 0)".into(),
            ));
        }
        self.coins = self.coins.saturating_add(amount);
        self.total_earned = self.total_earned.saturating_add(amount);
        Ok(())
    }

    /// Debite `amount` coins (strictement positif), clampe au solde.
    /// Retourne le montant REELLEMENT debite (peut etre < amount, jamais < 0).
    pub fn debit_clamped(&mut self, amount: i64) -> Result<i64, DomainError> {
        if amount <= 0 {
            return Err(DomainError::Validation(
                "montant de debit invalide (doit etre > 0)".into(),
            ));
        }
        let actual = amount.min(self.coins);
        self.coins -= actual;
        self.total_spent = self.total_spent.saturating_add(actual);
        Ok(actual)
    }
}

/// Mutation appliquee au wallet, a journaliser dans
/// `nexus_wallet_transactions` (positif = credit, negatif = debit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletMutation {
    pub amount: i64,
    pub balance_after: i64,
    pub source: String,
    pub description: String,
}

#[cfg(test)]
#[path = "tests/wallet.rs"]
mod tests;
