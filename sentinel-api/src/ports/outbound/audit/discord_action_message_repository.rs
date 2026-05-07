//! Port outbound pour le mapping `discord_action_messages` (migration 175).

use async_trait::async_trait;
use uuid::Uuid;

use sentinel_core::domain::entities::audit::discord_action_message::DiscordActionMessage;
use sentinel_core::domain::entities::audit::discord_action_message::NewDiscordActionMessage;
use sentinel_core::domain::errors::DomainError;

#[async_trait]
pub trait DiscordActionMessageRepository: Send + Sync {
    /// Enregistre la correspondance (idempotent : ON CONFLICT DO NOTHING
    /// sur la cle composite `(action_id, kind)`).
    async fn register(&self, msg: NewDiscordActionMessage) -> Result<(), DomainError>;

    /// Liste tous les mappings pour une `action_id`.
    async fn list_for_action(
        &self,
        action_id: Uuid,
    ) -> Result<Vec<DiscordActionMessage>, DomainError>;

    /// Recupere un mapping precis (action_id + kind).
    async fn get(
        &self,
        action_id: Uuid,
        kind: &str,
    ) -> Result<Option<DiscordActionMessage>, DomainError>;

    /// Marque le `last_edited_at` (optionnel — pour audit / observability).
    async fn touch_edited(
        &self,
        action_id: Uuid,
        kind: &str,
    ) -> Result<(), DomainError>;

    /// Supprime un mapping (ex. quand le message Discord est supprime
    /// manuellement ou que l'action est archivee).
    async fn delete(
        &self,
        action_id: Uuid,
        kind: &str,
    ) -> Result<bool, DomainError>;

    /// Retrouve un mapping a partir d'un `(guild_id, channel_id, message_id)`.
    /// Utile pour le bot quand il recoit un `MESSAGE_DELETE` Discord et
    /// veut nettoyer la table.
    async fn find_by_message(
        &self,
        guild_id: &str,
        channel_id: &str,
        message_id: &str,
    ) -> Result<Option<DiscordActionMessage>, DomainError>;
}
