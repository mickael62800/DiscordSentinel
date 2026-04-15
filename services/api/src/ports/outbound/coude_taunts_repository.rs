//! Port outbound pour la config taunts (Phase 9 Part D).

use async_trait::async_trait;

use crate::domain::entities::CoudeTauntsConfig;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait CoudeTauntsRepository: Send + Sync {
    /// Retourne la config (cree une row vide si absente) avec enabled=true.
    async fn get_or_init_config(&self, guild_id: &str) -> Result<CoudeTauntsConfig, DomainError>;

    /// Met a jour le channel_id (None = desactive sans perdre la row).
    async fn set_channel(
        &self,
        guild_id: &str,
        channel_id: Option<&str>,
    ) -> Result<(), DomainError>;

    /// Active/desactive globalement la feature pour une guild.
    async fn set_enabled(&self, guild_id: &str, enabled: bool) -> Result<(), DomainError>;

    /// True si le joueur est opted out des taunts.
    async fn is_opted_out(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError>;

    /// Set / clear l'opt-out d'un joueur.
    async fn set_opt_out(
        &self,
        guild_id: &str,
        user_id: &str,
        opted_out: bool,
    ) -> Result<(), DomainError>;
}
