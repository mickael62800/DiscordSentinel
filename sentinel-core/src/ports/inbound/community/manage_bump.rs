use async_trait::async_trait;

use crate::domain::entities::community::bump::{BumpReward, DueReminder};
use crate::domain::errors::DomainError;

/// Commande d'enregistrement d'un bump constate par le bot.
pub struct RecordBumpCommand {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    /// Salon ou le bot a poste (fallback si bump_channel_id non configure).
    pub channel_id: String,
    /// Plateforme ("disboard" | "discordl" | ...). Sera normalisee.
    pub provider: String,
}

#[async_trait]
pub trait ManageBumpUseCase: Send + Sync {
    /// Enregistre un bump : applique le cooldown atomique, calcule la recompense
    /// graduee de la semaine, credite le wallet et evalue le seuil VIP.
    async fn record_bump(&self, cmd: RecordBumpCommand) -> Result<BumpReward, DomainError>;
    /// (guild, provider) dont le cooldown est ecoule et le rappel non envoye.
    async fn due_reminders(&self) -> Result<Vec<DueReminder>, DomainError>;
    /// Marque le rappel envoye pour un provider (ou tous si `None`, retrocompat).
    async fn mark_reminder_sent(
        &self,
        guild_id: &str,
        provider: Option<String>,
    ) -> Result<(), DomainError>;
}
