use serde::{Deserialize, Serialize};

use crate::domain::entities::{GuildStatsOverview, UserStats};
use crate::ports::inbound::manage_stats::{RecordMessagesCommand, RecordVoiceCommand};

// ── Request DTOs ──

#[derive(Debug, Deserialize)]
pub struct RecordMessagesDto {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub count: u64,
}

#[derive(Debug, Deserialize)]
pub struct RecordVoiceDto {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    pub limit: Option<u32>,
}

// ── Response DTOs ──

#[derive(Debug, Serialize)]
pub struct UserStatsDto {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub message_count: u64,
    pub voice_seconds: u64,
    pub voice_hours: f64,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct GuildOverviewDto {
    pub guild_id: String,
    pub total_messages: u64,
    pub total_voice_seconds: u64,
    pub total_voice_hours: f64,
    pub active_members: u64,
    pub total_infractions: u64,
    pub total_warns: u64,
    pub total_mutes: u64,
    pub total_bans: u64,
    pub top_members: Vec<UserStatsDto>,
}

// ── Conversions ──

impl From<RecordMessagesDto> for RecordMessagesCommand {
    fn from(dto: RecordMessagesDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            user_id: dto.user_id,
            username: dto.username,
            count: dto.count,
        }
    }
}

impl From<RecordVoiceDto> for RecordVoiceCommand {
    fn from(dto: RecordVoiceDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            user_id: dto.user_id,
            username: dto.username,
            seconds: dto.seconds,
        }
    }
}

impl From<UserStats> for UserStatsDto {
    fn from(s: UserStats) -> Self {
        Self {
            guild_id: s.guild_id,
            user_id: s.user_id,
            username: s.username,
            voice_hours: s.voice_seconds as f64 / 3600.0,
            message_count: s.message_count,
            voice_seconds: s.voice_seconds,
            updated_at: s.updated_at.to_rfc3339(),
        }
    }
}

impl From<GuildStatsOverview> for GuildOverviewDto {
    fn from(o: GuildStatsOverview) -> Self {
        Self {
            guild_id: o.guild_id,
            total_messages: o.total_messages,
            total_voice_seconds: o.total_voice_seconds,
            total_voice_hours: o.total_voice_seconds as f64 / 3600.0,
            active_members: o.active_members,
            total_infractions: o.total_infractions,
            total_warns: o.total_warns,
            total_mutes: o.total_mutes,
            total_bans: o.total_bans,
            top_members: o.top_members.into_iter().map(UserStatsDto::from).collect(),
        }
    }
}
