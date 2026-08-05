use async_trait::async_trait;

use crate::domain::entities::community::bump::{BumpState, DueReminder};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait BumpRepository: Send + Sync {
    /// CAS de cooldown : met a jour `last_bump_at` (et reinitialise le rappel) du
    /// (guild, provider) UNIQUEMENT si le dernier bump date de plus de
    /// `cooldown_minutes`. Renvoie `true` si le creneau a ete reserve (donc
    /// recompensable), `false` si le bump tombe dans la fenetre de cooldown
    /// (doublon d'edition / rejeu / spam). Requete unique atomique (anti-TOCTOU).
    async fn try_claim_slot(
        &self,
        guild_id: &str,
        provider: &str,
        channel_id: &str,
        cooldown_minutes: i64,
        reminder_enabled: bool,
    ) -> Result<bool, DomainError>;
    /// Nombre de bumps du membre sur la fenetre glissante de 7 jours.
    async fn weekly_count(&self, guild_id: &str, user_id: &str) -> Result<i64, DomainError>;
    /// Nombre total (all-time) de bumps du membre (seuil VIP).
    async fn total_count(&self, guild_id: &str, user_id: &str) -> Result<i64, DomainError>;
    /// Journalise un bump (montant, index hebdo, provider).
    async fn record_event(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        reward_coins: i64,
        weekly_index: i64,
        provider: &str,
    ) -> Result<(), DomainError>;
    async fn due_reminders(&self) -> Result<Vec<DueReminder>, DomainError>;
    async fn mark_reminder_sent(
        &self,
        guild_id: &str,
        provider: Option<&str>,
    ) -> Result<(), DomainError>;
    /// Etats bump (par provider) d'une guild, pour la carte de statut.
    async fn guild_states(&self, guild_id: &str) -> Result<Vec<BumpState>, DomainError>;
}
