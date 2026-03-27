use std::future::Future;
use std::pin::Pin;

use reqwest::{Client, RequestBuilder, Response};

use crate::domain::entities::{AuditLog, AutoRoleConfig, BotDefinition, DailyActivity, LevelConfig, LevelReward, BotGuildConfig, ConductConfig, ConductPointsLog, Guild, Infraction, LogEntry, ModerationActionRequest, ModerationActionResponse, ModerationRule, RolePanel, RolePanelDetail, SecurityEvent, ServerStats, Ticket, TicketDetail, UpdateRuleParams, UserConductPoints, UserDossier, UserLevel, UserModerationHistory, VoiceChannel, VoiceChannelDetail, WatchedUser};
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
        let req = self.auth(self.client.get(format!("{}/api/guilds", self.base_url)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<Guild>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

// --- Bot Config ---

impl BotConfigRepository for ApiAdapter {
    fn get_definitions(&self) -> Pin<Box<dyn Future<Output = Result<Vec<BotDefinition>, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/bots/definitions", self.base_url)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<BotDefinition>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn get_guild_config(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<BotGuildConfig>, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/bots/config/{}", self.base_url, guild_id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<BotGuildConfig>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn set_config(&self, guild_id: String, bot_name: String, key: String, value: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        #[derive(serde::Serialize)]
        struct Payload { guild_id: String, bot_name: String, config_key: String, config_value: String }
        let req = self.auth(self.client.post(format!("{}/api/bots/config", self.base_url)))
            .json(&Payload { guild_id, bot_name, config_key: key, config_value: value });
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            check_response(resp).await?;
            Ok(())
        })
    }

    fn delete_config(&self, guild_id: String, bot_name: String, key: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        #[derive(serde::Serialize)]
        struct Payload { guild_id: String, bot_name: String, config_key: String }
        let req = self.auth(self.client.delete(format!("{}/api/bots/config", self.base_url)))
            .json(&Payload { guild_id, bot_name, config_key: key });
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            check_response(resp).await?;
            Ok(())
        })
    }
}

// --- Stats: GET /api/stats ---

impl StatsRepository for ApiAdapter {
    fn get_dashboard_stats(&self) -> Pin<Box<dyn Future<Output = Result<ServerStats, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/stats", self.base_url)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<ServerStats>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

// --- Logs: GET /api/logs?guild_id= ---

impl LogsRepository for ApiAdapter {
    fn get_logs(&self, guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<LogEntry>, String>> + Send>> {
        let url = self.url_with_guild("api/logs", &guild_id);
        let req = self.auth(self.client.get(url));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<LogEntry>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

// --- Infractions: GET /api/infractions?guild_id= ---

impl InfractionsRepository for ApiAdapter {
    fn get_infractions(&self, guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<Infraction>, String>> + Send>> {
        let url = self.url_with_guild("api/infractions", &guild_id);
        let req = self.auth(self.client.get(url));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<Infraction>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

// --- Rules: GET /api/rules?guild_id=, PATCH /api/rules/{id} ---

impl RulesRepository for ApiAdapter {
    fn get_rules(&self, guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<ModerationRule>, String>> + Send>> {
        let url = self.url_with_guild("api/rules", &guild_id);
        let req = self.auth(self.client.get(url));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<ModerationRule>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn toggle_rule(&self, id: String, enabled: bool) -> Pin<Box<dyn Future<Output = Result<bool, String>> + Send>> {
        #[derive(serde::Serialize)]
        struct Payload { enabled: bool }

        let req = self.auth(
            self.client.patch(format!("{}/api/rules/{}", self.base_url, id))
        ).json(&Payload { enabled });

        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            check_response(resp).await?;
            Ok(enabled)
        })
    }

    fn update_rule(&self, params: UpdateRuleParams) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        let req = self.auth(
            self.client.post(format!("{}/rules", self.base_url))
        ).json(&params);

        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            check_response(resp).await?;
            Ok(())
        })
    }
}

// --- Tickets ---

impl TicketsRepository for ApiAdapter {
    fn get_tickets(&self) -> Pin<Box<dyn Future<Output = Result<Vec<Ticket>, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/tickets", self.base_url)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<Ticket>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn get_ticket_detail(&self, id: String) -> Pin<Box<dyn Future<Output = Result<TicketDetail, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/tickets/{}", self.base_url, id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<TicketDetail>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn reply_ticket(&self, ticket_id: String, content: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        #[derive(serde::Serialize)]
        struct Payload { content: String }
        let req = self.auth(
            self.client.post(format!("{}/api/tickets/{}/messages", self.base_url, ticket_id))
        ).json(&Payload { content });
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            check_response(resp).await?;
            Ok(())
        })
    }

    fn close_ticket(&self, id: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        let req = self.auth(self.client.patch(format!("{}/api/tickets/{}/close", self.base_url, id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            check_response(resp).await?;
            Ok(())
        })
    }

    fn assign_ticket(&self, id: String, assignee: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        #[derive(serde::Serialize)]
        struct Payload { assignee: String }
        let req = self.auth(
            self.client.patch(format!("{}/api/tickets/{}/assign", self.base_url, id))
        ).json(&Payload { assignee });
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            check_response(resp).await?;
            Ok(())
        })
    }
}

// --- Security ---

impl SecurityRepository for ApiAdapter {
    fn get_events(&self, guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<SecurityEvent>, String>> + Send>> {
        let mut url = format!("{}/api/security/events", self.base_url);
        if let Some(gid) = guild_id {
            url = format!("{}?guild_id={}", url, gid);
        }
        let req = self.auth(self.client.get(url));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<SecurityEvent>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

// --- Moderation ---

impl ModerationRepository for ApiAdapter {
    fn log_action(&self, action: ModerationActionRequest) -> Pin<Box<dyn Future<Output = Result<ModerationActionResponse, String>> + Send>> {
        let req = self.auth(
            self.client.post(format!("{}/api/moderation/actions", self.base_url))
        ).json(&action);
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<ModerationActionResponse>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn get_history(&self, guild_id: String, user_id: String) -> Pin<Box<dyn Future<Output = Result<UserModerationHistory, String>> + Send>> {
        let req = self.auth(
            self.client.get(format!("{}/api/moderation/history/{}/{}", self.base_url, guild_id, user_id))
        );
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<UserModerationHistory>().await.map_err(|e| format!("Parse error: {}", e))
        })
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
        let req = self.auth(self.client.get(url));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<VoiceChannel>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn get_channel_detail(&self, channel_id: String) -> Pin<Box<dyn Future<Output = Result<VoiceChannelDetail, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/voice-channels/by-channel/{}", self.base_url, channel_id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<VoiceChannelDetail>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

impl ConductRepository for ApiAdapter {
    fn get_config(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<ConductConfig, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/conduct/config/{}", self.base_url, guild_id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<ConductConfig>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn get_leaderboard(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<UserConductPoints>, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/conduct/{}/leaderboard", self.base_url, guild_id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<UserConductPoints>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn get_points(&self, guild_id: String, user_id: String) -> Pin<Box<dyn Future<Output = Result<UserConductPoints, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/conduct/{}/{}", self.base_url, guild_id, user_id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<UserConductPoints>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn get_log(&self, guild_id: String, user_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<ConductPointsLog>, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/conduct/{}/{}/log", self.base_url, guild_id, user_id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<ConductPointsLog>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

// --- Dashboard Charts ---

impl DashboardChartsRepository for ApiAdapter {
    fn get_activity_trend(&self, guild_id: Option<String>, days: Option<i32>) -> Pin<Box<dyn Future<Output = Result<Vec<DailyActivity>, String>> + Send>> {
        let mut url = format!("{}/api/charts/activity", self.base_url);
        let mut params = Vec::new();
        if let Some(gid) = guild_id {
            params.push(format!("guild_id={gid}"));
        }
        if let Some(d) = days {
            params.push(format!("days={d}"));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }
        let req = self.auth(self.client.get(url));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<DailyActivity>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

// --- Levels ---

impl LevelRepository for ApiAdapter {
    fn get_level_config(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<LevelConfig, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/levels/config/{}", self.base_url, guild_id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<LevelConfig>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn get_level_leaderboard(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<UserLevel>, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/levels/{}/leaderboard", self.base_url, guild_id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<UserLevel>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn get_level_rewards(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<LevelReward>, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/levels/rewards/{}", self.base_url, guild_id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<LevelReward>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

// --- Audit Logs ---

impl AuditLogRepository for ApiAdapter {
    fn get_audit_logs(&self, guild_id: Option<String>, event_type: Option<String>, limit: Option<i64>) -> Pin<Box<dyn Future<Output = Result<Vec<AuditLog>, String>> + Send>> {
        let mut url = format!("{}/api/audit-logs", self.base_url);
        let mut params = Vec::new();
        if let Some(gid) = guild_id {
            params.push(format!("guild_id={gid}"));
        }
        if let Some(et) = event_type {
            params.push(format!("event_type={et}"));
        }
        if let Some(l) = limit {
            params.push(format!("limit={l}"));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }
        let req = self.auth(self.client.get(url));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<AuditLog>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

// --- Watched Users ---

impl WatchedUsersRepository for ApiAdapter {
    fn get_watched_users(&self, guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<WatchedUser>, String>> + Send>> {
        let url = self.url_with_guild("api/watched-users", &guild_id);
        let req = self.auth(self.client.get(url));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<WatchedUser>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }

    fn get_user_dossier(&self, guild_id: String, user_id: String) -> Pin<Box<dyn Future<Output = Result<UserDossier, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/watched-users/{}/{}", self.base_url, guild_id, user_id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<UserDossier>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

// --- Role Panels ---

impl RolePanelsRepository for ApiAdapter {
    fn get_panels(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<RolePanel>, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/role-panels/{}", self.base_url, guild_id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<RolePanel>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
    fn get_panel(&self, panel_id: String) -> Pin<Box<dyn Future<Output = Result<RolePanelDetail, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/role-panels/detail/{}", self.base_url, panel_id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<RolePanelDetail>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
    fn get_auto_roles(&self, guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<AutoRoleConfig>, String>> + Send>> {
        let req = self.auth(self.client.get(format!("{}/api/auto-roles/{}", self.base_url, guild_id)));
        Box::pin(async move {
            let resp = req.send().await.map_err(|e| format!("Connection failed: {}", e))?;
            let resp = check_response(resp).await?;
            resp.json::<Vec<AutoRoleConfig>>().await.map_err(|e| format!("Parse error: {}", e))
        })
    }
}

impl AppAdapter for ApiAdapter {}
