use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::{GuildStatsOverview, UserStats};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_stats::{ManageStatsUseCase, RecordMessagesCommand, RecordVoiceCommand};
use crate::ports::outbound::{CachePort, InfractionRepository, StatsRepository};
use crate::ports::inbound::InfractionFilters;

const OVERVIEW_TTL: u64 = 60; // 1 minute

pub struct ManageStatsService {
    stats_repo: Arc<dyn StatsRepository>,
    infraction_repo: Arc<dyn InfractionRepository>,
    cache: Arc<dyn CachePort>,
}

impl ManageStatsService {
    pub fn new(
        stats_repo: Arc<dyn StatsRepository>,
        infraction_repo: Arc<dyn InfractionRepository>,
        cache: Arc<dyn CachePort>,
    ) -> Self {
        Self { stats_repo, infraction_repo, cache }
    }
}

#[async_trait]
impl ManageStatsUseCase for ManageStatsService {
    async fn record_messages(&self, cmd: RecordMessagesCommand) -> Result<(), DomainError> {
        self.stats_repo
            .increment_messages(&cmd.guild_id, &cmd.user_id, &cmd.username, cmd.count)
            .await?;

        // Invalidate caches
        let overview_key = format!("stats:overview:{}", cmd.guild_id);
        self.cache.invalidate(&overview_key).await.ok();

        Ok(())
    }

    async fn record_voice(&self, cmd: RecordVoiceCommand) -> Result<(), DomainError> {
        self.stats_repo
            .add_voice_seconds(&cmd.guild_id, &cmd.user_id, &cmd.username, cmd.seconds)
            .await?;

        let overview_key = format!("stats:overview:{}", cmd.guild_id);
        self.cache.invalidate(&overview_key).await.ok();

        Ok(())
    }

    async fn get_user_stats(&self, guild_id: &str, user_id: &str) -> Result<Option<UserStats>, DomainError> {
        self.stats_repo.find_by_user(guild_id, user_id).await
    }

    async fn get_guild_overview(&self, guild_id: &str) -> Result<GuildStatsOverview, DomainError> {
        let cache_key = format!("stats:overview:{guild_id}");

        // Cache-first
        if let Some(json) = self.cache.get_json(&cache_key).await? {
            if let Ok(overview) = serde_json::from_str::<GuildStatsOverview>(&json) {
                return Ok(overview);
            }
        }

        // Fetch stats from DB
        let members = self.stats_repo.find_by_guild(guild_id, 100).await?;

        let total_messages: u64 = members.iter().map(|m| m.message_count).sum();
        let total_voice_seconds: u64 = members.iter().map(|m| m.voice_seconds).sum();
        let active_members = members.len() as u64;

        // Fetch infractions
        let filters = InfractionFilters {
            user_id: None,
            action: None,
            limit: 10000,
            offset: 0,
        };
        let infractions = self.infraction_repo.find_by_guild(guild_id, &filters).await.unwrap_or_default();

        let total_warns = infractions.iter().filter(|i| i.action.as_str() == "warn").count() as u64;
        let total_mutes = infractions.iter().filter(|i| i.action.as_str() == "mute").count() as u64;
        let total_bans = infractions.iter().filter(|i| i.action.as_str() == "ban").count() as u64;

        let top_members: Vec<UserStats> = members.into_iter().take(10).collect();

        let overview = GuildStatsOverview {
            guild_id: guild_id.to_string(),
            total_messages,
            total_voice_seconds,
            active_members,
            total_infractions: infractions.len() as u64,
            total_warns,
            total_mutes,
            total_bans,
            top_members,
        };

        // Populate cache
        if let Ok(json) = serde_json::to_string(&overview) {
            self.cache.set_json(&cache_key, &json, OVERVIEW_TTL).await.ok();
        }

        Ok(overview)
    }

    async fn get_leaderboard(&self, guild_id: &str, limit: u32) -> Result<Vec<UserStats>, DomainError> {
        self.stats_repo.find_by_guild(guild_id, limit).await
    }
}
