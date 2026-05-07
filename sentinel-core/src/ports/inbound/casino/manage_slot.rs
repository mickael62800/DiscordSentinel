//! Use case "machine a sous" / slot.
//!
//! Flow d un spin (cf. ManageSlotService) :
//!   1. Charger config (bot_guild_config: 'slot-bot' + parsing CSV)
//!   2. Validation mise (range, daily bonus exempt, cooldown)
//!   3. Tx atomique :
//!      - debit wallet (`debit_tx`) — sauf si is_daily=true
//!      - spin RNG + evaluate_spin
//!      - alimente jackpot pool (% de la mise)
//!      - log spin
//!      - si Jackpot : claim_jackpot_pool + reset
//!      - credit wallet (`credit_tx`) si payout > 0
//!      - mark_daily_claimed si is_daily
//!   4. Apres commit : post_commit_taunts (faillite/jackpot eco)
//!
//! Cooldown enforce avant ouverture de la tx pour limiter le bruit DB.

use async_trait::async_trait;

use crate::domain::entities::casino::slot::SlotSpin;
use crate::domain::entities::casino::slot::SlotTopWinner;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::errors::DomainError;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::entities::system::discord_ids::GuildId;

#[derive(Debug, Clone)]
pub struct SpinCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    /// Mise demandee. Ignoree si is_daily=true (utilise daily_bonus_mise de la config).
    pub mise: i64,
    /// Si true : utilise le spin gratuit quotidien (pas de debit wallet).
    pub is_daily: bool,
}

#[derive(Debug, Clone)]
pub struct SpinResult {
    pub spin: SlotSpin,
    /// Montant du pool jackpot APRES le spin (apres alimentation et eventuel reset).
    pub jackpot_pool_after: i64,
    /// Solde wallet du joueur apres credit du payout.
    pub balance_after: i64,
    /// Taunts declenches (faillite si solde tombe a 0, jackpot eco si gros payout).
    pub triggered_taunts: Vec<TauntEvent>,
}

#[async_trait]
pub trait ManageSlotUseCase: Send + Sync {
    /// Spin payant. Erreurs typiques :
    /// - ValidationError("Mise hors borne") si mise < min_bet ou > max_bet
    /// - ValidationError("Cooldown actif") si dernier spin trop recent
    /// - ValidationError("Solde insuffisant") si wallet < mise
    async fn spin(&self, cmd: SpinCommand) -> Result<SpinResult, DomainError>;

    /// Spin avec daily bonus (gratuit). Erreurs :
    /// - ValidationError("Daily bonus desactive")
    /// - ValidationError("Daily bonus deja reclame aujourd hui")
    async fn claim_daily_bonus(&self, cmd: SpinCommand) -> Result<SpinResult, DomainError>;

    /// Pool jackpot courant (0 si non initialise).
    async fn get_jackpot_pool(&self, guild_id: &str) -> Result<i64, DomainError>;

    /// Historique des derniers spins de la guild (tous joueurs).
    async fn recent_spins(&self, guild_id: &str, limit: i64) -> Result<Vec<SlotSpin>, DomainError>;

    /// Leaderboard sur les N derniers jours, top L joueurs.
    async fn top_winners(
        &self,
        guild_id: &str,
        days: i64,
        limit: i64,
    ) -> Result<Vec<SlotTopWinner>, DomainError>;
}
