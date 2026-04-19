//! Use case unifie du wallet (point d'entree unique pour les mutations).
//!
//! Objectif : centraliser **toutes** les mutations de `user_wallets` derriere
//! une seule API qui :
//! 1. Execute la mutation en transaction atomique + log `wallet_transactions`
//!    (delegue au `WalletRepository` qui implemente deja ces garanties).
//! 2. Detecte les cas taunts economiques (faillite, jackpot) et retourne les
//!    `TauntEvent` dans la `WalletMutation` pour que le caller les propage.
//!
//! # Statut de migration
//!
//! A l'heure actuelle, beaucoup de call sites mutent encore `user_wallets`
//! directement via leurs repositories (coude_economy_repository,
//! coude_bet_repository, coude_player_repository, blackjack_repository). La
//! migration progressive vers ce use case se fait au fil des refactos.
//!
//! Voir `application/manage_wallet_service.rs` pour l'implementation.

use async_trait::async_trait;

use crate::domain::entities::TauntEvent;
use crate::domain::errors::DomainError;

/// Resultat d'une mutation de wallet. Contient le nouveau solde + les taunts
/// declenches par l'operation (faillite, jackpot) — a propager vers le bot.
#[derive(Debug, Clone)]
pub struct WalletMutation {
    pub new_balance: i64,
    pub previous_balance: i64,
    pub triggered_taunts: Vec<TauntEvent>,
}

#[async_trait]
pub trait ManageWalletUseCase: Send + Sync {
    /// Credite `amount` coins sur le wallet du joueur.
    /// `source` est typiquement `"combat"`, `"heist"`, `"steal"`, `"tournament_prize"`,
    /// `"casino_win"`, `"daily"` etc. Utilise pour l'audit trail.
    async fn credit(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<WalletMutation, DomainError>;

    /// Debite `amount` coins du wallet du joueur. Retourne une erreur de
    /// validation si le solde est insuffisant. Detecte la faillite (passage a
    /// zero) et declenche le taunt associe si applicable.
    async fn debit(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<WalletMutation, DomainError>;

    /// Transfert atomique entre deux wallets (don, payout combat).
    /// Declenche eventuellement un taunt de faillite cote emetteur et de
    /// jackpot cote recepteur.
    async fn transfer(
        &self,
        guild_id: &str,
        from_user: &str,
        to_user: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<Vec<TauntEvent>, DomainError>;

    /// Lecture simple du solde (utility).
    async fn get_balance(&self, guild_id: &str, user_id: &str) -> Result<i64, DomainError>;
}
