use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::moderation::modstats::ModeratorBreakdown;
use crate::domain::entities::moderation::modstats::ModstatsTrendDay;
use crate::domain::errors::DomainError;
use crate::ports::inbound::moderation::read_modstats::ReadModstatsUseCase;
use crate::ports::outbound::audit::modstats_repository::ModstatsRepository;

/// Top 20 moderateurs (regle metier figee).
const TOP_LIMIT: i64 = 20;

pub struct ReadModstatsService {
    repo: Arc<dyn ModstatsRepository>,
}

impl ReadModstatsService {
    pub fn new(repo: Arc<dyn ModstatsRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ReadModstatsUseCase for ReadModstatsService {
    async fn modstats(&self, guild_id: &str, days: i32) -> Result<Vec<ModeratorBreakdown>, DomainError> {
        let days = days.clamp(1, 90);
        self.repo.breakdown(guild_id, days, TOP_LIMIT).await
    }

    async fn modstats_trend(&self, guild_id: &str, days: i32) -> Result<Vec<ModstatsTrendDay>, DomainError> {
        let days = days.clamp(1, 90);
        self.repo.daily_trend(guild_id, days).await
    }
}
