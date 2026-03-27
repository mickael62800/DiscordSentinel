use std::pin::Pin;
use std::future::Future;

use crate::domain::entities::{AuditLog, AutoRoleConfig, BotDefinition, DailyActivity, LevelConfig, LevelReward, BotGuildConfig, ConductConfig, ConductPointsLog, Guild, Infraction, LogEntry, ModerationActionRequest, ModerationActionResponse, ModerationRule, RolePanel, RolePanelDetail, SecurityEvent, ServerStats, Ticket, TicketDetail, TicketMessage, UpdateRuleParams, UserConductPoints, UserDossier, UserLevel, UserModerationHistory, VoiceChannel, VoiceChannelDetail, WatchedUser};
use crate::domain::ports::{AppAdapter, AuditLogRepository, DashboardChartsRepository, LevelRepository, RolePanelsRepository, BotConfigRepository, ConductRepository, GuildRepository, InfractionsRepository, LogsRepository, ModerationRepository, RulesRepository, SecurityRepository, StatsRepository, TicketsRepository, VoiceChannelRepository, WatchedUsersRepository};

pub struct MockAdapter;

impl MockAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl StatsRepository for MockAdapter {
    fn get_dashboard_stats(&self) -> Pin<Box<dyn Future<Output = Result<ServerStats, String>> + Send>> {
        Box::pin(async {
            Ok(ServerStats {
                total_servers: 12,
                total_users: 4850,
                messages_today: 23419,
                infractions_today: 17,
                bots_online: 3,
                bots_total: 4,
            })
        })
    }
}

impl GuildRepository for MockAdapter {
    fn get_guilds(&self) -> Pin<Box<dyn Future<Output = Result<Vec<Guild>, String>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }
}

impl BotConfigRepository for MockAdapter {
    fn get_definitions(&self) -> Pin<Box<dyn Future<Output = Result<Vec<BotDefinition>, String>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }
    fn get_guild_config(&self, _guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<BotGuildConfig>, String>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }
    fn set_config(&self, _guild_id: String, _bot_name: String, _key: String, _value: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        Box::pin(async { Ok(()) })
    }
    fn delete_config(&self, _guild_id: String, _bot_name: String, _key: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        Box::pin(async { Ok(()) })
    }
}

impl LogsRepository for MockAdapter {
    fn get_logs(&self, _guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<LogEntry>, String>> + Send>> {
        Box::pin(async {
            Ok(vec![
                LogEntry {
                    id: "1".into(),
                    timestamp: "2026-03-26 10:32:15".into(),
                    level: "warn".into(),
                    bot: "automod-bot".into(),
                    server: "Community FR".into(),
                    message: "Spam detected from user#1234 - 5 messages in 3s".into(),
                },
                LogEntry {
                    id: "2".into(),
                    timestamp: "2026-03-26 10:31:02".into(),
                    level: "info".into(),
                    bot: "moderation-bot".into(),
                    server: "Gaming Hub".into(),
                    message: "Mute applied to user#5678 for 30 minutes".into(),
                },
                LogEntry {
                    id: "3".into(),
                    timestamp: "2026-03-26 10:28:45".into(),
                    level: "error".into(),
                    bot: "security-bot".into(),
                    server: "Dev Server".into(),
                    message: "Raid detection triggered - 15 joins in 10s".into(),
                },
                LogEntry {
                    id: "4".into(),
                    timestamp: "2026-03-26 10:25:10".into(),
                    level: "info".into(),
                    bot: "ticket-bot".into(),
                    server: "Community FR".into(),
                    message: "Ticket #142 opened by user#9012".into(),
                },
            ])
        })
    }
}

impl InfractionsRepository for MockAdapter {
    fn get_infractions(&self, _guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<Infraction>, String>> + Send>> {
        Box::pin(async {
            Ok(vec![
                Infraction {
                    id: "1".into(),
                    user_id: "123456789".into(),
                    username: "ToxicUser#1234".into(),
                    server: "Community FR".into(),
                    infraction_type: "warn".into(),
                    reason: "Spam in #general".into(),
                    created_at: "2026-03-26 10:32:15".into(),
                    moderator: "automod-bot".into(),
                },
                Infraction {
                    id: "2".into(),
                    user_id: "987654321".into(),
                    username: "Raider#5678".into(),
                    server: "Gaming Hub".into(),
                    infraction_type: "ban".into(),
                    reason: "Raid attempt detected".into(),
                    created_at: "2026-03-26 09:15:00".into(),
                    moderator: "security-bot".into(),
                },
                Infraction {
                    id: "3".into(),
                    user_id: "456789123".into(),
                    username: "Spammer#9012".into(),
                    server: "Dev Server".into(),
                    infraction_type: "mute".into(),
                    reason: "Repeated link spam".into(),
                    created_at: "2026-03-26 08:45:30".into(),
                    moderator: "moderation-bot".into(),
                },
            ])
        })
    }
}

impl RulesRepository for MockAdapter {
    fn get_rules(&self, _guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<ModerationRule>, String>> + Send>> {
        Box::pin(async {
            Ok(vec![
                ModerationRule {
                    id: "1".into(),
                    name: "Anti-Spam".into(),
                    enabled: true,
                    rule_type: "rate_limit".into(),
                    action: "mute".into(),
                    description: "Mute users sending more than 5 messages in 3 seconds".into(),
                },
                ModerationRule {
                    id: "2".into(),
                    name: "Anti-Raid".into(),
                    enabled: true,
                    rule_type: "join_rate".into(),
                    action: "lockdown".into(),
                    description: "Lockdown server if more than 10 joins in 10 seconds".into(),
                },
                ModerationRule {
                    id: "3".into(),
                    name: "Link Filter".into(),
                    enabled: false,
                    rule_type: "content_filter".into(),
                    action: "delete".into(),
                    description: "Delete messages containing blacklisted links".into(),
                },
                ModerationRule {
                    id: "4".into(),
                    name: "Profanity Filter".into(),
                    enabled: true,
                    rule_type: "content_filter".into(),
                    action: "warn".into(),
                    description: "Warn users using profanity in messages".into(),
                },
            ])
        })
    }

    fn toggle_rule(&self, id: String, enabled: bool) -> Pin<Box<dyn Future<Output = Result<bool, String>> + Send>> {
        Box::pin(async move {
            println!("Mock: Rule {} toggled to {}", id, enabled);
            Ok(enabled)
        })
    }

    fn update_rule(&self, params: UpdateRuleParams) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        Box::pin(async move {
            println!("Mock: Rule updated — guild={} flag={} weight={} thresholds={}/{}/{}/{}",
                params.guild_id, params.flag_type, params.weight,
                params.threshold_warn, params.threshold_delete, params.threshold_mute, params.threshold_ban);
            Ok(())
        })
    }
}

impl TicketsRepository for MockAdapter {
    fn get_tickets(&self) -> Pin<Box<dyn Future<Output = Result<Vec<Ticket>, String>> + Send>> {
        Box::pin(async {
            Ok(vec![
                Ticket {
                    id: "1".into(),
                    title: "Cannot access #mod-logs channel".into(),
                    status: "open".into(),
                    priority: "high".into(),
                    author_id: "111222333".into(),
                    author_name: "Alice#0001".into(),
                    assigned_to: Some("ModeratorBob".into()),
                    server: "Community FR".into(),
                    category: "permissions".into(),
                    created_at: "2026-03-26 09:15:00".into(),
                    updated_at: "2026-03-26 10:30:00".into(),
                    messages_count: 4,
                },
                Ticket {
                    id: "2".into(),
                    title: "User harassing me in DMs after ban".into(),
                    status: "open".into(),
                    priority: "urgent".into(),
                    author_id: "444555666".into(),
                    author_name: "Charlie#0042".into(),
                    assigned_to: None,
                    server: "Gaming Hub".into(),
                    category: "report".into(),
                    created_at: "2026-03-26 08:45:00".into(),
                    updated_at: "2026-03-26 08:45:00".into(),
                    messages_count: 1,
                },
                Ticket {
                    id: "3".into(),
                    title: "Request to unban friend".into(),
                    status: "pending".into(),
                    priority: "low".into(),
                    author_id: "777888999".into(),
                    author_name: "Dave#1337".into(),
                    assigned_to: Some("ModeratorBob".into()),
                    server: "Community FR".into(),
                    category: "appeal".into(),
                    created_at: "2026-03-25 14:20:00".into(),
                    updated_at: "2026-03-26 09:00:00".into(),
                    messages_count: 6,
                },
                Ticket {
                    id: "4".into(),
                    title: "Bot not responding to commands".into(),
                    status: "closed".into(),
                    priority: "medium".into(),
                    author_id: "101112131".into(),
                    author_name: "Eve#7777".into(),
                    assigned_to: Some("AdminCarl".into()),
                    server: "Dev Server".into(),
                    category: "bug".into(),
                    created_at: "2026-03-24 16:00:00".into(),
                    updated_at: "2026-03-25 11:30:00".into(),
                    messages_count: 8,
                },
                Ticket {
                    id: "5".into(),
                    title: "Suggestion: add music bot".into(),
                    status: "open".into(),
                    priority: "low".into(),
                    author_id: "141516171".into(),
                    author_name: "Frank#2222".into(),
                    assigned_to: None,
                    server: "Gaming Hub".into(),
                    category: "suggestion".into(),
                    created_at: "2026-03-26 07:00:00".into(),
                    updated_at: "2026-03-26 07:00:00".into(),
                    messages_count: 1,
                },
            ])
        })
    }

    fn get_ticket_detail(&self, id: String) -> Pin<Box<dyn Future<Output = Result<TicketDetail, String>> + Send>> {
        Box::pin(async move {
            let ticket = Ticket {
                id: id.clone(),
                title: "Cannot access #mod-logs channel".into(),
                status: "open".into(),
                priority: "high".into(),
                author_id: "111222333".into(),
                author_name: "Alice#0001".into(),
                assigned_to: Some("ModeratorBob".into()),
                server: "Community FR".into(),
                category: "permissions".into(),
                created_at: "2026-03-26 09:15:00".into(),
                updated_at: "2026-03-26 10:30:00".into(),
                messages_count: 4,
            };

            let messages = vec![
                TicketMessage {
                    id: "m1".into(),
                    ticket_id: id.clone(),
                    author_name: "Alice#0001".into(),
                    author_role: "user".into(),
                    content: "Hi, I used to have access to #mod-logs but since yesterday I can't see the channel anymore. Can someone check my permissions?".into(),
                    created_at: "2026-03-26 09:15:00".into(),
                },
                TicketMessage {
                    id: "m2".into(),
                    ticket_id: id.clone(),
                    author_name: "ModeratorBob".into(),
                    author_role: "moderator".into(),
                    content: "Hi Alice, I'll check the role configuration. Which role do you have?".into(),
                    created_at: "2026-03-26 09:22:00".into(),
                },
                TicketMessage {
                    id: "m3".into(),
                    ticket_id: id.clone(),
                    author_name: "Alice#0001".into(),
                    author_role: "user".into(),
                    content: "I have the Trusted Member role. It worked fine before the server restructure.".into(),
                    created_at: "2026-03-26 09:30:00".into(),
                },
                TicketMessage {
                    id: "m4".into(),
                    ticket_id: id.clone(),
                    author_name: "ModeratorBob".into(),
                    author_role: "moderator".into(),
                    content: "Found it - the channel permission override was reset during restructure. I'll fix it now.".into(),
                    created_at: "2026-03-26 10:30:00".into(),
                },
            ];

            Ok(TicketDetail { ticket, messages })
        })
    }

    fn reply_ticket(&self, ticket_id: String, content: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        Box::pin(async move {
            println!("Mock: Reply to ticket {}: {}", ticket_id, content);
            Ok(())
        })
    }

    fn close_ticket(&self, id: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        Box::pin(async move {
            println!("Mock: Ticket {} closed", id);
            Ok(())
        })
    }

    fn assign_ticket(&self, id: String, assignee: String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        Box::pin(async move {
            println!("Mock: Ticket {} assigned to {}", id, assignee);
            Ok(())
        })
    }
}

impl SecurityRepository for MockAdapter {
    fn get_events(&self, _guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<SecurityEvent>, String>> + Send>> {
        Box::pin(async {
            Ok(vec![
                SecurityEvent {
                    id: "se-1".into(),
                    guild_id: "123456789".into(),
                    event_type: "raid_detected".into(),
                    severity: "critical".into(),
                    description: "Raid detected: 15 joins in 10 seconds".into(),
                    user_ids: vec!["111".into(), "222".into(), "333".into()],
                    created_at: "2026-03-26 10:28:45".into(),
                },
                SecurityEvent {
                    id: "se-2".into(),
                    guild_id: "123456789".into(),
                    event_type: "suspicious_account".into(),
                    severity: "medium".into(),
                    description: "New member account is only 2 hours old".into(),
                    user_ids: vec!["444".into()],
                    created_at: "2026-03-26 09:15:00".into(),
                },
            ])
        })
    }
}

impl ModerationRepository for MockAdapter {
    fn log_action(&self, action: ModerationActionRequest) -> Pin<Box<dyn Future<Output = Result<ModerationActionResponse, String>> + Send>> {
        Box::pin(async move {
            println!("Mock: Moderation action {} on {}", action.action_type, action.target_name);
            Ok(ModerationActionResponse {
                id: "mod-1".into(),
                action_type: action.action_type,
                target_name: action.target_name,
                reason: action.reason,
            })
        })
    }

    fn get_history(&self, _guild_id: String, _user_id: String) -> Pin<Box<dyn Future<Output = Result<UserModerationHistory, String>> + Send>> {
        Box::pin(async {
            Ok(UserModerationHistory {
                target_id: "111222333".into(),
                target_name: "ToxicUser#1234".into(),
                total_warns: 3,
                total_mutes: 1,
                total_bans: 0,
                actions: vec![
                    ModerationActionResponse {
                        id: "mod-1".into(),
                        action_type: "warn".into(),
                        target_name: "ToxicUser#1234".into(),
                        reason: "Spam in #general".into(),
                    },
                ],
            })
        })
    }
}

impl VoiceChannelRepository for MockAdapter {
    fn get_channels(&self, _guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<VoiceChannel>, String>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }
    fn get_channel_detail(&self, _channel_id: String) -> Pin<Box<dyn Future<Output = Result<VoiceChannelDetail, String>> + Send>> {
        Box::pin(async { Err("Not implemented in mock".into()) })
    }
}

impl ConductRepository for MockAdapter {
    fn get_config(&self, _guild_id: String) -> Pin<Box<dyn Future<Output = Result<ConductConfig, String>> + Send>> {
        Box::pin(async {
            Ok(ConductConfig {
                guild_id: String::new(),
                max_points: 12,
                regen_amount: 1,
                regen_interval: "weekly".into(),
                penalty_warn: 1,
                penalty_delete: 2,
                penalty_mute: 3,
                penalty_ban: 6,
            })
        })
    }
    fn get_leaderboard(&self, _guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<UserConductPoints>, String>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }
    fn get_points(&self, _guild_id: String, _user_id: String) -> Pin<Box<dyn Future<Output = Result<UserConductPoints, String>> + Send>> {
        Box::pin(async { Err("Not implemented in mock".into()) })
    }
    fn get_log(&self, _guild_id: String, _user_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<ConductPointsLog>, String>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }
}

impl DashboardChartsRepository for MockAdapter {
    fn get_activity_trend(&self, _guild_id: Option<String>, _days: Option<i32>) -> Pin<Box<dyn Future<Output = Result<Vec<DailyActivity>, String>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }
}

impl LevelRepository for MockAdapter {
    fn get_level_config(&self, _guild_id: String) -> Pin<Box<dyn Future<Output = Result<LevelConfig, String>> + Send>> {
        Box::pin(async {
            Ok(LevelConfig {
                guild_id: String::new(),
                xp_per_message: 15,
                xp_per_voice_minute: 5,
                xp_cooldown_secs: 60,
                level_up_channel_id: None,
                level_up_message: "GG {user}, tu es maintenant niveau **{level}** !".into(),
                excluded_channels: vec![],
                enabled: true,
            })
        })
    }
    fn get_level_leaderboard(&self, _guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<UserLevel>, String>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }
    fn get_level_rewards(&self, _guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<LevelReward>, String>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }
}

impl AuditLogRepository for MockAdapter {
    fn get_audit_logs(&self, _guild_id: Option<String>, _event_type: Option<String>, _limit: Option<i64>) -> Pin<Box<dyn Future<Output = Result<Vec<AuditLog>, String>> + Send>> {
        Box::pin(async {
            Ok(vec![
                AuditLog {
                    id: "al-1".into(),
                    guild_id: "111222333".into(),
                    event_type: "message_delete".into(),
                    actor_id: None,
                    actor_name: None,
                    target_id: Some("msg-123".into()),
                    target_name: None,
                    channel_id: Some("444555666".into()),
                    channel_name: Some("#general".into()),
                    details: serde_json::json!({}),
                    created_at: "2026-03-27 10:00:00".into(),
                },
                AuditLog {
                    id: "al-2".into(),
                    guild_id: "111222333".into(),
                    event_type: "member_join".into(),
                    actor_id: None,
                    actor_name: None,
                    target_id: Some("123456789".into()),
                    target_name: Some("NewUser#0001".into()),
                    channel_id: None,
                    channel_name: None,
                    details: serde_json::json!({"account_created_at": "2026-03-27"}),
                    created_at: "2026-03-27 09:30:00".into(),
                },
            ])
        })
    }
}

fn mock_toxic_user() -> WatchedUser {
    WatchedUser {
        user_id: "123456789".into(),
        username: "ToxicUser#1234".into(),
        guild_id: "111222333".into(),
        guild_name: "Community FR".into(),
        risk_level: "critical".into(),
        total_warns: 3,
        total_mutes: 1,
        total_bans: 1,
        conduct_points: Some(2),
        max_conduct_points: Some(12),
        last_incident_at: Some("2026-03-26 10:32:15".into()),
        security_events_count: 1,
        first_seen_at: "2026-03-20 08:00:00".into(),
    }
}

impl WatchedUsersRepository for MockAdapter {
    fn get_watched_users(&self, _guild_id: Option<String>) -> Pin<Box<dyn Future<Output = Result<Vec<WatchedUser>, String>> + Send>> {
        Box::pin(async {
            Ok(vec![
                mock_toxic_user(),
                WatchedUser {
                    user_id: "987654321".into(),
                    username: "Spammer#5678".into(),
                    guild_id: "111222333".into(),
                    guild_name: "Community FR".into(),
                    risk_level: "high".into(),
                    total_warns: 2,
                    total_mutes: 1,
                    total_bans: 0,
                    conduct_points: Some(6),
                    max_conduct_points: Some(12),
                    last_incident_at: Some("2026-03-25 14:20:00".into()),
                    security_events_count: 0,
                    first_seen_at: "2026-03-22 12:00:00".into(),
                },
            ])
        })
    }

    fn get_user_dossier(&self, _guild_id: String, _user_id: String) -> Pin<Box<dyn Future<Output = Result<UserDossier, String>> + Send>> {
        Box::pin(async {
            Ok(UserDossier {
                user: mock_toxic_user(),
                infractions: vec![
                    Infraction {
                        id: "1".into(),
                        user_id: "123456789".into(),
                        username: "ToxicUser#1234".into(),
                        server: "Community FR".into(),
                        infraction_type: "warn".into(),
                        reason: "Spam in #general".into(),
                        created_at: "2026-03-26 10:32:15".into(),
                        moderator: "automod-bot".into(),
                    },
                ],
                moderation_actions: vec![
                    ModerationActionResponse {
                        id: "mod-1".into(),
                        action_type: "warn".into(),
                        target_name: "ToxicUser#1234".into(),
                        reason: "Spam in #general".into(),
                    },
                ],
                security_events: vec![],
                conduct_log: vec![
                    ConductPointsLog {
                        id: "cl-1".into(),
                        delta: -1,
                        reason: "warn".into(),
                        points_before: 3,
                        points_after: 2,
                        created_at: "2026-03-26 10:32:15".into(),
                    },
                ],
            })
        })
    }
}

impl RolePanelsRepository for MockAdapter {
    fn get_panels(&self, _guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<RolePanel>, String>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }
    fn get_panel(&self, _panel_id: String) -> Pin<Box<dyn Future<Output = Result<RolePanelDetail, String>> + Send>> {
        Box::pin(async { Err("Not implemented in mock".into()) })
    }
    fn get_auto_roles(&self, _guild_id: String) -> Pin<Box<dyn Future<Output = Result<Vec<AutoRoleConfig>, String>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }
}

impl AppAdapter for MockAdapter {}
