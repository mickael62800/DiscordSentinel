use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use uuid::Uuid;

use crate::application::ManageTicketsService;
use crate::domain::entities::{Rule, Ticket, TicketMessage};
use crate::domain::errors::DomainError;
use crate::ports::inbound::{
    AssignTicketCommand, CreateTicketCommand, ManageTicketsUseCase, ReplyTicketCommand,
    UpdateTicketChannelCommand,
};
use crate::ports::outbound::{CachePort, TicketRepository};

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
    async fn find_all(&self, status: Option<&str>, _priority: Option<&str>, _search: Option<&str>, _author_id: Option<&str>, _limit: i64, _offset: i64) -> Result<Vec<Ticket>, DomainError> {
        let tickets = self.tickets.lock().unwrap();
        Ok(tickets.iter().filter(|t| {
            status.is_none_or(|s| t.status == s)
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
    async fn update_priority(&self, _id: Uuid, _priority: &str) -> Result<(), DomainError> { Ok(()) }
    async fn update_sla(&self, _id: Uuid, _fr: Option<&str>, _ra: Option<&str>, _rating: Option<i32>) -> Result<(), DomainError> { Ok(()) }
}

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
    let cache = Arc::new(MockCache);
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

    let tickets = service.list_tickets(None, None, None, None, 50, 0).await.unwrap();
    assert_eq!(tickets.len(), 1);
}

#[tokio::test]
async fn test_list_tickets_filter_by_status() {
    let (service, _repo) = make_service();
    service.create_ticket(make_create_cmd()).await.unwrap();

    let open = service.list_tickets(Some("open".to_string()), None, None, None, 50, 0).await.unwrap();
    assert_eq!(open.len(), 1);

    let closed = service.list_tickets(Some("closed".to_string()), None, None, None, 50, 0).await.unwrap();
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

// ── update_priority ──

#[tokio::test]
async fn update_priority_delegates() {
    let (service, _) = make_service();
    let ticket = service.create_ticket(make_create_cmd()).await.unwrap();
    // Ne panique pas malgre la mock trivial update_priority -> Ok(())
    assert!(service.update_priority(ticket.id, "high").await.is_ok());
}

// ── update_sla ──

#[tokio::test]
async fn update_sla_with_all_fields() {
    let (service, _) = make_service();
    let ticket = service.create_ticket(make_create_cmd()).await.unwrap();
    assert!(service
        .update_sla(
            ticket.id,
            Some("2026-01-01T00:00:00Z"),
            Some("2026-01-02T00:00:00Z"),
            Some(5),
        )
        .await
        .is_ok());
}

#[tokio::test]
async fn update_sla_with_none_fields() {
    let (service, _) = make_service();
    let ticket = service.create_ticket(make_create_cmd()).await.unwrap();
    assert!(service.update_sla(ticket.id, None, None, None).await.is_ok());
}

// ── reply_ticket / assign_ticket / update_ticket_channel avec ID invalide ──

#[tokio::test]
async fn reply_ticket_invalid_uuid_returns_error() {
    let (service, _) = make_service();
    let err = service.reply_ticket(ReplyTicketCommand {
        ticket_id: "not-a-uuid".into(),
        content: "".into(),
        author_name: "".into(),
        author_role: "".into(),
    }).await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidRule(_)));
}

#[tokio::test]
async fn assign_ticket_invalid_uuid_returns_error() {
    let (service, _) = make_service();
    let err = service.assign_ticket(AssignTicketCommand {
        ticket_id: "bad".into(),
        assignee: "x".into(),
    }).await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidRule(_)));
}

#[tokio::test]
async fn update_ticket_channel_invalid_uuid_returns_error() {
    let (service, _) = make_service();
    let err = service.update_ticket_channel(UpdateTicketChannelCommand {
        ticket_id: "xyz".into(),
        voice_channel_id: Some("v".into()),
        invited_user_id: None,
    }).await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidRule(_)));
}

#[tokio::test]
async fn update_status_invalid_uuid_returns_error() {
    let (service, _) = make_service();
    let err = service.update_status("nope", "pending").await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidRule(_)));
}

// ── list_tickets avec filtre author_id + priority + search ──

#[tokio::test]
async fn list_tickets_with_all_filters_bypasses_cache() {
    let (service, _repo) = make_service();
    // Les filtres actifs forcent un read repo direct (pas de cache set/get).
    let tickets = service.list_tickets(
        Some("open".into()),
        Some("high".into()),
        Some("search".into()),
        Some("author".into()),
        10, 0,
    ).await.unwrap();
    assert!(tickets.is_empty());
}

#[tokio::test]
async fn list_tickets_with_offset_skips_cache() {
    let (service, _repo) = make_service();
    // offset != 0 → pas de cache
    let tickets = service.list_tickets(None, None, None, None, 10, 50).await.unwrap();
    assert!(tickets.is_empty());
}
