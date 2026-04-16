//! Tests d'integration HTTP pour les endpoints tickets.

mod test_helpers;

use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::domain::entities::*;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::ports::inbound::*;

use test_helpers::build_test_state_tickets;

// ══════════════════════════════════════════════════════════
// Mock Tickets Use Case
// ══════════════════════════════════════════════════════════

struct MockTicketsUC {
    tickets: Vec<Ticket>,
    messages: Vec<TicketMessage>,
}

impl MockTicketsUC {
    fn new() -> Self {
        Self { tickets: vec![], messages: vec![] }
    }

    fn with_ticket(mut self, t: Ticket) -> Self {
        self.tickets.push(t);
        self
    }
}

fn make_ticket(id: Uuid, title: &str, status: &str, priority: &str) -> Ticket {
    Ticket {
        id,
        title: title.into(),
        status: status.into(),
        priority: priority.into(),
        author_id: "user1".into(),
        author_name: "Alice".into(),
        assigned_to: None,
        server: "TestServer".into(),
        category: "bug".into(),
        ticket_type: "probleme_serveur".into(),
        channel_id: None,
        voice_channel_id: None,
        invited_user_id: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        messages_count: 0,
    }
}

#[async_trait]
impl ManageTicketsUseCase for MockTicketsUC {
    async fn list_tickets(&self, status: Option<String>, priority: Option<String>, _search: Option<String>, _author_id: Option<String>, _limit: i64, _offset: i64) -> Result<Vec<Ticket>, DomainError> {
        let mut result = self.tickets.clone();
        if let Some(s) = status {
            result.retain(|t| t.status == s);
        }
        if let Some(p) = priority {
            result.retain(|t| t.priority == p);
        }
        Ok(result)
    }

    async fn get_ticket_detail(&self, id: &str) -> Result<TicketDetail, DomainError> {
        let uuid = Uuid::parse_str(id)
            .map_err(|_| DomainError::NotFound(format!("Ticket {id}")))?;
        let ticket = self.tickets.iter().find(|t| t.id == uuid)
            .ok_or_else(|| DomainError::NotFound(format!("Ticket {id}")))?;
        let msgs = self.messages.iter().filter(|m| m.ticket_id == uuid).cloned().collect();
        Ok(TicketDetail { ticket: ticket.clone(), messages: msgs })
    }

    async fn create_ticket(&self, cmd: CreateTicketCommand) -> Result<Ticket, DomainError> {
        Ok(Ticket {
            id: Uuid::new_v4(),
            title: cmd.title,
            status: "open".into(),
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
            messages_count: 0,
        })
    }

    async fn reply_ticket(&self, _: ReplyTicketCommand) -> Result<(), DomainError> { Ok(()) }
    async fn close_ticket(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn assign_ticket(&self, _: AssignTicketCommand) -> Result<(), DomainError> { Ok(()) }
    async fn update_status(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn update_ticket_channel(&self, _: UpdateTicketChannelCommand) -> Result<(), DomainError> { Ok(()) }
    async fn update_priority(&self, _: uuid::Uuid, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn update_sla(&self, _: uuid::Uuid, _: Option<&str>, _: Option<&str>, _: Option<i32>) -> Result<(), DomainError> { Ok(()) }
}

// ══════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════

fn build_app(uc: MockTicketsUC) -> axum::Router {
    let state = build_test_state_tickets(Arc::new(uc));
    router::build_for_test(state)
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null))
}

async fn post_json(app: axum::Router, uri: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("POST").uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

async fn patch_json(app: axum::Router, uri: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("PATCH").uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — List tickets
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_tickets_empty() {
    let app = build_app(MockTicketsUC::new());
    let (status, json) = get(app, "/api/tickets").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_tickets_with_data() {
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let uc = MockTicketsUC::new()
        .with_ticket(make_ticket(id1, "Bug 1", "open", "high"))
        .with_ticket(make_ticket(id2, "Bug 2", "closed", "low"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/tickets").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_tickets_filter_by_status() {
    let uc = MockTicketsUC::new()
        .with_ticket(make_ticket(Uuid::new_v4(), "Open", "open", "high"))
        .with_ticket(make_ticket(Uuid::new_v4(), "Closed", "closed", "low"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/tickets?status=open").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["status"], "open");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_tickets_filter_by_priority() {
    let uc = MockTicketsUC::new()
        .with_ticket(make_ticket(Uuid::new_v4(), "High", "open", "high"))
        .with_ticket(make_ticket(Uuid::new_v4(), "Low", "open", "low"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/tickets?priority=high").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["priority"], "high");
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — Get ticket detail
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_ticket_detail_success() {
    let id = Uuid::new_v4();
    let uc = MockTicketsUC::new().with_ticket(make_ticket(id, "Bug", "open", "high"));
    let app = build_app(uc);
    let (status, json) = get(app, &format!("/api/tickets/{id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ticket"]["title"], "Bug");
    assert!(json["messages"].as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_ticket_detail_not_found() {
    let app = build_app(MockTicketsUC::new());
    let (status, json) = get(app, &format!("/api/tickets/{}", Uuid::new_v4())).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].as_str().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_ticket_detail_invalid_uuid() {
    let app = build_app(MockTicketsUC::new());
    let (status, _) = get(app, "/api/tickets/not-a-uuid").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — Create ticket
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_ticket_success() {
    let app = build_app(MockTicketsUC::new());
    let body = serde_json::json!({
        "title": "Nouveau bug",
        "author_id": "user1",
        "author_name": "Alice",
        "server": "TestServer"
    });
    let (status, json) = post_json(app, "/api/tickets", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["title"], "Nouveau bug");
    assert_eq!(json["status"], "open");
    assert_eq!(json["priority"], "medium"); // default
    assert_eq!(json["ticket_type"], "autre"); // default
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_ticket_with_custom_priority() {
    let app = build_app(MockTicketsUC::new());
    let body = serde_json::json!({
        "title": "Urgent",
        "priority": "urgent",
        "author_id": "user1",
        "author_name": "Alice",
        "server": "TestServer",
        "ticket_type": "urgence_detresse"
    });
    let (status, json) = post_json(app, "/api/tickets", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["priority"], "urgent");
    assert_eq!(json["ticket_type"], "urgence_detresse");
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — Reply
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reply_ticket_success() {
    let id = Uuid::new_v4();
    let uc = MockTicketsUC::new().with_ticket(make_ticket(id, "Bug", "open", "high"));
    let app = build_app(uc);
    let body = serde_json::json!({
        "content": "On regarde"
    });
    let (status, json) = post_json(app, &format!("/api/tickets/{id}/messages"), body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — Close
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn close_ticket_success() {
    let id = Uuid::new_v4();
    let uc = MockTicketsUC::new().with_ticket(make_ticket(id, "Bug", "open", "high"));
    let app = build_app(uc);
    let (status, json) = patch_json(app, &format!("/api/tickets/{id}/close"), serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — Assign
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn assign_ticket_success() {
    let id = Uuid::new_v4();
    let uc = MockTicketsUC::new().with_ticket(make_ticket(id, "Bug", "open", "high"));
    let app = build_app(uc);
    let body = serde_json::json!({ "assignee": "mod1" });
    let (status, json) = patch_json(app, &format!("/api/tickets/{id}/assign"), body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — Update status (with validation)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_status_valid_open() {
    let id = Uuid::new_v4();
    let uc = MockTicketsUC::new().with_ticket(make_ticket(id, "Bug", "pending", "high"));
    let app = build_app(uc);
    let body = serde_json::json!({ "status": "open" });
    let (status, json) = patch_json(app, &format!("/api/tickets/{id}/status"), body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_status_valid_pending() {
    let id = Uuid::new_v4();
    let app = build_app(MockTicketsUC::new().with_ticket(make_ticket(id, "T", "open", "low")));
    let body = serde_json::json!({ "status": "pending" });
    let (status, _) = patch_json(app, &format!("/api/tickets/{id}/status"), body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_status_valid_closed() {
    let id = Uuid::new_v4();
    let app = build_app(MockTicketsUC::new().with_ticket(make_ticket(id, "T", "open", "low")));
    let body = serde_json::json!({ "status": "closed" });
    let (status, _) = patch_json(app, &format!("/api/tickets/{id}/status"), body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_status_invalid_rejected() {
    let id = Uuid::new_v4();
    let app = build_app(MockTicketsUC::new().with_ticket(make_ticket(id, "T", "open", "low")));
    let body = serde_json::json!({ "status": "invalid_status" });
    let (status, json) = patch_json(app, &format!("/api/tickets/{id}/status"), body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(json["error"].as_str().unwrap().contains("invalide"));
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — Update ticket channel
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_ticket_channel_success() {
    let id = Uuid::new_v4();
    let app = build_app(MockTicketsUC::new().with_ticket(make_ticket(id, "T", "open", "low")));
    let body = serde_json::json!({
        "voice_channel_id": "vc123",
        "invited_user_id": "user2"
    });
    let (status, json) = patch_json(app, &format!("/api/tickets/{id}/channels"), body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}
