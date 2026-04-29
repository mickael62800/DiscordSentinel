//! Service filet de securite (cf. COUPE_AMELIORATIONS 4.4).

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::coude::guild_settings::GuildSettings;
use crate::domain::entities::coude::safety_net::ActiveSafetyNet;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_safety_net::ManageCoudeSafetyNetUseCase;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::outbound::coude::safety_net_repository::SafetyNetRepository;
pub struct ManageCoudeSafetyNetService {
    repo: Arc<dyn SafetyNetRepository>,
    bot_config_repo: Option<Arc<dyn BotConfigRepository>>,
}

impl ManageCoudeSafetyNetService {
    pub fn new(repo: Arc<dyn SafetyNetRepository>) -> Self {
        Self { repo, bot_config_repo: None }
    }

    pub fn with_bot_config_repo(mut self, repo: Arc<dyn BotConfigRepository>) -> Self {
        self.bot_config_repo = Some(repo);
        self
    }
}

#[async_trait]
impl ManageCoudeSafetyNetUseCase for ManageCoudeSafetyNetService {
    async fn try_activate(
        &self,
        guild_id: &str,
        user_id: &str,
        current_balance: i64,
    ) -> Result<Option<ActiveSafetyNet>, DomainError> {
        let (trigger, duration) = match &self.bot_config_repo {
            Some(repo) => {
                let s = GuildSettings::load(&**repo, guild_id).await;
                (
                    s.get_i64("safety_net_trigger_coins", 50),
                    s.get_i64("safety_net_duration_hours", 72),
                )
            }
            None => (50, 72),
        };
        if current_balance >= trigger {
            return Ok(None);
        }
        // Skip si filet deja actif (pas de cumul).
        if self.repo.get_active(guild_id, user_id).await?.is_some() {
            return Ok(None);
        }
        let _id = self.repo.activate(guild_id, user_id, duration).await?;
        // Re-read pour avoir les timestamps definitifs.
        self.repo.get_active(guild_id, user_id).await
    }

    async fn get_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<ActiveSafetyNet>, DomainError> {
        self.repo.get_active(guild_id, user_id).await
    }

    async fn list_active(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ActiveSafetyNet>, DomainError> {
        self.repo.list_active(guild_id).await
    }
}

#[cfg(test)]
#[path = "tests/manage_safety_net.rs"]
mod tests;
