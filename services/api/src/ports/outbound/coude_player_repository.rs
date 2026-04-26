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

    // ── Économie / coins (stats-only depuis migration wallet finale) ──
    //
    // Ces methodes NE mutent PLUS `user_wallets`. Elles incrementent juste
    // les compteurs `total_earned` / `total_lost` de `coude_players`. Les
    // mouvements d'argent passent par `ManageWalletUseCase` (ou
    // `WalletRepository` pour les call sites qui ne peuvent injecter le use
    // case). `adjust_coins` (ajustement admin) a ete supprime : les handlers
    // HTTP/gRPC delegent directement au use case wallet.

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

    // ── Streaks (Phase 9 Part D) ──
    //
    // Mis a jour apres record_win/record_loss/record_draw, en dehors
    // de leur transaction (impact faible, 1 UPDATE supplementaire).
    // Retourne la nouvelle valeur, ou None si le joueur n'existe pas.

    /// Incremente `current_win_streak` et remet `current_loss_streak` a 0.
    async fn touch_win_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<i32>, DomainError>;

    /// Incremente `current_loss_streak` et remet `current_win_streak` a 0.
    async fn touch_loss_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<i32>, DomainError>;

    /// Remet a 0 les deux streaks de combat (draw).
    async fn reset_combat_streaks(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError>;

    /// Lecture des streaks combat courantes (win, loss) sans mutation.
    /// Utilise par la detection Régicide (cf. COUPE_AMELIORATIONS 5.3) :
    /// avant d incrementer la loss du perdant, on lit son win_streak
    /// pour savoir si le winner casse une streak >= 5.
    /// Default impl returns Ok(None) pour preserver les mocks existants.
    async fn get_combat_streaks(
        &self,
        _guild_id: &str,
        _user_id: &str,
    ) -> Result<Option<(i32, i32)>, DomainError> {
        Ok(None)
    }

    /// Lecture du compteur de prestige (cf. COUPE_AMELIORATIONS 3.3).
    /// Utilise pour appliquer le bonus +5%/prestige sur les gains. Default
    /// impl Ok(None) pour preserver les mocks existants — equivaut a
    /// "pas de bonus" (multiplier 1.0).
    async fn get_prestige_count(
        &self,
        _guild_id: &str,
        _user_id: &str,
    ) -> Result<Option<i32>, DomainError> {
        Ok(None)
    }

    /// Incremente `current_steal_victim_streak`. Retourne la nouvelle valeur.
    async fn touch_steal_victim_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<i32>, DomainError>;

    /// Remet a 0 `current_steal_victim_streak` (blocage reussi).
    async fn reset_steal_victim_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError>;

    // ── Blackjack streaks (migration 139) ──

    /// Incremente `bj_win_streak` et reset `bj_bust_streak`. Retourne la
    /// nouvelle valeur du `bj_win_streak`, ou None si le joueur n'existe
    /// pas.
    async fn touch_bj_win_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<i32>, DomainError>;

    /// Inverse de `touch_bj_win_streak` : incremente `bj_bust_streak` et
    /// reset `bj_win_streak`. Retourne la nouvelle valeur du
    /// `bj_bust_streak`.
    async fn touch_bj_bust_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<i32>, DomainError>;

    /// Reset `bj_bust_streak` (ex : blackjack naturel post-bust).
    async fn reset_bj_bust_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError>;

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

    /// Phase 4 refacto : tick de regeneration passive des HP. Regen degressive
    /// par palier de % HP courant (0-25% / 25-50% / 50-75% / 75-100%) avec
    /// les taux fournis en HP/h. Exclut les joueurs avec un combat actif
    /// (pending/betting/resolving) pour eviter d'ecraser un hp_current frais.
    /// Retourne le nombre de joueurs mis a jour.
    async fn regen_hp_tick(
        &self,
        rate_0_25: f64,
        rate_25_50: f64,
        rate_50_75: f64,
        rate_75_100: f64,
    ) -> Result<u64, DomainError>;
}

