use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::ports::inbound::game::manage_game_servers::GameServerDetail;
use crate::ports::outbound::game::container_runtime::ContainerStats;
use sentinel_core::domain::entities::game::server::{GameServer, GameServerStatus};

#[derive(Debug, Deserialize)]
pub struct CreateGameServerDto {
    pub template_slug: String,
    pub name: String,
    /// Memoire en Mo (sinon default du template).
    pub memory_mb: Option<i32>,
    pub owner_user_id: String,
    /// Overrides initiaux (key/value SCREAMING_SNAKE).
    #[serde(default)]
    pub config: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigDto {
    pub config: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct RconCommandDto {
    pub command: String,
}

#[derive(Debug, Serialize)]
pub struct GameServerDto {
    pub id: Uuid,
    pub guild_id: String,
    pub template_id: Uuid,
    pub name: String,
    pub status: String,
    pub host_port: Option<u16>,
    pub rcon_port: Option<u16>,
    pub allocated_memory_mb: i32,
    pub owner_user_id: String,
    pub last_active_at: Option<DateTime<Utc>>,
    pub last_player_count: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub stopped_at: Option<DateTime<Utc>>,
}

impl From<GameServer> for GameServerDto {
    fn from(s: GameServer) -> Self {
        Self {
            id: s.id,
            guild_id: s.guild_id,
            template_id: s.template_id,
            name: s.name,
            status: status_str(s.status).to_string(),
            host_port: s.host_port,
            rcon_port: s.rcon_port,
            allocated_memory_mb: s.allocated_memory_mb,
            owner_user_id: s.owner_user_id,
            last_active_at: s.last_active_at,
            last_player_count: s.last_player_count,
            last_error: s.last_error,
            created_at: s.created_at,
            started_at: s.started_at,
            stopped_at: s.stopped_at,
        }
    }
}

fn status_str(s: GameServerStatus) -> &'static str {
    s.as_str()
}

#[derive(Debug, Serialize)]
pub struct GameServerDetailDto {
    pub server: GameServerDto,
    pub config: HashMap<String, String>,
}

impl From<GameServerDetail> for GameServerDetailDto {
    fn from(d: GameServerDetail) -> Self {
        Self {
            server: GameServerDto::from(d.server),
            config: d.config,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GameServerStatsDto {
    pub cpu_percent: f64,
    pub memory_used_mb: u64,
    pub memory_limit_mb: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

impl From<ContainerStats> for GameServerStatsDto {
    fn from(s: ContainerStats) -> Self {
        Self {
            cpu_percent: s.cpu_percent,
            memory_used_mb: s.memory_used_bytes / (1024 * 1024),
            memory_limit_mb: s.memory_limit_bytes / (1024 * 1024),
            network_rx_bytes: s.network_rx_bytes,
            network_tx_bytes: s.network_tx_bytes,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RconCommandResponseDto {
    pub response: String,
}
