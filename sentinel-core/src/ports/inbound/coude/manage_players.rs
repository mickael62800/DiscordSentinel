use async_trait::async_trait;

use crate::domain::entities::coude::player::CombatStat;
use crate::domain::entities::coude::player::Player;
use crate::domain::entities::coude::player::XpProgress;
use crate::domain::entities::system::discord_ids::GuildId;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::errors::DomainError;

/// Vue transport d'un palier de niveau (donnees d'affichage + statut).
#[derive(Debug, Clone)]
pub struct MilestoneView {
    pub level: i32,
    pub key: String,
    pub label: String,
    pub emoji: String,
    pub description: String,
    pub unlocked: bool,
}

/// Etat de progression derive (succes + paliers + cooldown effectif), resolu
/// entierement server-side. Le bot ne fait que l'affichage.
#[derive(Debug, Clone)]
pub struct PlayerProgression {
    /// Clefs des succes debloques (le bot mappe vers emoji/label pour affichage).
    pub unlocked_achievements: Vec<String>,
    /// Nombre total de succes disponibles (pour l'affichage "n / total").
    pub total_achievements: i32,
    /// Tous les paliers avec leur statut de deblocage.
    pub milestones: Vec<MilestoneView>,
    /// Prochain palier a viser (None si tout est debloque).
    pub next_milestone: Option<MilestoneView>,
    /// Cooldown /repos effectif (heures) pour ce joueur.
    pub effective_repos_cooldown_hours: i64,
}

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
        guild_id: GuildId,
        user_id: UserId,
        username: String,
    ) -> Result<Player, DomainError>;

    async fn get(&self, guild_id: &str, user_id: &str) -> Result<Player, DomainError>;

    async fn list(&self, guild_id: &str) -> Result<Vec<Player>, DomainError>;

    async fn random_active(&self, guild_id: &str, count: i64) -> Result<Vec<Player>, DomainError>;

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

    async fn increment_cowardice(&self, guild_id: &str, user_id: &str) -> Result<i32, DomainError>;

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

    // ── Progression derivee (succes / paliers / cooldown) ──

    /// Cooldown /repos effectif (heures) pour ce joueur : lit le cooldown
    /// configure (`repos_cooldown_hours`, defaut 12) et applique le palier
    /// "Convalescence" (niveau 15+ -> plafond 8h). Bareme resolu server-side.
    /// Default `unimplemented!()` pour ne pas casser les mocks.
    async fn effective_repos_cooldown_hours(
        &self,
        _guild_id: &str,
        _user_id: &str,
    ) -> Result<i64, DomainError> {
        unimplemented!("effective_repos_cooldown_hours not implemented")
    }

    /// Etat de progression complet (succes debloques + paliers + cooldown
    /// effectif), derive server-side depuis les stats du joueur.
    /// Default `unimplemented!()` pour ne pas casser les mocks.
    async fn get_progression(
        &self,
        _guild_id: &str,
        _user_id: &str,
    ) -> Result<PlayerProgression, DomainError> {
        unimplemented!("get_progression not implemented")
    }

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
