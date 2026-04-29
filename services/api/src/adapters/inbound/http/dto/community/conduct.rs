use serde::Deserialize;
use serde::Serialize;
use crate::domain::entities::community::conduct::ConductConfig;
use crate::domain::entities::community::conduct::ConductPointsLog;
use crate::domain::entities::community::conduct::UserConductPoints;
use crate::ports::inbound::community::manage_conduct::SaveConductConfigCommand;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::entities::system::discord_ids::GuildId;

// ── Request DTOs ──

#[derive(Debug, Deserialize)]
pub struct SaveConductConfigDto {
    pub guild_id: GuildId,
    #[serde(default = "default_max_points")]
    pub max_points: i32,
    #[serde(default = "default_regen_amount")]
    pub regen_amount: i32,
    #[serde(default = "default_regen_interval")]
    pub regen_interval: String,
    #[serde(default = "default_penalty_warn")]
    pub penalty_warn: i32,
    #[serde(default = "default_penalty_delete")]
    pub penalty_delete: i32,
    #[serde(default = "default_penalty_mute")]
    pub penalty_mute: i32,
    #[serde(default = "default_penalty_ban")]
    pub penalty_ban: i32,
}

fn default_max_points() -> i32 { 12 }
fn default_regen_amount() -> i32 { 1 }
fn default_regen_interval() -> String { "weekly".to_string() }
fn default_penalty_warn() -> i32 { 1 }
fn default_penalty_delete() -> i32 { 2 }
fn default_penalty_mute() -> i32 { 3 }
fn default_penalty_ban() -> i32 { 6 }

#[derive(Debug, Deserialize)]
pub struct AddPointsDto {
    pub amount: i32,
    pub reason: String,
}

// ── Response DTOs ──

#[derive(Debug, Serialize)]
pub struct ConductConfigDto {
    pub guild_id: GuildId,
    pub max_points: i32,
    pub regen_amount: i32,
    pub regen_interval: String,
    pub penalty_warn: i32,
    pub penalty_delete: i32,
    pub penalty_mute: i32,
    pub penalty_ban: i32,
}

#[derive(Debug, Serialize)]
pub struct UserConductPointsDto {
    pub id: String,
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub points: i32,
    pub last_regen_at: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ConductPointsLogDto {
    pub id: String,
    pub delta: i32,
    pub reason: String,
    pub points_before: i32,
    pub points_after: i32,
    pub created_at: String,
}

// ── From impls ──

impl From<SaveConductConfigDto> for SaveConductConfigCommand {
    fn from(dto: SaveConductConfigDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            max_points: dto.max_points,
            regen_amount: dto.regen_amount,
            regen_interval: dto.regen_interval,
            penalty_warn: dto.penalty_warn,
            penalty_delete: dto.penalty_delete,
            penalty_mute: dto.penalty_mute,
            penalty_ban: dto.penalty_ban,
        }
    }
}

impl From<ConductConfig> for ConductConfigDto {
    fn from(c: ConductConfig) -> Self {
        Self {
            guild_id: c.guild_id,
            max_points: c.max_points,
            regen_amount: c.regen_amount,
            regen_interval: c.regen_interval,
            penalty_warn: c.penalty_warn,
            penalty_delete: c.penalty_delete,
            penalty_mute: c.penalty_mute,
            penalty_ban: c.penalty_ban,
        }
    }
}

impl From<UserConductPoints> for UserConductPointsDto {
    fn from(p: UserConductPoints) -> Self {
        Self {
            id: p.id.to_string(),
            guild_id: p.guild_id,
            user_id: p.user_id,
            username: p.username,
            points: p.points,
            last_regen_at: p.last_regen_at.to_rfc3339(),
            created_at: p.created_at.to_rfc3339(),
        }
    }
}

impl From<ConductPointsLog> for ConductPointsLogDto {
    fn from(l: ConductPointsLog) -> Self {
        Self {
            id: l.id.to_string(),
            delta: l.delta,
            reason: l.reason,
            points_before: l.points_before,
            points_after: l.points_after,
            created_at: l.created_at.to_rfc3339(),
        }
    }
}

#[cfg(test)]
#[path = "tests/conduct.rs"]
mod tests;
