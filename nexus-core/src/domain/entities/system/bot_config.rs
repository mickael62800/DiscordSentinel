//! Config bot par guild (cle/valeur) — portee depuis sentinel-core.
//!
//! Version nexus autonome : les helpers de parsing (`parse_bool_str`,
//! `parse_enabled_flag`) sont inlines ici plutot que dans un module
//! `config_parsers` separe, sentinel-core restant la reference.

use crate::domain::entities::system::discord_ids::GuildId;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotGuildConfig {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub bot_name: String,
    pub config_key: String,
    pub config_value: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotDefinition {
    pub bot_name: String,
    pub display_name: String,
    pub description: String,
    pub config_schema: serde_json::Value,
}

/// "true"/"1"/"yes" (insensible a la casse) => true, tout le reste => false.
pub fn parse_bool_str(v: &str) -> bool {
    matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes")
}

/// Flag `enabled` : absent = active (comportement inclusif).
pub fn parse_enabled_flag(v: Option<&str>) -> bool {
    v.map(parse_bool_str).unwrap_or(true)
}

/// Valeur brute d'une cle de config, si presente.
pub fn cfg_str<'a>(entries: &'a [BotGuildConfig], key: &str) -> Option<&'a str> {
    entries
        .iter()
        .find(|e| e.config_key == key)
        .map(|e| e.config_value.as_str())
}

/// Flag booleen : present => `parse_bool_str`, absent => `default`.
pub fn cfg_bool(entries: &[BotGuildConfig], key: &str, default: bool) -> bool {
    cfg_str(entries, key).map(parse_bool_str).unwrap_or(default)
}

/// Entier i64 : cle absente ou non numerique => `default`.
pub fn cfg_i64(entries: &[BotGuildConfig], key: &str, default: i64) -> i64 {
    cfg_str(entries, key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Entier u64 : cle absente ou non numerique => `default`.
pub fn cfg_u64(entries: &[BotGuildConfig], key: &str, default: u64) -> u64 {
    cfg_str(entries, key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Flag `enabled` du module : absent = active.
pub fn cfg_enabled(entries: &[BotGuildConfig]) -> bool {
    parse_enabled_flag(cfg_str(entries, "enabled"))
}
