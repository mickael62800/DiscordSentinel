//! Tests d'integration postgres pour PgTicketRepository.

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::PgTicketRepository;
use sentinel_api::domain::entities::system::ticket::Ticket;
use sentinel_api::domain::entities::system::ticket::TicketMessage;
use sentinel_api::ports::outbound::system::ticket_repository::TicketRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

fn sample(title: &str) -> Ticket {
    let now = Utc::now();
    Ticket {
        id: Uuid::new_v4(),
        title: title.into(), status: "open".into(), priority: "medium".into(),
        author_id: fresh_id(), author_name: "Alice".into(),
        assigned_to: None,
        server: fresh_id(), category: "support".into(),
        ticket_type: "standard".into(),
        channel_id: None, voice_channel_id: None, invited_user_id: None,
        created_at: now, updated_at: now,
        messages_count: 0,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_all_empty() {
    let repo = PgTicketRepository::new(pool().await);
    let before = repo.find_all(None, None, Some("__no_match_search_term_xyz__"), None, 50, 0).await.unwrap();
    assert!(before.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_and_find_by_id() {
    let repo = PgTicketRepository::new(pool().await);
    let t = sample("My ticket");
    repo.save(&t).await.unwrap();
    let got = repo.find_by_id(t.id).await.unwrap().unwrap();
    assert_eq!(got.title, "My ticket");
    assert_eq!(got.status, "open");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_id_absent_returns_none() {
    let repo = PgTicketRepository::new(pool().await);
    assert!(repo.find_by_id(Uuid::new_v4()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_status_persists() {
    let repo = PgTicketRepository::new(pool().await);
    let t = sample("x");
    repo.save(&t).await.unwrap();
    repo.update_status(t.id, "closed").await.unwrap();
    let got = repo.find_by_id(t.id).await.unwrap().unwrap();
    assert_eq!(got.status, "closed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_assignee_persists() {
    let repo = PgTicketRepository::new(pool().await);
    let t = sample("x");
    repo.save(&t).await.unwrap();
    repo.update_assignee(t.id, "mod1").await.unwrap();
    let got = repo.find_by_id(t.id).await.unwrap().unwrap();
    assert_eq!(got.assigned_to.as_deref(), Some("mod1"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_priority_persists() {
    let repo = PgTicketRepository::new(pool().await);
    let t = sample("x");
    repo.save(&t).await.unwrap();
    repo.update_priority(t.id, "high").await.unwrap();
    let got = repo.find_by_id(t.id).await.unwrap().unwrap();
    assert_eq!(got.priority, "high");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_voice_channel_and_invited_user() {
    let repo = PgTicketRepository::new(pool().await);
    let t = sample("x");
    repo.save(&t).await.unwrap();
    repo.update_voice_channel(t.id, Some("voice1")).await.unwrap();
    repo.update_invited_user(t.id, Some("invited1")).await.unwrap();
    let got = repo.find_by_id(t.id).await.unwrap().unwrap();
    assert_eq!(got.voice_channel_id.as_deref(), Some("voice1"));
    assert_eq!(got.invited_user_id.as_deref(), Some("invited1"));
    // Clear (None)
    repo.update_voice_channel(t.id, None).await.unwrap();
    repo.update_invited_user(t.id, None).await.unwrap();
    let got = repo.find_by_id(t.id).await.unwrap().unwrap();
    assert!(got.voice_channel_id.is_none());
    assert!(got.invited_user_id.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_and_find_messages() {
    let repo = PgTicketRepository::new(pool().await);
    let t = sample("x");
    repo.save(&t).await.unwrap();

    let m1 = TicketMessage {
        id: Uuid::new_v4(), ticket_id: t.id,
        author_name: "Alice".into(), author_role: "user".into(),
        content: "Hello".into(), created_at: Utc::now() - chrono::Duration::seconds(5),
    };
    let m2 = TicketMessage {
        id: Uuid::new_v4(), ticket_id: t.id,
        author_name: "Mod".into(), author_role: "staff".into(),
        content: "Hi there".into(), created_at: Utc::now(),
    };
    repo.save_message(&m1).await.unwrap();
    repo.save_message(&m2).await.unwrap();

    let msgs = repo.find_messages(t.id).await.unwrap();
    assert_eq!(msgs.len(), 2);
    // Content preserved
    assert!(msgs.iter().any(|m| m.content == "Hello"));
    assert!(msgs.iter().any(|m| m.content == "Hi there"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_messages_empty_for_fresh_ticket() {
    let repo = PgTicketRepository::new(pool().await);
    let t = sample("x");
    repo.save(&t).await.unwrap();
    assert!(repo.find_messages(t.id).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_all_filters_by_author_and_status() {
    let repo = PgTicketRepository::new(pool().await);
    // Meme author pour scope unique au test.
    let author = fresh_id();
    let mut t1 = sample("one"); t1.status = "open".into(); t1.author_id = author.clone();
    let mut t2 = sample("two"); t2.status = "closed".into(); t2.author_id = author.clone();
    let mut t3 = sample("three"); t3.status = "open".into(); t3.author_id = author.clone();
    repo.save(&t1).await.unwrap();
    repo.save(&t2).await.unwrap();
    repo.save(&t3).await.unwrap();

    let open = repo.find_all(Some("open"), None, None, Some(&author), 50, 0).await.unwrap();
    assert_eq!(open.len(), 2);
    assert!(open.iter().all(|t| t.status == "open"));

    let all = repo.find_all(None, None, None, Some(&author), 50, 0).await.unwrap();
    assert_eq!(all.len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_sla_sets_satisfaction_rating() {
    let repo = PgTicketRepository::new(pool().await);
    let t = sample("x");
    repo.save(&t).await.unwrap();
    repo.update_sla(t.id, None, None, Some(5)).await.unwrap();
    // Pas d'expose de satisfaction dans Ticket, juste verifier que ca ne leve pas.
}
