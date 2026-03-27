use serde::{Deserialize, Serialize};

use crate::domain::entities::{xp_progress, LevelConfig, LevelReward, UserLevel};
use crate::ports::inbound::manage_levels::{AddXpResult, SaveLevelConfigCommand};

// ── Request DTOs ──

#[derive(Debug, Deserialize)]
pub struct SaveLevelConfigDto {
    pub guild_id: String,
    #[serde(default = "default_xp_per_message")]
    pub xp_per_message: i32,
    #[serde(default = "default_xp_per_voice_minute")]
    pub xp_per_voice_minute: i32,
    #[serde(default = "default_xp_cooldown")]
    pub xp_cooldown_secs: i32,
    pub level_up_channel_id: Option<String>,
    #[serde(default = "default_level_up_message")]
    pub level_up_message: String,
    #[serde(default)]
    pub excluded_channels: Vec<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_xp_per_message() -> i32 { 15 }
fn default_xp_per_voice_minute() -> i32 { 5 }
fn default_xp_cooldown() -> i32 { 60 }
fn default_level_up_message() -> String { "GG {user}, tu es maintenant niveau **{level}** !".to_string() }
fn default_enabled() -> bool { true }

#[derive(Debug, Deserialize)]
pub struct AddXpDto {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct SetRewardDto {
    pub guild_id: String,
    pub level: i32,
    pub role_id: String,
}

#[derive(Debug, Deserialize)]
pub struct LevelLeaderboardParams {
    pub limit: Option<i64>,
}

// ── Response DTOs ──

#[derive(Debug, Serialize)]
pub struct LevelConfigDto {
    pub guild_id: String,
    pub xp_per_message: i32,
    pub xp_per_voice_minute: i32,
    pub xp_cooldown_secs: i32,
    pub level_up_channel_id: Option<String>,
    pub level_up_message: String,
    pub excluded_channels: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct UserLevelDto {
    pub id: String,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub xp: i64,
    pub level: i32,
    pub xp_current: i64,
    pub xp_needed: i64,
    pub last_xp_at: String,
}

#[derive(Debug, Serialize)]
pub struct LevelRewardDto {
    pub id: String,
    pub guild_id: String,
    pub level: i32,
    pub role_id: String,
}

#[derive(Debug, Serialize)]
pub struct AddXpResponseDto {
    pub user: UserLevelDto,
    pub leveled_up: bool,
    pub old_level: i32,
    pub reward_role_id: Option<String>,
}

// ── From impls ──

impl From<SaveLevelConfigDto> for SaveLevelConfigCommand {
    fn from(dto: SaveLevelConfigDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            xp_per_message: dto.xp_per_message,
            xp_per_voice_minute: dto.xp_per_voice_minute,
            xp_cooldown_secs: dto.xp_cooldown_secs,
            level_up_channel_id: dto.level_up_channel_id,
            level_up_message: dto.level_up_message,
            excluded_channels: dto.excluded_channels,
            enabled: dto.enabled,
        }
    }
}

impl From<LevelConfig> for LevelConfigDto {
    fn from(c: LevelConfig) -> Self {
        Self {
            guild_id: c.guild_id,
            xp_per_message: c.xp_per_message,
            xp_per_voice_minute: c.xp_per_voice_minute,
            xp_cooldown_secs: c.xp_cooldown_secs,
            level_up_channel_id: c.level_up_channel_id,
            level_up_message: c.level_up_message,
            excluded_channels: c.excluded_channels,
            enabled: c.enabled,
        }
    }
}

impl From<UserLevel> for UserLevelDto {
    fn from(u: UserLevel) -> Self {
        let (xp_current, xp_needed) = xp_progress(u.xp);
        Self {
            id: u.id.to_string(),
            guild_id: u.guild_id,
            user_id: u.user_id,
            username: u.username,
            xp: u.xp,
            level: u.level,
            xp_current,
            xp_needed,
            last_xp_at: u.last_xp_at.to_rfc3339(),
        }
    }
}

impl From<LevelReward> for LevelRewardDto {
    fn from(r: LevelReward) -> Self {
        Self {
            id: r.id.to_string(),
            guild_id: r.guild_id,
            level: r.level,
            role_id: r.role_id,
        }
    }
}

impl From<AddXpResult> for AddXpResponseDto {
    fn from(r: AddXpResult) -> Self {
        Self {
            user: UserLevelDto::from(r.user_level),
            leveled_up: r.leveled_up,
            old_level: r.old_level,
            reward_role_id: r.reward_role_id,
        }
    }
}
