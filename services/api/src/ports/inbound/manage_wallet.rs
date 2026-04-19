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
use sqlx::{Postgres, Transaction};

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

/// Resultat d'une mutation "dans une tx en cours" — ne contient pas les
/// taunts car ceux-ci sont detectes apres commit (le service qui owne la tx
/// doit rappeler `check_post_commit_taunts` apres `tx.commit()`).
#[derive(Debug, Clone)]
pub struct TxWalletMutation {
    pub new_balance: i64,
    pub previous_balance: i64,
    /// Indique qu'une faillite a potentiellement eu lieu (previous>0, new==0).
    /// Le caller declenchera le taunt associe APRES commit via
    /// `post_commit_bankruptcy_taunt`.
    pub maybe_bankruptcy: bool,
    /// Pour un credit : le montant credite (pour verifier le jackpot apres
    /// commit). None pour un debit.
    pub maybe_jackpot_amount: Option<i64>,
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

    // ─────────────────────────────────────────────────────────────────
    // Mode "tx en cours" — permet aux call sites qui ont deja une tx
    // composite (ex: mise pari = debit wallet + insert coude_bets) de
    // passer par le service sans casser leur atomicite.
    //
    // Ces variantes :
    //   - operent sur la tx fournie, ne commit pas
    //   - font le UPDATE user_wallets + INSERT wallet_transactions
    //   - NE declenchent PAS les taunts (detectes apres commit seulement,
    //     lire le champ `maybe_bankruptcy` / `maybe_jackpot_amount` puis
    //     appeler `post_commit_taunts` apres `tx.commit()`)
    // ─────────────────────────────────────────────────────────────────

    /// Credit dans une tx en cours. Met a jour user_wallets + log
    /// wallet_transactions sur la tx fournie, sans commit.
    async fn credit_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<TxWalletMutation, DomainError>;

    /// Debit dans une tx en cours. Verifie que le solde est suffisant,
    /// met a jour user_wallets + log wallet_transactions, sans commit.
    async fn debit_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<TxWalletMutation, DomainError>;

    /// Apres commit : joue les detections faillite/jackpot accumulees.
    /// Retourne la liste des TauntEvent a propager vers le bot.
    async fn post_commit_taunts(
        &self,
        guild_id: &str,
        user_id: &str,
        mutation: &TxWalletMutation,
    ) -> Vec<TauntEvent>;
}
