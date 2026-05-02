//! Helper qui agrege la config bot game-portal pour une guild en une struct.
//!
//! Lit les valeurs via `BotConfigRepository::get_config(guild_id, "game-portal")`
//! et applique les defaults documentes en migration 189. Centralise pour
//! eviter de dupliquer les defaults dans chaque use case.

use std::sync::Arc;

use crate::domain::entities::system::bot_config::BotGuildConfig;
use crate::domain::errors::DomainError;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;

#[derive(Debug, Clone)]
pub struct GamePortalConfig {
    pub enabled: bool,
    pub max_servers_per_guild: i32,
    pub max_memory_total_mb: i32,
    pub port_range_start: u16,
    pub port_range_end: u16,
    pub rcon_port_range_start: u16,
    pub rcon_port_range_end: u16,
    pub allowed_templates: Vec<String>,
    pub default_idle_shutdown_days: i32,
    pub docker_network_name: String,
    pub container_user: String,
    pub host_data_dir: String,
    pub auto_create_world_volume: bool,
    pub rcon_enabled: bool,
    pub log_channel_id: Option<String>,
    /// Active la suppression auto des images Docker non utilisees.
    pub auto_remove_unused_images: bool,
    /// Nombre de jours sans aucun serveur actif avant suppression de l'image.
    pub unused_image_grace_days: i32,
}

fn find<'a>(items: &'a [BotGuildConfig], key: &str) -> Option<&'a str> {
    items
        .iter()
        .find(|c| c.config_key == key)
        .map(|c| c.config_value.as_str())
}

fn parse_bool(s: Option<&str>, default: bool) -> bool {
    match s {
        Some("true") | Some("1") => true,
        Some("false") | Some("0") => false,
        _ => default,
    }
}

fn parse_i32(s: Option<&str>, default: i32) -> i32 {
    s.and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn parse_u16(s: Option<&str>, default: u16) -> u16 {
    s.and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn parse_string(s: Option<&str>, default: &str) -> String {
    s.unwrap_or(default).to_string()
}

fn parse_csv(s: Option<&str>, default: &str) -> Vec<String> {
    let raw = s.unwrap_or(default);
    raw.split(',')
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

pub async fn load_game_portal_config(
    bot_config: &Arc<dyn BotConfigRepository>,
    guild_id: &str,
) -> Result<GamePortalConfig, DomainError> {
    let entries = bot_config.get_config(guild_id, "game-portal").await?;
    Ok(GamePortalConfig {
        enabled: parse_bool(find(&entries, "enabled"), true),
        max_servers_per_guild: parse_i32(find(&entries, "max_servers_per_guild"), 5),
        max_memory_total_mb: parse_i32(find(&entries, "max_memory_total_mb"), 8192),
        port_range_start: parse_u16(find(&entries, "port_range_start"), 25500),
        port_range_end: parse_u16(find(&entries, "port_range_end"), 25599),
        rcon_port_range_start: parse_u16(find(&entries, "rcon_port_range_start"), 25700),
        rcon_port_range_end: parse_u16(find(&entries, "rcon_port_range_end"), 25799),
        allowed_templates: parse_csv(
            find(&entries, "allowed_templates"),
            "minecraft-vanilla,valheim,terraria,factorio,palworld,ark,7dtd",
        ),
        default_idle_shutdown_days: parse_i32(find(&entries, "default_idle_shutdown_days"), 7),
        docker_network_name: parse_string(
            find(&entries, "docker_network_name"),
            "sentinel-games",
        ),
        container_user: parse_string(find(&entries, "container_user"), "1000:1000"),
        host_data_dir: parse_string(find(&entries, "host_data_dir"), "/var/lib/sentinel/games"),
        auto_create_world_volume: parse_bool(find(&entries, "auto_create_world_volume"), true),
        rcon_enabled: parse_bool(find(&entries, "rcon_enabled"), true),
        log_channel_id: find(&entries, "log_channel_id").map(String::from),
        auto_remove_unused_images: parse_bool(find(&entries, "auto_remove_unused_images"), true),
        unused_image_grace_days: parse_i32(find(&entries, "unused_image_grace_days"), 7),
    })
}
