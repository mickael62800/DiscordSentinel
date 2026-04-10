use async_trait::async_trait;

use crate::domain::entities::{CombatStat, CoudePlayer, XpProgress};
use crate::domain::errors::DomainError;

/// Repository d'accès aux joueurs Coup de Coude.
///
/// Toutes les opérations qui touchent la table `coude_players` doivent passer
/// par ce port — les handlers HTTP ne doivent jamais écrire de SQL en direct.
#[async_trait]
pub trait CoudePlayerRepository: Send + Sync {
    // ── CRUD ──

    /// Récupère un joueur ; le crée si absent. Si l'on a déjà un row, met à jour
    /// son `username` (gestion des renommages Discord).
    async fn get_or_create(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<CoudePlayer, DomainError>;

    async fn get(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoudePlayer>, DomainError>;

    async fn list(&self, guild_id: &str, limit: i64) -> Result<Vec<CoudePlayer>, DomainError>;

    async fn random_active(
        &self,
        guild_id: &str,
        count: i64,
        min_coins: i64,
    ) -> Result<Vec<CoudePlayer>, DomainError>;

    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError>;

    // ── Mutations ciblées ──

    async fn update_class(
        &self,
        guild_id: &str,
        user_id: &str,
        class: &str,
    ) -> Result<bool, DomainError>;

    /// Ajoute de l'XP au joueur, applique les level-ups (avec lock pessimiste)
    /// et retourne le nouvel état de progression.
    ///
    /// Atomique : tout est fait dans une seule transaction `BEGIN/SELECT FOR UPDATE/UPDATE/COMMIT`.
    /// L'adapter applique les helpers domaine `coude_xp_for_level` / `coude_title_for_level` pour
    /// calculer les level-ups — ce n'est pas du métier, juste l'application déterministe d'un barème.
    async fn add_xp(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<Option<XpProgress>, DomainError>;

    /// Incrémente une stat (ATK ou DEF) en consommant 1 stat point. Retourne
    /// le joueur mis à jour, ou None si stat_points insuffisants.
    async fn spend_stat_point(
        &self,
        guild_id: &str,
        user_id: &str,
        stat: CombatStat,
    ) -> Result<Option<CoudePlayer>, DomainError>;

    /// Reset atomique : restitue les points dépensés en ATK/DEF dans `stat_points`,
    /// remet ATK/DEF à 0, déduit `cost` coins. Retourne le joueur mis à jour
    /// ou None si la garde SQL échoue (coins insuffisants ou rien à reset).
    async fn reset_stats(
        &self,
        guild_id: &str,
        user_id: &str,
        cost: i64,
    ) -> Result<Option<CoudePlayer>, DomainError>;

    // ── Économie / coins ──

    async fn adjust_coins(
        &self,
        guild_id: &str,
        user_id: &str,
        delta: i64,
    ) -> Result<bool, DomainError>;

    async fn record_coins_earned(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<bool, DomainError>;

    async fn record_coins_lost(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<bool, DomainError>;

    // ── Stats de combat ──

    async fn record_win(
        &self,
        guild_id: &str,
        user_id: &str,
        earned: i64,
        stolen: i64,
    ) -> Result<bool, DomainError>;

    async fn record_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<bool, DomainError>;

    async fn record_draw(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<bool, DomainError>;

    /// Incrémente le compteur de couardise et retourne sa nouvelle valeur.
    async fn increment_cowardice(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<i32>, DomainError>;

    async fn increment_chaos(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<bool, DomainError>;

    // ── HP ──

    async fn update_hp(
        &self,
        guild_id: &str,
        user_id: &str,
        hp_current: i32,
        hp_max: i32,
    ) -> Result<(), DomainError>;

    /// Restaure les HP au max (commande `/repos`) et touche `repos_last_used`.
    async fn full_heal(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError>;
}

