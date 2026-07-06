//! Tresorerie d'organisation — cagnotte commune libellee dans la monnaie
//! partagee (`user_wallets`). Un depot debite le wallet du membre et incremente
//! la tresorerie ; un retrait fait l'inverse. Historique append-only.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Nature d'un mouvement de tresorerie (le signe est porte par le `kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreasuryKind {
    /// Un membre reverse des coins a l'organisation.
    Deposit,
    /// Un dirigeant retire des coins vers son wallet.
    Withdrawal,
}

impl TreasuryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TreasuryKind::Deposit => "deposit",
            TreasuryKind::Withdrawal => "withdrawal",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            TreasuryKind::Deposit => "Dépôt",
            TreasuryKind::Withdrawal => "Retrait",
        }
    }
}

/// Un mouvement immuable de la tresorerie (audit / historique).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreasuryMovement {
    pub kind: TreasuryKind,
    pub amount: i64,
    pub treasury_after: i64,
    pub actor_username: String,
    pub created_at: DateTime<Utc>,
}

/// Vue de la tresorerie pour `/org tresorerie`.
#[derive(Debug, Clone)]
pub struct TreasuryView {
    pub org_name: String,
    pub balance: i64,
    pub movements: Vec<TreasuryMovement>,
}

/// Identifiant d'un mouvement (retour de dépôt/retrait, non affiché).
pub type MovementId = Uuid;
