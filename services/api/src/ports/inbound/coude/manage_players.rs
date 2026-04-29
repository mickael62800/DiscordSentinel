use async_trait::async_trait;

use crate::domain::entities::coude::player::CombatStat;
use crate::domain::entities::coude::player::Player;
use crate::domain::entities::coude::player::XpProgress;
use crate::domain::errors::DomainError;

/// Use case "gérer les joueurs Coup de Coude".
///
/// Englobe le cycle de vie d'un joueur (CRUD), la progression (XP/level/stats),
/// les compteurs de combats (wins/losses/draws/cowardice/chaos), les mouvements
/// de coins liés au joueur, et les HP.
///
/// Note : les opérations purement économiques inter-joueurs (transferts, vols,
/// casino) ainsi que les combats relèvent d'autres use cases dédiés.
#[async_trait]
pub trait ManageCoudePlayersUseCase: Send + Sync {
    // ── CRUD ──

    async fn get_or_create(
        &self,
        guild_id: String,
        user_id: String,
        username: String,
    ) -> Result<Player, DomainError>;

    async fn get(&self, guild_id: &str, user_id: &str) -> Result<Player, DomainError>;

    async fn list(&self, guild_id: &str) -> Result<Vec<Player>, DomainError>;

    async fn random_active(
        &self,
        guild_id: &str,
        count: i64,
    ) -> Result<Vec<Player>, DomainError>;

    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError>;

    // ── Progression ──

    async fn update_class(
        &self,
        guild_id: &str,
        user_id: &str,
        class: &str,
    ) -> Result<(), DomainError>;

    async fn add_xp(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<XpProgress, DomainError>;

    async fn spend_stat_point(
        &self,
        guild_id: &str,
        user_id: &str,
        stat: CombatStat,
    ) -> Result<Player, DomainError>;

    async fn reset_stats(
        &self,
        guild_id: &str,
        user_id: &str,
        cost: i64,
    ) -> Result<Player, DomainError>;

    // ── Compteurs combat ──

    async fn record_win(
        &self,
        guild_id: &str,
        user_id: &str,
        earned: i64,
        stolen: i64,
    ) -> Result<(), DomainError>;

    async fn record_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), DomainError>;

    async fn record_draw(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), DomainError>;

    async fn increment_cowardice(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i32, DomainError>;

    async fn increment_chaos(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;

    // ── Coins (stats-only : incrementent juste total_earned/total_lost) ──
    //
    // Les mouvements d'argent vers `user_wallets` passent par
    // `ManageWalletUseCase`. Ces methodes ne font plus que l'update stats.
    // `adjust_coins` a ete supprime : les handlers delegent directement au
    // use case wallet.

    async fn record_coins_earned(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<(), DomainError>;

    async fn record_coins_lost(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<(), DomainError>;

    // ── HP ──

    async fn update_hp(
        &self,
        guild_id: &str,
        user_id: &str,
        hp_current: i32,
        hp_max: i32,
    ) -> Result<(), DomainError>;

    async fn full_heal(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;

    /// Phase 4 : tick batch de regeneration passive des HP. Retourne le nombre
    /// de joueurs mis a jour.
    async fn regen_hp_tick(
        &self,
        rate_0_25: f64,
        rate_25_50: f64,
        rate_50_75: f64,
        rate_75_100: f64,
    ) -> Result<u64, DomainError>;
}
