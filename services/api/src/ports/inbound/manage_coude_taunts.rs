//! Use case railleries (Phase 9 Part D).
//!
//! Expose tout ce dont les callers (services de combat, service de vol,
//! handlers gRPC de config) ont besoin pour tracker les streaks et
//! emettre des TauntEvents. La totalite de la logique (seuils, messages,
//! suffixes) vit dans le domain `coude_taunt` et dans ce service.

use async_trait::async_trait;

use crate::domain::entities::{CoudeTauntsConfig, TauntEvent};
use crate::domain::errors::DomainError;

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

    // ── Config (exposee par les RPCs d'admin) ──

    async fn get_config(&self, guild_id: &str) -> Result<CoudeTauntsConfig, DomainError>;

    async fn set_channel(
        &self,
        guild_id: &str,
        channel_id: Option<&str>,
    ) -> Result<(), DomainError>;

    async fn set_enabled(&self, guild_id: &str, enabled: bool) -> Result<(), DomainError>;

    async fn set_opt_out(
        &self,
        guild_id: &str,
        user_id: &str,
        opted_out: bool,
    ) -> Result<(), DomainError>;

    async fn is_opted_out(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError>;

    async fn list_opt_outs(&self, guild_id: &str) -> Result<Vec<String>, DomainError>;
}
