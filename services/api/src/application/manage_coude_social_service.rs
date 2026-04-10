use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::entities::{
    CoudeCurrentSeason, CoudeEvent, CoudeLeaderboardEntry, LeaderboardCategory, NewDailyChaos,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_social::ManageCoudeSocialUseCase;
use crate::ports::outbound::CoudeSocialRepository;

pub struct ManageCoudeSocialService {
    repo: Arc<dyn CoudeSocialRepository>,
}

impl ManageCoudeSocialService {
    pub fn new(repo: Arc<dyn CoudeSocialRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageCoudeSocialUseCase for ManageCoudeSocialService {
    async fn check_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        self.repo.get_cooldown(guild_id, user_id, action).await
    }

    async fn set_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
        duration_secs: i64,
    ) -> Result<(), DomainError> {
        if duration_secs <= 0 {
            return Err(DomainError::ValidationError(
                "La duree doit etre positive".into(),
            ));
        }
        self.repo
            .set_cooldown(guild_id, user_id, action, duration_secs)
            .await
    }

    async fn leaderboard(
        &self,
        guild_id: &str,
        category: LeaderboardCategory,
        limit: i64,
    ) -> Result<Vec<CoudeLeaderboardEntry>, DomainError> {
        let limit = limit.clamp(1, 100);
        self.repo.leaderboard(guild_id, category, limit).await
    }

    async fn list_active_events(&self, guild_id: &str) -> Result<Vec<CoudeEvent>, DomainError> {
        self.repo.list_active_events(guild_id).await
    }

    async fn log_daily_chaos(&self, chaos: NewDailyChaos) -> Result<(), DomainError> {
        self.repo.log_daily_chaos(chaos).await
    }

    async fn current_season(&self, guild_id: &str) -> Result<CoudeCurrentSeason, DomainError> {
        self.repo.get_or_bootstrap_current_season(guild_id).await
    }
}
