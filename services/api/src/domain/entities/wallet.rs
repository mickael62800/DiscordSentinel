use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub coins: i64,
    pub total_earned: i64,
    pub total_spent: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransaction {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub amount: i64,
    pub balance_after: i64,
    pub source: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

/// Clamp un montant a debiter en respectant l'invariant "on ne debite
/// jamais plus que le solde disponible, ni une valeur negative".
///
/// Retourne la valeur effective a debiter (0 si solde <= 0 ou si `amount`
/// est negatif, sinon `min(amount, balance)`).
///
/// Regle metier pure — utilisee par les handlers qui doivent pre-clamper
/// avant appel a `wallet_uc.debit` pour preserver un comportement legacy
/// "best-effort" (ne pas echouer quand le solde est insuffisant mais juste
/// debiter ce qui reste).
pub fn clamp_debit_to_balance(amount: i64, balance: i64) -> i64 {
    amount.min(balance).max(0)
}

#[cfg(test)]
#[path = "tests/wallet.rs"]
mod tests;
