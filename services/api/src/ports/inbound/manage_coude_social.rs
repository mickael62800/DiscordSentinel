use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::entities::{
    CoudeCurrentSeason, CoudeEvent, CoudeLeaderboardEntry, LeaderboardCategory, NewDailyChaos,
};
use crate::domain::errors::DomainError;

/// Use case "fonctionnalités sociales Coup de Coude".
#[async_trait]
pub trait ManageCoudeSocialUseCase: Send + Sync {
    async fn check_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError>;

    async fn set_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
        duration_secs: i64,
    ) -> Result<(), DomainError>;

    async fn leaderboard(
        &self,
        guild_id: &str,
        category: LeaderboardCategory,
        limit: i64,
    ) -> Result<Vec<CoudeLeaderboardEntry>, DomainError>;

    async fn list_active_events(&self, guild_id: &str) -> Result<Vec<CoudeEvent>, DomainError>;

    async fn log_daily_chaos(&self, chaos: NewDailyChaos) -> Result<(), DomainError>;

    async fn current_season(&self, guild_id: &str) -> Result<CoudeCurrentSeason, DomainError>;
}
