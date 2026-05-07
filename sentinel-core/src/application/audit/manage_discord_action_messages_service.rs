//! Implementation du use case `ManageDiscordActionMessagesUseCase`.
//!
//! Le service ne contient pas de regles metier — c'est un mapping pur
//! entre les ports inbound et outbound. La validation reste minimale
//! (kind non vide) et la logique business (quand register, quand
//! delete) appartient aux callers (bot, web, autres services).

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::audit::discord_action_message::DiscordActionMessage;
use crate::domain::entities::audit::discord_action_message::NewDiscordActionMessage;
use crate::domain::errors::DomainError;
use crate::ports::inbound::audit::manage_discord_action_messages::ManageDiscordActionMessagesUseCase;
use crate::ports::outbound::audit::discord_action_message_repository::DiscordActionMessageRepository;

pub struct ManageDiscordActionMessagesService {
    repo: Arc<dyn DiscordActionMessageRepository>,
}

impl ManageDiscordActionMessagesService {
    pub fn new(repo: Arc<dyn DiscordActionMessageRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageDiscordActionMessagesUseCase for ManageDiscordActionMessagesService {
    async fn register(
        &self,
        msg: NewDiscordActionMessage,
    ) -> Result<(), DomainError> {
        if msg.kind.trim().is_empty() {
            return Err(DomainError::ValidationError("kind requis".into()));
        }
        if msg.guild_id.trim().is_empty()
            || msg.channel_id.trim().is_empty()
            || msg.message_id.trim().is_empty()
        {
            return Err(DomainError::ValidationError(
                "guild_id, channel_id et message_id requis".into(),
            ));
        }
        self.repo.register(msg).await
    }

    async fn list_for_action(
        &self,
        action_id: Uuid,
    ) -> Result<Vec<DiscordActionMessage>, DomainError> {
        self.repo.list_for_action(action_id).await
    }

    async fn get(
        &self,
        action_id: Uuid,
        kind: &str,
    ) -> Result<Option<DiscordActionMessage>, DomainError> {
        self.repo.get(action_id, kind).await
    }

    async fn touch_edited(
        &self,
        action_id: Uuid,
        kind: &str,
    ) -> Result<(), DomainError> {
        self.repo.touch_edited(action_id, kind).await
    }

    async fn delete(
        &self,
        action_id: Uuid,
        kind: &str,
    ) -> Result<bool, DomainError> {
        self.repo.delete(action_id, kind).await
    }

    async fn find_by_message(
        &self,
        guild_id: &str,
        channel_id: &str,
        message_id: &str,
    ) -> Result<Option<DiscordActionMessage>, DomainError> {
        self.repo
            .find_by_message(guild_id, channel_id, message_id)
            .await
    }
}
