//! Use case railleries (Phase 9 Part D).
//!
//! Expose tout ce dont les callers (services de combat, service de vol,
//! handlers gRPC de config) ont besoin pour tracker les streaks et
//! emettre des TauntEvents. La totalite de la logique (seuils, messages,
//! suffixes) vit dans le domain `coude_taunt` et dans ce service.

use async_trait::async_trait;

use sentinel_core::domain::entities::coude::taunt::TauntsConfig;
use sentinel_core::domain::entities::coude::taunt::TauntEvent;
use sentinel_core::domain::errors::DomainError;

#[async_trait]
pub trait ManageCoudeTauntsUseCase: Send + Sync {
    // ── Tracking events (appeles par les services de combat/vol) ──

    /// Appele apres un combat gagne. Incremente le win_streak du joueur,
    /// reset le loss_streak, et retourne un TauntEvent si un seuil a ete
    /// franchi (None sinon).
    async fn on_player_won(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, DomainError>;

    /// Idem pour une defaite.
    async fn on_player_lost(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, DomainError>;

    /// Apres un egalite : reset les deux streaks de combat. Jamais d'event.
    async fn on_player_drew(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError>;

    /// Appele apres un vol reussi (victim perd des coins).
    async fn on_player_stolen_from(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, DomainError>;

    /// Appele quand une protection a bloque un vol (reset la streak de
    /// victime). Pas de TauntEvent ici.
    async fn on_player_defended_steal(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError>;

    // ── Blackjack (migration 139) ──

    /// Blackjack naturel (21 en 2 cartes). One-shot, pas de palier.
    /// Reset egalement la `bj_bust_streak` et ne touche pas la win streak
    /// (le caller appelera `on_bj_hand_won` a cote si approprie).
    async fn on_bj_natural(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, DomainError>;

    /// Main blackjack gagnee. Incremente `bj_win_streak`, reset
    /// `bj_bust_streak`. Retourne un TauntEvent si palier franchi.
    async fn on_bj_hand_won(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, DomainError>;

    /// Bust (depassement de 21). Incremente `bj_bust_streak`, reset
    /// `bj_win_streak`. Retourne un TauntEvent si palier franchi.
    async fn on_bj_hand_bust(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, DomainError>;

    // ── Economie (migration 139) ──

    /// Passage du wallet a 0 apres une operation.
    /// Lit `bankruptcy_taunt_enabled` dans bot_guild_config (default true).
    async fn on_bankruptcy(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<TauntEvent>, DomainError>;

    /// Gros gain en une operation. One-shot si `amount >= jackpot_threshold`
    /// (default 10_000).
    async fn on_jackpot(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<Option<TauntEvent>, DomainError>;

    /// Don significatif vers un autre joueur. One-shot si
    /// `amount >= generous_donor_threshold` (default 1_000).
    async fn on_generous_donor(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<Option<TauntEvent>, DomainError>;

    // ── Config (exposee par les RPCs d'admin) ──

    async fn get_config(&self, guild_id: &str) -> Result<TauntsConfig, DomainError>;

    async fn set_channel(
        &self,
        guild_id: &str,
        channel_id: Option<&str>,
    ) -> Result<(), DomainError>;

    async fn set_enabled(&self, guild_id: &str, enabled: bool) -> Result<(), DomainError>;

    async fn set_rename_enabled(&self, guild_id: &str, rename_enabled: bool) -> Result<(), DomainError>;

    async fn set_messages_enabled(&self, guild_id: &str, messages_enabled: bool) -> Result<(), DomainError>;

    async fn set_opt_out(
        &self,
        guild_id: &str,
        user_id: &str,
        opted_out: bool,
    ) -> Result<(), DomainError>;

    async fn is_opted_out(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError>;

    async fn list_opt_outs(&self, guild_id: &str) -> Result<Vec<String>, DomainError>;
}
