use serde::Deserialize;
use serde::Serialize;
use crate::domain::entities::audit::dashboard_stats::DashboardStats;
use crate::domain::entities::system::guild::Guild;
use crate::domain::entities::moderation::infraction::Infraction;
use crate::domain::entities::system::log_entry::LogEntry;
use crate::domain::entities::system::rule::Rule;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::entities::system::discord_ids::GuildId;
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
    pub user_id: UserId,
    pub username: String,
    pub server: String,
    pub infraction_type: String,
    pub reason: String,
    pub created_at: String,
    pub moderator: String,
    /// "detection" = detection automod (propose, non appliquee)
    /// "action"    = sanction effectivement appliquee (bot ou moderateur)
    pub source: String,
    /// Duree en secondes (pour mute/timeout/ban temporaire). None sinon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
    /// Contenu original du message analyse (None pour les actions de moderation
    /// manuelles). Utilise par la vue debug "Historique d'analyse" cote web.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Score IA brut combine (regex + IA + tension). None pour les actions
    /// manuelles. Utilise par la vue debug "Historique d'analyse".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

impl From<Infraction> for DashboardInfractionDto {
    fn from(inf: Infraction) -> Self {
        Self {
            id: inf.id.to_string(),
            user_id: inf.user_id,
            username: inf.username,
            server: inf.guild_id.into(),
            infraction_type: inf.action.as_str().to_string(),
            reason: inf.reason,
            created_at: inf.created_at.to_rfc3339(),
            moderator: "AutoMod".to_string(),
            source: "detection".to_string(),
            duration: inf.duration,
            content: Some(inf.content),
            score: Some(inf.score),
        }
    }
}

impl From<crate::domain::entities::moderation::action::applied::ModerationAction> for DashboardInfractionDto {
    fn from(action: crate::domain::entities::moderation::action::applied::ModerationAction) -> Self {
        Self {
            id: action.id.to_string(),
            user_id: action.target_id.into(),
            username: action.target_name,
            server: action.guild_id.into(),
            infraction_type: action.action_type,
            reason: action.reason,
            created_at: action.created_at.to_rfc3339(),
            moderator: action.moderator_name,
            source: "action".to_string(),
            duration: action.duration,
            content: None,
            score: None,
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

#[derive(Debug, Serialize, serde::Deserialize)]
pub struct GuildDto {
    pub guild_id: GuildId,
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
    pub guild_id: GuildId,
    pub name: String,
    pub icon: Option<String>,
    pub member_count: Option<i32>,
    /// Discord user ID du proprietaire de la guild. Si fourni, l API
    /// l enregistre automatiquement comme `owner` RBAC (ON CONFLICT DO NOTHING
    /// pour ne pas ecraser un role deja defini).
    pub owner_id: Option<String>,
}

// ── Filtre par guild ──

#[derive(Debug, Deserialize)]
pub struct GuildFilterParams {
    pub guild_id: Option<String>,
}

#[cfg(test)]
#[path = "tests/dashboard.rs"]
mod tests;
