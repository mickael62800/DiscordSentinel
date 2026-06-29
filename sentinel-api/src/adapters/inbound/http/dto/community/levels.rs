use crate::ports::inbound::community::manage_levels::AddXpResult;
use crate::ports::inbound::community::manage_levels::SaveLevelConfigCommand;
use sentinel_core::domain::entities::community::level::xp_progress;
use sentinel_core::domain::entities::community::level::LevelConfig;
use sentinel_core::domain::entities::community::level::UserLevel;
use sentinel_core::domain::entities::system::discord_ids::GuildId;
use sentinel_core::domain::entities::system::discord_ids::UserId;
use serde::Deserialize;
use serde::Serialize;
// ── Request DTOs ──

#[derive(Debug, Deserialize)]
pub struct SaveLevelConfigDto {
    pub guild_id: GuildId,
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

fn default_xp_per_message() -> i32 {
    15
}
fn default_xp_per_voice_minute() -> i32 {
    5
}
fn default_xp_cooldown() -> i32 {
    60
}
fn default_level_up_message() -> String {
    "GG {user}, tu es maintenant niveau **{level}** !".to_string()
}
fn default_enabled() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct AddXpDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub amount: i64,
    /// "text" ou "voice" (defaut: "text")
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    "text".to_string()
}

/// Set la valeur exacte XP texte/voix d'un user (admin override).
/// Champs Option : non envoye = on ne touche pas a ce champ.
#[derive(Debug, Deserialize)]
pub struct SetUserXpDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub xp_text: Option<i64>,
    pub xp_voice: Option<i64>,
}

/// Reset XP d'un user (admin override).
/// `target` = "all" / "text" / "voice".
#[derive(Debug, Deserialize)]
pub struct ResetUserXpDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub target: String,
}

#[derive(Debug, Deserialize)]
pub struct LevelLeaderboardParams {
    pub limit: Option<i64>,
    /// "text", "voice" ou absent (= total)
    pub source: Option<String>,
}

// ── Response DTOs ──

#[derive(Debug, Serialize)]
pub struct LevelConfigDto {
    pub guild_id: GuildId,
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
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub xp: i64,
    pub level: i32,
    pub xp_current: i64,
    pub xp_needed: i64,
    pub xp_text: i64,
    pub level_text: i32,
    pub xp_text_current: i64,
    pub xp_text_needed: i64,
    pub xp_voice: i64,
    pub level_voice: i32,
    pub xp_voice_current: i64,
    pub xp_voice_needed: i64,
    pub last_xp_at: String,
}

#[derive(Debug, Serialize)]
pub struct AddXpResponseDto {
    pub user: UserLevelDto,
    pub leveled_up: bool,
    pub old_level: i32,
    pub old_level_global: i32,
    pub source: String,
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
        let (xp_text_current, xp_text_needed) = xp_progress(u.xp_text);
        let (xp_voice_current, xp_voice_needed) = xp_progress(u.xp_voice);
        Self {
            id: u.id.to_string(),
            guild_id: u.guild_id,
            user_id: u.user_id,
            username: u.username,
            xp: u.xp,
            level: u.level,
            xp_current,
            xp_needed,
            xp_text: u.xp_text,
            level_text: u.level_text,
            xp_text_current,
            xp_text_needed,
            xp_voice: u.xp_voice,
            level_voice: u.level_voice,
            xp_voice_current,
            xp_voice_needed,
            last_xp_at: u.last_xp_at.to_rfc3339(),
        }
    }
}

impl From<AddXpResult> for AddXpResponseDto {
    fn from(r: AddXpResult) -> Self {
        Self {
            user: UserLevelDto::from(r.user_level),
            leveled_up: r.leveled_up,
            old_level: r.old_level,
            old_level_global: r.old_level_global,
            source: r.source.as_str().to_string(),
        }
    }
}

#[cfg(test)]
#[path = "tests/levels.rs"]
mod tests;
