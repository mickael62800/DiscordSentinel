//! Tests d'integration HTTP pour les endpoints tickets.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_core::ports::inbound::system::manage_tickets::*;
use sentinel_core::domain::entities::system::ticket::*;
use sentinel_core::domain::errors::DomainError;

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
        Self {
            tickets: vec![],
            messages: vec![],
        }
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
        guild_id: Some("123456789012345678".into()),
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
    async fn list_tickets(
        &self,
        status: Option<String>,
        priority: Option<String>,
        _search: Option<String>,
        _author_id: Option<String>,
        _limit: i64,
        _offset: i64,
    ) -> Result<Vec<Ticket>, DomainError> {
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
        let uuid =
            Uuid::parse_str(id).map_err(|_| DomainError::NotFound(format!("Ticket {id}")))?;
        let ticket = self
            .tickets
            .iter()
            .find(|t| t.id == uuid)
            .ok_or_else(|| DomainError::NotFound(format!("Ticket {id}")))?;
        let msgs = self
            .messages
            .iter()
            .filter(|m| m.ticket_id == uuid)
            .cloned()
            .collect();
        Ok(TicketDetail {
            ticket: ticket.clone(),
            messages: msgs,
        })
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
            guild_id: cmd.guild_id,
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

    async fn reply_ticket(&self, _: ReplyTicketCommand) -> Result<(), DomainError> {
        Ok(())
    }
    async fn close_ticket(&self, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn assign_ticket(&self, _: AssignTicketCommand) -> Result<(), DomainError> {
        Ok(())
    }
    async fn update_status(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn update_ticket_channel(
        &self,
        _: UpdateTicketChannelCommand,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn update_priority(&self, _: uuid::Uuid, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn update_sla(
        &self,
        _: uuid::Uuid,
        _: Option<&str>,
        _: Option<&str>,
        _: Option<i32>,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn moderated_guilds(
        &self,
        _: &str,
    ) -> Result<std::collections::HashSet<String>, DomainError> {
        Ok(std::collections::HashSet::new())
    }
    async fn bulk_delete_tickets(
        &self,
        author_id: Option<&str>,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<u64, DomainError> {
        // Ces tests inserent des tickets dans la vraie DB test puis verifient le
        // comptage : le mock delegue donc a la meme DB via une connexion directe.
        let p = pool().await;
        let res = sqlx::query(
            r#"
            DELETE FROM tickets
            WHERE ($1::text IS NULL OR author_id = $1)
              AND ($2::timestamptz IS NULL OR created_at >= $2)
              AND ($3::timestamptz IS NULL OR created_at <= $3)
            "#,
        )
        .bind(author_id)
        .bind(from)
        .bind(to)
        .execute(&p)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(res.rows_affected())
    }
}

// ══════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════

fn build_app(uc: MockTicketsUC) -> axum::Router {
    let state = build_test_state_tickets(Arc::new(uc));
    router::build_for_test(state)
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null),
    )
}

async fn post_json(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

async fn patch_json(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("PATCH")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
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
    let (status, json) = patch_json(
        app,
        &format!("/api/tickets/{id}/close"),
        serde_json::json!({}),
    )
    .await;
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

// ══════════════════════════════════════════════════════════
// bulk_delete_tickets (sqlx direct -> utilise la vraie DB test)
// ══════════════════════════════════════════════════════════

async fn pool() -> sqlx::PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    sqlx::PgPool::connect(&url).await.unwrap()
}

async fn insert_ticket(
    pool: &sqlx::PgPool,
    server: &str,
    author_id: &str,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Uuid {
    let id = Uuid::new_v4();
    match created_at {
        Some(dt) => {
            sqlx::query(
                "INSERT INTO tickets (id, title, status, priority, author_id, author_name, server, category, created_at) \
                 VALUES ($1, 'Test', 'open', 'medium', $2, 'User', $3, 'general', $4)",
            ).bind(id).bind(author_id).bind(server).bind(dt).execute(pool).await.unwrap();
        }
        None => {
            sqlx::query(
                "INSERT INTO tickets (id, title, status, priority, author_id, author_name, server, category) \
                 VALUES ($1, 'Test', 'open', 'medium', $2, 'User', $3, 'general')",
            ).bind(id).bind(author_id).bind(server).execute(pool).await.unwrap();
        }
    }
    id
}

async fn count_tickets_for_server(pool: &sqlx::PgPool, server: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM tickets WHERE server = $1")
        .bind(server)
        .fetch_one(pool)
        .await
        .unwrap()
        .0
}

async fn delete_req(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bulk_delete_requires_filter_or_all_flag_422() {
    let app = build_app(MockTicketsUC::new());
    let (status, json) = delete_req(app, "/api/tickets/bulk").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("Aucun filtre"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bulk_delete_filter_by_author_id_targeted() {
    let p = pool().await;
    let server = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let author_target = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let author_other = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    insert_ticket(&p, &server, &author_target, None).await;
    insert_ticket(&p, &server, &author_target, None).await;
    insert_ticket(&p, &server, &author_other, None).await;

    let app = build_app(MockTicketsUC::new());
    let (status, json) =
        delete_req(app, &format!("/api/tickets/bulk?author_id={author_target}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["deleted"], 2);
    let remaining = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM tickets WHERE server = $1 AND author_id = $2",
    )
    .bind(&server)
    .bind(&author_other)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;
    assert_eq!(remaining, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bulk_delete_filter_by_date_range_rfc3339() {
    let p = pool().await;
    let server = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let author = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let old_dt = chrono::DateTime::parse_from_rfc3339("2020-06-15T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    insert_ticket(&p, &server, &author, Some(old_dt)).await;
    insert_ticket(&p, &server, &author, None).await;

    let app = build_app(MockTicketsUC::new());
    let (status, json) = delete_req(
        app,
        &format!("/api/tickets/bulk?author_id={author}&from=2020-01-01T00:00:00Z&to=2020-12-31T23:59:59Z"),
    ).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["deleted"], 1);
    assert_eq!(count_tickets_for_server(&p, &server).await, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bulk_delete_filter_by_date_range_yyyy_mm_dd() {
    // Le format YYYY-MM-DD emprunte la branche 2 de parse_date.
    let p = pool().await;
    let server = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let author = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let old_dt = chrono::DateTime::parse_from_rfc3339("2021-03-15T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    insert_ticket(&p, &server, &author, Some(old_dt)).await;

    let app = build_app(MockTicketsUC::new());
    let (status, _) = delete_req(
        app,
        &format!("/api/tickets/bulk?author_id={author}&from=2021-01-01&to=2021-12-31"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(count_tickets_for_server(&p, &server).await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bulk_delete_invalid_date_format_422() {
    let app = build_app(MockTicketsUC::new());
    let (status, json) = delete_req(app, "/api/tickets/bulk?from=not-a-date").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().unwrap().contains("Date invalide"));
}