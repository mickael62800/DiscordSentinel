//! Service : consultation de la memoire du serveur (archives / actu).

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::influence::guild_settings::InfluenceSettings;
use crate::domain::entities::influence::archive::ArchiveEntry;
use crate::domain::errors::DomainError;
use crate::ports::inbound::influence::read_archives::ReadArchivesUseCase;
use crate::ports::outbound::influence::information_repository::ArchiveRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

pub struct ReadArchivesService {
    archives: Arc<dyn ArchiveRepository>,
    cfg_repo: Option<Arc<dyn BotConfigRepository>>,
}

impl ReadArchivesService {
    pub fn new(archives: Arc<dyn ArchiveRepository>) -> Self {
        Self {
            archives,
            cfg_repo: None,
        }
    }

    pub fn with_bot_config_repo(mut self, repo: Arc<dyn BotConfigRepository>) -> Self {
        self.cfg_repo = Some(repo);
        self
    }

    async fn feed_size(&self, guild_id: &str) -> i64 {
        match &self.cfg_repo {
            Some(repo) => InfluenceSettings::load(repo.as_ref(), guild_id).await.feed_size(),
            None => InfluenceSettings::default().feed_size(),
        }
    }
}

#[async_trait]
impl ReadArchivesUseCase for ReadArchivesService {
    async fn list(
        &self,
        guild_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<ArchiveEntry>, DomainError> {
        let n = match limit {
            Some(l) => l.clamp(1, 25),
            None => self.feed_size(guild_id).await,
        };
        self.archives.list_recent(guild_id, n).await
    }
}
