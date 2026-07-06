use crate::domain::entities::system::discord_ids::GuildId;
use crate::domain::entities::system::discord_ids::UserId;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub user_id: UserId,
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
    pub guild_id: GuildId,
    pub user_id: UserId,
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

/// Solde de depart par defaut d'un wallet (coins offerts au premier
/// `get_or_create`).
pub const DEFAULT_STARTING_COINS: i64 = 100;

/// Resout le solde de depart d'un wallet, en respectant l'override
/// environnement `WALLET_STARTING_COINS`. Regle metier pure.
///
/// Le parametre `env_override` correspond a la valeur lue depuis l'env
/// (None si non defini). Un parsing invalide (non-numerique) retombe sur
/// la valeur par defaut.
pub fn resolve_starting_coins(env_override: Option<&str>) -> i64 {
    env_override
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_STARTING_COINS)
}

/// Valide qu'un montant pour credit/debit/transfer est strictement positif.
/// Regle metier : on n'accepte ni zero ni negatif (utiliser le handler
/// specifique `reset` pour remettre a zero).
/// Plafond metier d'une operation wallet unitaire. Empeche un montant absurde
/// (proche de i64::MAX) qui saturerait la ligne (overflow bigint -> toutes les
/// mutations suivantes echouent) ou dupliquerait des coins en masse.
pub const MAX_WALLET_AMOUNT: i64 = 1_000_000_000_000; // 1e12

pub fn validate_positive_amount(amount: i64) -> Result<(), &'static str> {
    if amount <= 0 {
        Err("Le montant doit etre positif")
    } else if amount > MAX_WALLET_AMOUNT {
        Err("Montant trop eleve")
    } else {
        Ok(())
    }
}

/// Valide qu'un transfert ne vise pas l'utilisateur lui-meme.
pub fn validate_transfer_distinct_users(from: &str, to: &str) -> Result<(), &'static str> {
    if from == to {
        Err("Impossible de transferer vers soi-meme")
    } else {
        Ok(())
    }
}

/// Resout le solde apres reset : valeur fournie ou default 100, floor 0.
pub fn resolve_reset_balance(input: Option<i64>) -> i64 {
    input.unwrap_or(DEFAULT_STARTING_COINS).max(0)
}

#[cfg(test)]
#[path = "tests/wallet.rs"]
mod tests;
