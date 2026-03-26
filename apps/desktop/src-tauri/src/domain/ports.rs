use std::future::Future;
use std::pin::Pin;

use super::entities::{
    Infraction, LogEntry, ModerationActionRequest, ModerationActionResponse, ModerationRule,
    SecurityEvent, ServerStats, Ticket, TicketDetail, UpdateRuleParams, UserModerationHistory,
};

type BoxFut<T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send>>;

pub trait StatsRepository: Send + Sync + 'static {
    fn get_dashboard_stats(&self) -> BoxFut<ServerStats>;
}

pub trait LogsRepository: Send + Sync + 'static {
    fn get_logs(&self) -> BoxFut<Vec<LogEntry>>;
}

pub trait InfractionsRepository: Send + Sync + 'static {
    fn get_infractions(&self) -> BoxFut<Vec<Infraction>>;
}

pub trait RulesRepository: Send + Sync + 'static {
    fn get_rules(&self) -> BoxFut<Vec<ModerationRule>>;
    fn toggle_rule(&self, id: String, enabled: bool) -> BoxFut<bool>;
    fn update_rule(&self, params: UpdateRuleParams) -> BoxFut<()>;
}

pub trait TicketsRepository: Send + Sync + 'static {
    fn get_tickets(&self) -> BoxFut<Vec<Ticket>>;
    fn get_ticket_detail(&self, id: String) -> BoxFut<TicketDetail>;
    fn reply_ticket(&self, ticket_id: String, content: String) -> BoxFut<()>;
    fn close_ticket(&self, id: String) -> BoxFut<()>;
    fn assign_ticket(&self, id: String, assignee: String) -> BoxFut<()>;
}

pub trait SecurityRepository: Send + Sync + 'static {
    fn get_events(&self, guild_id: Option<String>) -> BoxFut<Vec<SecurityEvent>>;
}

pub trait ModerationRepository: Send + Sync + 'static {
    fn log_action(&self, action: ModerationActionRequest) -> BoxFut<ModerationActionResponse>;
    fn get_history(&self, guild_id: String, user_id: String) -> BoxFut<UserModerationHistory>;
}

pub trait AppAdapter:
    StatsRepository
    + LogsRepository
    + InfractionsRepository
    + RulesRepository
    + TicketsRepository
    + SecurityRepository
    + ModerationRepository
{
}
