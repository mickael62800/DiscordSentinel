//! Use case du mapping `discord_action_messages` (cf.
//! SYNC_DISCORD_WEB_DESIGN.md). Permet aux adapters inbound (HTTP, gRPC)
//! d'enregistrer / lister / supprimer un mapping action_id <-> message
//! Discord SANS appeler directement le repo outbound.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::audit::discord_action_message::DiscordActionMessage;
use crate::domain::entities::audit::discord_action_message::NewDiscordActionMessage;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ManageDiscordActionMessagesUseCase: Send + Sync {
    /// Enregistre un mapping (idempotent). Le `kind` doit etre non vide.
    async fn register(
        &self,
        msg: NewDiscordActionMessage,
    ) -> Result<(), DomainError>;

    /// Liste tous les mappings d'une action (toutes les representations
    /// Discord d'une meme entite metier).
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

    /// Marque un mapping comme edite (pour audit / observability).
    async fn touch_edited(
        &self,
        action_id: Uuid,
        kind: &str,
    ) -> Result<(), DomainError>;

    /// Supprime un mapping. Retourne `false` si rien n'a ete supprime.
    async fn delete(
        &self,
        action_id: Uuid,
        kind: &str,
    ) -> Result<bool, DomainError>;

    /// Retrouve un mapping a partir d'un message Discord (utile au bot
    /// quand il recoit un MESSAGE_DELETE Discord).
    async fn find_by_message(
        &self,
        guild_id: &str,
        channel_id: &str,
        message_id: &str,
    ) -> Result<Option<DiscordActionMessage>, DomainError>;
}
