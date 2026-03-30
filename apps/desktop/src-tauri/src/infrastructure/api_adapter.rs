use std::future::Future;
use std::pin::Pin;

use reqwest::{Client, RequestBuilder, Response};

use crate::domain::entities::{AuditLog, AutoRoleConfig, BotDefinition, ConfirmedBan, DailyActivity, TopUser, LevelConfig, LevelReward, BotGuildConfig, ConductConfig, ConductPointsLog, Guild, Infraction, LogEntry, ModerationActionRequest, ModerationActionResponse, ModerationRule, RolePanel, RolePanelDetail, SecurityEvent, ServerStats, Ticket, TicketDetail, UpdateRuleParams, UserConductPoints, UserDossier, UserLevel, UserModerationHistory, VoiceChannel, VoiceChannelDetail, WatchedUser};
use crate::domain::ports::{AppAdapter, AuditLogRepository, DashboardChartsRepository, LevelRepository, RolePanelsRepository, BotConfigRepository, ConductRepository, GuildRepository, InfractionsRepository, LogsRepository, ModerationRepository, RulesRepository, SecurityRepository, StatsRepository, TicketsRepository, VoiceChannelRepository, WatchedUsersRepository};

pub struct ApiAdapter {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiAdapter {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url,
            api_key,
        }
    }

    fn auth(&self, req: RequestBuilder) -> RequestBuilder {
        if self.api_key.is_empty() {
            req
        } else {
            req.bearer_auth(&self.api_key)
        }
    }

    fn url_with_guild(&self, path: &str, guild_id: &Option<String>) -> String {
        match guild_id {
            Some(gid) => format!("{}/{}?guild_id={}", self.base_url, path, gid),
            None => format!("{}/{}", self.base_url, path),
        }
    }

    /// GET → deserialize JSON. Factorise le pattern repete dans tous les endpoints.
    fn get_json<T: serde::de::DeserializeOwned + Send + 'static>(
        &self,
        req: RequestBuilder,
    ) -> Pin<Box<dyn Future<Output = Result<T, String>> + Send>> {
        let req = self.auth(req);
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<T>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    /// POST/PATCH/DELETE → ignore response body. Factorise le pattern fire-and-check.
    fn send_only(
        &self,
        req: RequestBuilder,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        let req = self.auth(req);
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            check_response(resp).await?;
            Ok(())
        })
    }
}

async fn check_response(resp: Response) -> Result<Response, String> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp)
    } else if status.as_u16() == 401 {
        Err("Unauthorized: invalid API key".into())
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(format!("API error {}: {}", status.as_u16(), body))
    }
}

// --- Guilds: GET /api/guilds ---

impl GuildRepository for ApiAdapter {
    fn get_guilds(&self) -> Pin<Box<dyn Future<Output = Result<Vec<Guild>, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/guilds", self.base_url)))
    }
}

// --- Bot Config ---

impl BotConfigRepository for ApiAdapter {
    fn get_definitions(&self) -> Pin<Box<dyn Future<Output = Result<Vec<BotDefinition>, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/bots/definitions", self.base_url)))
    }

    fn get_guild_config(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<BotGuildConfig>, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/bots/config/{}", self.base_url, guild_id)))
    }

    fn set_config(&self, guild_id: String, bot_name: String, key: String, value: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        #[derive(serde::Serialize)]
        struct Payload { guild_id: String, bot_name: String, config_key: String, config_value: String }
        self.send_only(self.client.post(format!("{}/api/bots/config", self.base_url))
            .json(&Payload { guild_id, bot_name, config_key: key, config_value: value }))
    }

    fn delete_config(&self, guild_id: String, bot_name: String, key: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        #[derive(serde::Serialize)]
        struct Payload { guild_id: String, bot_name: String, config_key: String }
        self.send_only(self.client.delete(format!("{}/api/bots/config", self.base_url))
            .json(&Payload { guild_id, bot_name, config_key: key }))
    }
}

// --- Stats: GET /api/stats ---

impl StatsRepository for ApiAdapter {
    fn get_dashboard_stats(&self) -> Pin<Box<dyn Future<Output = Result<ServerStats, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/stats", self.base_url)))
    }
}

// --- Logs: GET /api/logs?guild_id= ---

impl LogsRepository for ApiAdapter {
    fn get_logs(&self, guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<LogEntry>, String>> + Send>> {
        self.get_json(self.client.get(self.url_with_guild("api/logs", &guild_id)))
    }

    fn delete_logs_by_category(&self, category: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        self.send_only(self.client.delete(format!("{}/api/logs/{}", self.base_url, category)))
    }
}

// --- Infractions ---

impl InfractionsRepository for ApiAdapter {
    fn get_infractions(&self, guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<Infraction>, String>> + Send>> {
        self.get_json(self.client.get(self.url_with_guild("api/infractions", &guild_id)))
    }
}

// --- Rules ---

impl RulesRepository for ApiAdapter {
    fn get_rules(&self, guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<ModerationRule>, String>> + Send>> {
        self.get_json(self.client.get(self.url_with_guild("api/rules", &guild_id)))
    }

    fn toggle_rule(&self, id: String, enabled: bool) -> Pin<Box<dyn Future<Output = Result<bool, String>> + Send>> {
        #[derive(serde::Serialize)]
        struct Payload { enabled: bool }
        let req = self.auth(self.client.patch(format!("{}/api/rules/{}", self.base_url, id)))
            .json(&Payload { enabled });
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            check_response(resp).await?;
            Ok(enabled)
        })
    }

    fn update_rule(&self, params: UpdateRuleParams) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        self.send_only(self.client.post(format!("{}/rules", self.base_url)).json(&params))
    }
}

// --- Tickets ---

impl TicketsRepository for ApiAdapter {
    fn get_tickets(&self) -> Pin<Box<dyn Future<Output = Result<Vec<Ticket>, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/tickets", self.base_url)))
    }

    fn get_ticket_detail(&self, id: String) -> Pin<Box<dyn Future<Output = Result<TicketDetail, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/tickets/{}", self.base_url, id)))
    }

    fn reply_ticket(&self, ticket_id: String, content: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        #[derive(serde::Serialize)]
        struct Payload { content: String }
        self.send_only(self.client.post(format!("{}/api/tickets/{}/messages", self.base_url, ticket_id))
            .json(&Payload { content }))
    }

    fn close_ticket(&self, id: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        self.send_only(self.client.patch(format!("{}/api/tickets/{}/close", self.base_url, id)))
    }

    fn assign_ticket(&self, id: String, assignee: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        #[derive(serde::Serialize)]
        struct Payload { assignee: String }
        self.send_only(self.client.patch(format!("{}/api/tickets/{}/assign", self.base_url, id))
            .json(&Payload { assignee }))
    }
}

// --- Security ---

impl SecurityRepository for ApiAdapter {
    fn get_events(&self, guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<SecurityEvent>, String>> + Send>> {
        let url = self.url_with_guild("api/security/events", &guild_id);
        self.get_json(self.client.get(url))
    }
}

// --- Moderation ---

impl ModerationRepository for ApiAdapter {
    fn log_action(&self, action: ModerationActionRequest) -> Pin<Box<dyn Future<Output = Result<ModerationActionResponse, String>> + Send>> {
        self.get_json(self.client.post(format!("{}/api/moderation/actions", self.base_url)).json(&action))
    }

    fn get_history(&self, guild_id: String, user_id: String) -> Pin<Box<dyn Future<Output = Result<UserModerationHistory, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/moderation/history/{}/{}", self.base_url, guild_id, user_id)))
    }

    fn get_confirmed_bans(&self, guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<ConfirmedBan>, String>> + Send>> {
        self.get_json(self.client.get(self.url_with_guild("api/moderation/bans", &guild_id)))
    }

    fn execute_ban(&self, guild_id: String, user_id: String, reason: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        #[derive(serde::Serialize)]
        struct Payload { guild_id: String, user_id: String, reason: String }
        self.send_only(self.client.post(format!("{}/api/moderation/execute-ban", self.base_url))
            .json(&Payload { guild_id, user_id, reason }))
    }

    fn execute_unban(&self, guild_id: String, user_id: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        #[derive(serde::Serialize)]
        struct Payload { guild_id: String, user_id: String }
        self.send_only(self.client.post(format!("{}/api/moderation/execute-unban", self.base_url))
            .json(&Payload { guild_id, user_id }))
    }
}

// --- Voice Channels ---

impl VoiceChannelRepository for ApiAdapter {
    fn get_channels(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<VoiceChannel>, String>> + Send>> {
        let url = if guild_id.is_empty() {
            format!("{}/api/voice-channels/_all", self.base_url)
        } else {
            format!("{}/api/voice-channels/{}", self.base_url, guild_id)
        };
        self.get_json(self.client.get(url))
    }

    fn get_channel_detail(&self, channel_id: String) -> Pin<Box<dyn Future<Output = Result<VoiceChannelDetail, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/voice-channels/by-channel/{}", self.base_url, channel_id)))
    }
}

impl ConductRepository for ApiAdapter {
    fn get_config(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<ConductConfig, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/conduct/config/{}", self.base_url, guild_id)))
    }

    fn get_leaderboard(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<UserConductPoints>, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/conduct/{}/leaderboard", self.base_url, guild_id)))
    }

    fn get_points(&self, guild_id: String, user_id: String) -> Pin<Box<dyn Future<Output = Result<UserConductPoints, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/conduct/{}/{}", self.base_url, guild_id, user_id)))
    }

    fn get_log(&self, guild_id: String, user_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<ConductPointsLog>, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/conduct/{}/{}/log", self.base_url, guild_id, user_id)))
    }
}

// --- Dashboard Charts ---

impl DashboardChartsRepository for ApiAdapter {
    fn get_activity_trend(&self, guild_id: Option<String>, days: Option<i32>) -> Pin<Box<dyn Future<Output = Result<Vec<DailyActivity>, String>> + Send>> {
        let mut url = format!("{}/api/charts/activity", self.base_url);
        let mut params = Vec::new();
        if let Some(gid) = guild_id { params.push(format!("guild_id={gid}")); }
        if let Some(d) = days { params.push(format!("days={d}")); }
        if !params.is_empty() { url = format!("{}?{}", url, params.join("&")); }
        self.get_json(self.client.get(url))
    }

    fn get_top_users(&self, guild_id: String, limit: Option<u32>) -> Pin<Box<dyn Future<Output = Result<Vec<TopUser>, String>> + Send>> {
        let l = limit.unwrap_or(10);
        self.get_json(self.client.get(format!("{}/api/stats/{}/leaderboard?limit={}", self.base_url, guild_id, l)))
    }
}

// --- Levels ---

impl LevelRepository for ApiAdapter {
    fn get_level_config(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<LevelConfig, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/levels/config/{}", self.base_url, guild_id)))
    }

    fn get_level_leaderboard(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<UserLevel>, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/levels/{}/leaderboard", self.base_url, guild_id)))
    }

    fn get_level_rewards(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<LevelReward>, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/levels/rewards/{}", self.base_url, guild_id)))
    }
}

// --- Audit Logs ---

impl AuditLogRepository for ApiAdapter {
    fn get_audit_logs(&self, guild_id: Option<String>, event_type: Option<String>, limit: Option<i64>) -> Pin<Box<dyn Future<Output = Result<Vec<AuditLog>, String>> + Send>> {
        let mut url = format!("{}/api/audit-logs", self.base_url);
        let mut params = Vec::new();
        if let Some(gid) = guild_id { params.push(format!("guild_id={gid}")); }
        if let Some(et) = event_type { params.push(format!("event_type={et}")); }
        if let Some(l) = limit { params.push(format!("limit={l}")); }
        if !params.is_empty() { url = format!("{}?{}", url, params.join("&")); }
        self.get_json(self.client.get(url))
    }
}

// --- Watched Users ---

impl WatchedUsersRepository for ApiAdapter {
    fn get_watched_users(&self, guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<WatchedUser>, String>> + Send>> {
        self.get_json(self.client.get(self.url_with_guild("api/watched-users", &guild_id)))
    }

    fn get_user_dossier(&self, guild_id: String, user_id: String) -> Pin<Box<dyn Future<Output = Result<UserDossier, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/watched-users/{}/{}", self.base_url, guild_id, user_id)))
    }
}

// --- Role Panels ---

impl RolePanelsRepository for ApiAdapter {
    fn get_panels(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<RolePanel>, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/role-panels/{}", self.base_url, guild_id)))
    }

    fn get_panel(&self, panel_id: String) -> Pin<Box<dyn Future<Output = Result<RolePanelDetail, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/role-panels/detail/{}", self.base_url, panel_id)))
    }

    fn get_auto_roles(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<AutoRoleConfig>, String>> + Send>> {
        self.get_json(self.client.get(format!("{}/api/auto-roles/{}", self.base_url, guild_id)))
    }
}

impl AppAdapter for ApiAdapter {}
