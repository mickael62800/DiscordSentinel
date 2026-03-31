use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{Ticket, TicketDetail, TicketMessage};
use crate::domain::errors::DomainError;
use crate::ports::inbound::{
    AssignTicketCommand, CreateTicketCommand, ManageTicketsUseCase, ReplyTicketCommand,
    UpdateTicketChannelCommand,
};
use crate::ports::outbound::{CachePort, TicketRepository};

const TICKETS_LIST_TTL: u64 = 60; // 1 minute
const TICKET_DETAIL_TTL: u64 = 120; // 2 minutes

pub struct ManageTicketsService {
    ticket_repo: Arc<dyn TicketRepository>,
    cache: Arc<dyn CachePort>,
}

impl ManageTicketsService {
    pub fn new(ticket_repo: Arc<dyn TicketRepository>, cache: Arc<dyn CachePort>) -> Self {
        Self { ticket_repo, cache }
    }

    async fn invalidate_tickets_cache(&self) {
        self.cache.invalidate("tickets:all").await.ok();
        self.cache.invalidate_pattern("ticket:*").await.ok();
    }
}

#[async_trait]
impl ManageTicketsUseCase for ManageTicketsService {
    async fn list_tickets(&self, status: Option<String>, priority: Option<String>, search: Option<String>, author_id: Option<String>) -> Result<Vec<Ticket>, DomainError> {
        let has_filters = status.is_some() || priority.is_some() || search.is_some() || author_id.is_some();

        // Cache-first uniquement si pas de filtres
        if !has_filters {
            if let Some(json) = self.cache.get_json("tickets:all").await? {
                if let Ok(tickets) = serde_json::from_str::<Vec<Ticket>>(&json) {
                    return Ok(tickets);
                }
            }
        }

        let tickets = self.ticket_repo.find_all(
            status.as_deref(),
            priority.as_deref(),
            search.as_deref(),
            author_id.as_deref(),
        ).await?;

        // Populate cache uniquement si pas de filtres
        if !has_filters {
            if let Ok(json) = serde_json::to_string(&tickets) {
                self.cache.set_json("tickets:all", &json, TICKETS_LIST_TTL).await.ok();
            }
        }

        Ok(tickets)
    }

    async fn get_ticket_detail(&self, id: &str) -> Result<TicketDetail, DomainError> {
        let cache_key = format!("ticket:{id}");

        // Cache-first
        if let Some(json) = self.cache.get_json(&cache_key).await? {
            if let Ok(detail) = serde_json::from_str::<TicketDetail>(&json) {
                return Ok(detail);
            }
        }

        let uuid = id
            .parse::<Uuid>()
            .map_err(|_| DomainError::InvalidRule(format!("ID ticket invalide : {id}")))?;

        let ticket = self
            .ticket_repo
            .find_by_id(uuid)
            .await?
            .ok_or(DomainError::Internal(format!("Ticket introuvable : {id}")))?;

        let messages = self.ticket_repo.find_messages(uuid).await?;
        let detail = TicketDetail { ticket, messages };

        // Populate cache
        if let Ok(json) = serde_json::to_string(&detail) {
            self.cache.set_json(&cache_key, &json, TICKET_DETAIL_TTL).await.ok();
        }

        Ok(detail)
    }

    async fn create_ticket(&self, cmd: CreateTicketCommand) -> Result<Ticket, DomainError> {
        let now = chrono::Utc::now();
        let ticket = Ticket {
            id: Uuid::new_v4(),
            title: cmd.title,
            status: "open".to_string(),
            priority: cmd.priority,
            author_id: cmd.author_id,
            author_name: cmd.author_name,
            assigned_to: None,
            server: cmd.server,
            category: cmd.category,
            ticket_type: cmd.ticket_type,
            channel_id: cmd.channel_id,
            voice_channel_id: None,
            invited_user_id: None,
            created_at: now,
            updated_at: now,
            messages_count: 0,
        };

        self.ticket_repo.save(&ticket).await?;
        self.invalidate_tickets_cache().await;

        Ok(ticket)
    }

    async fn reply_ticket(&self, cmd: ReplyTicketCommand) -> Result<(), DomainError> {
        let ticket_id = cmd
            .ticket_id
            .parse::<Uuid>()
            .map_err(|_| DomainError::InvalidRule(format!("ID ticket invalide : {}", cmd.ticket_id)))?;

        let message = TicketMessage {
            id: Uuid::new_v4(),
            ticket_id,
            author_name: cmd.author_name,
            author_role: cmd.author_role,
            content: cmd.content,
            created_at: chrono::Utc::now(),
        };

        self.ticket_repo.save_message(&message).await?;
        self.ticket_repo.update_status(ticket_id, "pending").await.ok();
        self.invalidate_tickets_cache().await;

        Ok(())
    }

    async fn close_ticket(&self, id: &str) -> Result<(), DomainError> {
        let uuid = id
            .parse::<Uuid>()
            .map_err(|_| DomainError::InvalidRule(format!("ID ticket invalide : {id}")))?;

        self.ticket_repo.update_status(uuid, "closed").await?;
        self.invalidate_tickets_cache().await;

        Ok(())
    }

    async fn update_status(&self, id: &str, status: &str) -> Result<(), DomainError> {
        let uuid = id
            .parse::<Uuid>()
            .map_err(|_| DomainError::InvalidRule(format!("ID ticket invalide : {id}")))?;

        self.ticket_repo.update_status(uuid, status).await?;
        self.invalidate_tickets_cache().await;

        Ok(())
    }

    async fn assign_ticket(&self, cmd: AssignTicketCommand) -> Result<(), DomainError> {
        let uuid = cmd
            .ticket_id
            .parse::<Uuid>()
            .map_err(|_| DomainError::InvalidRule(format!("ID ticket invalide : {}", cmd.ticket_id)))?;

        self.ticket_repo.update_assignee(uuid, &cmd.assignee).await?;
        self.invalidate_tickets_cache().await;

        Ok(())
    }

    async fn update_ticket_channel(&self, cmd: UpdateTicketChannelCommand) -> Result<(), DomainError> {
        let uuid = cmd
            .ticket_id
            .parse::<Uuid>()
            .map_err(|_| DomainError::InvalidRule(format!("ID ticket invalide : {}", cmd.ticket_id)))?;

        if let Some(ref vc_id) = cmd.voice_channel_id {
            self.ticket_repo.update_voice_channel(uuid, Some(vc_id)).await?;
        }
        if let Some(ref inv_id) = cmd.invited_user_id {
            self.ticket_repo.update_invited_user(uuid, Some(inv_id)).await?;
        }
        self.invalidate_tickets_cache().await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Rule;
    use std::sync::Mutex;

    // ── Mock TicketRepository ──

    #[derive(Default)]
    struct MockTicketRepo {
        tickets: Mutex<Vec<Ticket>>,
        messages: Mutex<Vec<TicketMessage>>,
        last_status: Mutex<Option<(Uuid, String)>>,
        last_assignee: Mutex<Option<(Uuid, String)>>,
        last_voice_channel: Mutex<Option<(Uuid, Option<String>)>>,
        last_invited_user: Mutex<Option<(Uuid, Option<String>)>>,
    }

    #[async_trait]
    impl TicketRepository for MockTicketRepo {
        async fn find_all(&self, status: Option<&str>, _priority: Option<&str>, _search: Option<&str>, _author_id: Option<&str>) -> Result<Vec<Ticket>, DomainError> {
            let tickets = self.tickets.lock().unwrap();
            Ok(tickets.iter().filter(|t| {
                status.map_or(true, |s| t.status == s)
            }).cloned().collect())
        }

        async fn find_by_id(&self, id: Uuid) -> Result<Option<Ticket>, DomainError> {
            let tickets = self.tickets.lock().unwrap();
            Ok(tickets.iter().find(|t| t.id == id).cloned())
        }

        async fn save(&self, ticket: &Ticket) -> Result<(), DomainError> {
            self.tickets.lock().unwrap().push(ticket.clone());
            Ok(())
        }

        async fn update_status(&self, id: Uuid, status: &str) -> Result<(), DomainError> {
            *self.last_status.lock().unwrap() = Some((id, status.to_string()));
            let mut tickets = self.tickets.lock().unwrap();
            if let Some(t) = tickets.iter_mut().find(|t| t.id == id) {
                t.status = status.to_string();
            }
            Ok(())
        }

        async fn update_assignee(&self, id: Uuid, assignee: &str) -> Result<(), DomainError> {
            *self.last_assignee.lock().unwrap() = Some((id, assignee.to_string()));
            Ok(())
        }

        async fn find_messages(&self, ticket_id: Uuid) -> Result<Vec<TicketMessage>, DomainError> {
            let msgs = self.messages.lock().unwrap();
            Ok(msgs.iter().filter(|m| m.ticket_id == ticket_id).cloned().collect())
        }

        async fn save_message(&self, message: &TicketMessage) -> Result<(), DomainError> {
            self.messages.lock().unwrap().push(message.clone());
            Ok(())
        }

        async fn update_voice_channel(&self, id: Uuid, vc_id: Option<&str>) -> Result<(), DomainError> {
            *self.last_voice_channel.lock().unwrap() = Some((id, vc_id.map(|s| s.to_string())));
            Ok(())
        }

        async fn update_invited_user(&self, id: Uuid, inv_id: Option<&str>) -> Result<(), DomainError> {
            *self.last_invited_user.lock().unwrap() = Some((id, inv_id.map(|s| s.to_string())));
            Ok(())
        }
    }

    // ── Mock CachePort ──

    #[derive(Default)]
    struct MockCache;

    #[async_trait]
    impl CachePort for MockCache {
        async fn get_rules(&self, _: &str) -> Result<Option<Vec<Rule>>, DomainError> { Ok(None) }
        async fn set_rules(&self, _: &str, _: &[Rule]) -> Result<(), DomainError> { Ok(()) }
        async fn invalidate_rules(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn get_json(&self, _: &str) -> Result<Option<String>, DomainError> { Ok(None) }
        async fn set_json(&self, _: &str, _: &str, _: u64) -> Result<(), DomainError> { Ok(()) }
        async fn invalidate(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
        async fn invalidate_pattern(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
    }

    fn make_service() -> (ManageTicketsService, Arc<MockTicketRepo>) {
        let repo = Arc::new(MockTicketRepo::default());
        let cache = Arc::new(MockCache::default());
        let service = ManageTicketsService::new(repo.clone(), cache);
        (service, repo)
    }

    fn make_create_cmd() -> CreateTicketCommand {
        CreateTicketCommand {
            title: "Test ticket".to_string(),
            priority: "medium".to_string(),
            author_id: "123".to_string(),
            author_name: "testuser".to_string(),
            server: "TestServer".to_string(),
            category: "question".to_string(),
            ticket_type: "question".to_string(),
            channel_id: Some("999".to_string()),
        }
    }

    // ── Tests ──

    #[tokio::test]
    async fn test_create_ticket_returns_open_status() {
        let (service, repo) = make_service();
        let ticket = service.create_ticket(make_create_cmd()).await.unwrap();

        assert_eq!(ticket.status, "open");
        assert_eq!(ticket.title, "Test ticket");
        assert_eq!(ticket.priority, "medium");
        assert_eq!(ticket.author_id, "123");
        assert_eq!(ticket.messages_count, 0);
        assert!(ticket.assigned_to.is_none());
        assert!(ticket.voice_channel_id.is_none());

        let saved = repo.tickets.lock().unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].id, ticket.id);
    }

    #[tokio::test]
    async fn test_list_tickets_no_filters() {
        let (service, _repo) = make_service();
        service.create_ticket(make_create_cmd()).await.unwrap();

        let tickets = service.list_tickets(None, None, None, None).await.unwrap();
        assert_eq!(tickets.len(), 1);
    }

    #[tokio::test]
    async fn test_list_tickets_filter_by_status() {
        let (service, _repo) = make_service();
        service.create_ticket(make_create_cmd()).await.unwrap();

        let open = service.list_tickets(Some("open".to_string()), None, None, None).await.unwrap();
        assert_eq!(open.len(), 1);

        let closed = service.list_tickets(Some("closed".to_string()), None, None, None).await.unwrap();
        assert_eq!(closed.len(), 0);
    }

    #[tokio::test]
    async fn test_close_ticket() {
        let (service, repo) = make_service();
        let ticket = service.create_ticket(make_create_cmd()).await.unwrap();
        let id = ticket.id.to_string();

        service.close_ticket(&id).await.unwrap();

        let (uuid, status) = repo.last_status.lock().unwrap().clone().unwrap();
        assert_eq!(uuid, ticket.id);
        assert_eq!(status, "closed");
    }

    #[tokio::test]
    async fn test_close_ticket_invalid_uuid() {
        let (service, _) = make_service();
        let result = service.close_ticket("not-a-uuid").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_status() {
        let (service, repo) = make_service();
        let ticket = service.create_ticket(make_create_cmd()).await.unwrap();
        let id = ticket.id.to_string();

        service.update_status(&id, "pending").await.unwrap();

        let (uuid, status) = repo.last_status.lock().unwrap().clone().unwrap();
        assert_eq!(uuid, ticket.id);
        assert_eq!(status, "pending");
    }

    #[tokio::test]
    async fn test_reply_ticket_sets_pending() {
        let (service, repo) = make_service();
        let ticket = service.create_ticket(make_create_cmd()).await.unwrap();

        service.reply_ticket(ReplyTicketCommand {
            ticket_id: ticket.id.to_string(),
            content: "Hello".to_string(),
            author_name: "user".to_string(),
            author_role: "user".to_string(),
        }).await.unwrap();

        let msgs = repo.messages.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "Hello");

        let (_, status) = repo.last_status.lock().unwrap().clone().unwrap();
        assert_eq!(status, "pending");
    }

    #[tokio::test]
    async fn test_assign_ticket() {
        let (service, repo) = make_service();
        let ticket = service.create_ticket(make_create_cmd()).await.unwrap();

        service.assign_ticket(AssignTicketCommand {
            ticket_id: ticket.id.to_string(),
            assignee: "modo123".to_string(),
        }).await.unwrap();

        let (uuid, assignee) = repo.last_assignee.lock().unwrap().clone().unwrap();
        assert_eq!(uuid, ticket.id);
        assert_eq!(assignee, "modo123");
    }

    #[tokio::test]
    async fn test_update_ticket_channel_voice() {
        let (service, repo) = make_service();
        let ticket = service.create_ticket(make_create_cmd()).await.unwrap();

        service.update_ticket_channel(UpdateTicketChannelCommand {
            ticket_id: ticket.id.to_string(),
            voice_channel_id: Some("vc123".to_string()),
            invited_user_id: None,
        }).await.unwrap();

        let (uuid, vc_id) = repo.last_voice_channel.lock().unwrap().clone().unwrap();
        assert_eq!(uuid, ticket.id);
        assert_eq!(vc_id.unwrap(), "vc123");
        assert!(repo.last_invited_user.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn test_update_ticket_channel_invite() {
        let (service, repo) = make_service();
        let ticket = service.create_ticket(make_create_cmd()).await.unwrap();

        service.update_ticket_channel(UpdateTicketChannelCommand {
            ticket_id: ticket.id.to_string(),
            voice_channel_id: None,
            invited_user_id: Some("user456".to_string()),
        }).await.unwrap();

        assert!(repo.last_voice_channel.lock().unwrap().is_none());
        let (uuid, inv_id) = repo.last_invited_user.lock().unwrap().clone().unwrap();
        assert_eq!(uuid, ticket.id);
        assert_eq!(inv_id.unwrap(), "user456");
    }

    #[tokio::test]
    async fn test_get_ticket_detail() {
        let (service, _) = make_service();
        let ticket = service.create_ticket(make_create_cmd()).await.unwrap();

        let detail = service.get_ticket_detail(&ticket.id.to_string()).await.unwrap();
        assert_eq!(detail.ticket.id, ticket.id);
        assert!(detail.messages.is_empty());
    }

    #[tokio::test]
    async fn test_get_ticket_detail_invalid_uuid() {
        let (service, _) = make_service();
        let result = service.get_ticket_detail("invalid").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_ticket_detail_not_found() {
        let (service, _) = make_service();
        let fake_id = Uuid::new_v4().to_string();
        let result = service.get_ticket_detail(&fake_id).await;
        assert!(result.is_err());
    }
}
