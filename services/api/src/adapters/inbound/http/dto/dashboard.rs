use serde::{Deserialize, Serialize};

use crate::domain::entities::{DashboardStats, Guild, Infraction, LogEntry, Rule};

// ── Stats DTO (format desktop) ──

#[derive(Debug, Serialize)]
pub struct DashboardStatsDto {
    pub total_servers: u32,
    pub total_users: u32,
    pub messages_today: u64,
    pub infractions_today: u32,
    pub bots_online: u32,
    pub bots_total: u32,
    pub workers_online: u32,
    pub workers_total: u32,
    pub postgres_online: bool,
    pub redis_online: bool,
}

impl From<DashboardStats> for DashboardStatsDto {
    fn from(s: DashboardStats) -> Self {
        Self {
            total_servers: s.total_servers,
            total_users: s.total_users,
            messages_today: s.messages_today,
            infractions_today: s.infractions_today,
            bots_online: s.bots_online,
            bots_total: s.bots_total,
            workers_online: s.workers_online,
            workers_total: s.workers_total,
            postgres_online: s.postgres_online,
            redis_online: s.redis_online,
        }
    }
}

// ── Log DTO (format desktop) ──

#[derive(Debug, Serialize)]
pub struct LogEntryDto {
    pub id: String,
    pub timestamp: String,
    pub level: String,
    pub bot: String,
    pub server: String,
    pub message: String,
    pub category: String,
    pub details: serde_json::Value,
}

impl From<LogEntry> for LogEntryDto {
    fn from(e: LogEntry) -> Self {
        Self {
            id: e.id.to_string(),
            timestamp: e.timestamp.to_rfc3339(),
            level: e.level,
            bot: e.bot,
            server: e.server,
            message: e.message,
            category: e.category,
            details: e.details,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateLogDto {
    pub level: Option<String>,
    pub bot: Option<String>,
    pub server: Option<String>,
    pub message: String,
    pub category: Option<String>,
    pub details: Option<serde_json::Value>,
}

// ── Infraction DTO (format desktop) ──
// Le desktop attend : id, user_id, username, server, infraction_type, reason, created_at, moderator

#[derive(Debug, Serialize)]
pub struct DashboardInfractionDto {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub server: String,
    pub infraction_type: String,
    pub reason: String,
    pub created_at: String,
    pub moderator: String,
}

impl From<Infraction> for DashboardInfractionDto {
    fn from(inf: Infraction) -> Self {
        Self {
            id: inf.id.to_string(),
            user_id: inf.user_id,
            username: inf.username,
            server: inf.guild_id,
            infraction_type: inf.action.as_str().to_string(),
            reason: inf.reason,
            created_at: inf.created_at.to_rfc3339(),
            moderator: "AutoMod".to_string(),
        }
    }
}

// ── Rule DTO (format desktop) ──
// Le desktop attend : id, name, enabled, rule_type, action, description

#[derive(Debug, Serialize)]
pub struct DashboardRuleDto {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub rule_type: String,
    pub action: String,
    pub description: String,
}

impl From<Rule> for DashboardRuleDto {
    fn from(rule: Rule) -> Self {
        let flag_label = match rule.flag_type.as_str() {
            "spam" => "Anti-Spam",
            "insult" => "Anti-Insulte",
            "link" => "Anti-Lien",
            "phishing" => "Anti-Hameconnage",
            "nsfw" => "Anti-NSFW",
            "illicit" => "Anti-Illicite",
            "anger" => "Detection colere",
            "rage" => "Detection rage",
            "threat" => "Detection menace",
            "harassment" => "Detection harcelement",
            other => other,
        };

        // Déterminer l'action principale basée sur les seuils
        let action = if rule.threshold_ban > 0.0 {
            "ban"
        } else if rule.threshold_mute > 0.0 {
            "mute"
        } else if rule.threshold_delete > 0.0 {
            "delete"
        } else {
            "warn"
        };

        let description = format!(
            "Règle {} pour le serveur {} (poids: {:.1})",
            flag_label, rule.guild_id, rule.weight
        );

        Self {
            id: rule.id.to_string(),
            name: format!("{} ({})", flag_label, rule.guild_id),
            enabled: rule.enabled,
            rule_type: rule.flag_type.as_str().to_string(),
            action: action.to_string(),
            description,
        }
    }
}

// ── Guild DTO ──

#[derive(Debug, Serialize)]
pub struct GuildDto {
    pub guild_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub member_count: i32,
}

impl From<Guild> for GuildDto {
    fn from(g: Guild) -> Self {
        Self {
            guild_id: g.guild_id,
            name: g.name,
            icon: g.icon,
            member_count: g.member_count,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterGuildDto {
    pub guild_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub member_count: Option<i32>,
}

// ── Filtre par guild ──

#[derive(Debug, Deserialize)]
pub struct GuildFilterParams {
    pub guild_id: Option<String>,
}
