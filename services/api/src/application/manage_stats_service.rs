use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::{DashboardStats, GuildStatsOverview, UserStats};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_stats::{ManageStatsUseCase, RecordMessagesCommand, RecordVoiceCommand};
use crate::ports::outbound::{CachePort, InfractionRepository, StatsRepository};
use crate::ports::inbound::InfractionFilters;

const OVERVIEW_TTL: u64 = 60; // 1 minute

pub struct ManageStatsService {
    stats_repo: Arc<dyn StatsRepository>,
    infraction_repo: Arc<dyn InfractionRepository>,
    cache: Arc<dyn CachePort>,
    redis_client: redis::Client,
}

impl ManageStatsService {
    pub fn new(
        stats_repo: Arc<dyn StatsRepository>,
        infraction_repo: Arc<dyn InfractionRepository>,
        cache: Arc<dyn CachePort>,
        redis_client: redis::Client,
    ) -> Self {
        Self { stats_repo, infraction_repo, cache, redis_client }
    }

    async fn count_services(&self) -> (u32, u32, u32, u32) {
        if let Ok(mut conn) = self.redis_client.get_multiplexed_async_connection().await {
            use redis::AsyncCommands;
            let known: Vec<String> = conn.smembers("bots:known").await.unwrap_or_default();

            let mut bots_online = 0u32;
            let mut bots_total = 0u32;
            let mut workers_online = 0u32;
            let mut workers_total = 0u32;

            for name in &known {
                let is_worker = name.contains("worker");
                let exists: bool = conn.exists(format!("bot:online:{}", name)).await.unwrap_or(false);

                if is_worker {
                    workers_total += 1;
                    if exists { workers_online += 1; }
                } else {
                    bots_total += 1;
                    if exists { bots_online += 1; }
                }
            }

            (bots_online, bots_total, workers_online, workers_total)
        } else {
            (0, 0, 0, 0)
        }
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

    async fn get_dashboard_stats(&self) -> Result<DashboardStats, DomainError> {
        let total_servers = self.stats_repo.count_distinct_guilds().await.unwrap_or(0) as u32;
        let total_users = self.stats_repo.count_distinct_users().await.unwrap_or(0) as u32;
        let infractions_today = self.infraction_repo.count_today().await.unwrap_or(0) as u32;

        let (bots_online, bots_total, workers_online, workers_total) = self.count_services().await;

        // Check PostgreSQL
        let postgres_online = self.stats_repo.count_distinct_guilds().await.is_ok();

        // Check Redis
        let redis_online = self.redis_client
            .get_multiplexed_async_connection()
            .await
            .map(|mut conn| {
                tokio::spawn(async move {
                    let _: Result<String, _> = redis::AsyncCommands::get(&mut conn, "ping_test").await;
                });
                true
            })
            .unwrap_or(false);

        Ok(DashboardStats {
            total_servers,
            total_users,
            messages_today: 0,
            infractions_today,
            bots_online,
            bots_total,
            workers_online,
            workers_total,
            postgres_online,
            redis_online,
        })
    }
}
