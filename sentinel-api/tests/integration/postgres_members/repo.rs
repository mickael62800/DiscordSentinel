//! Tests d'integration postgres pour PgMemberRepository.

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::community::member_repository::PgMemberRepository;
use sentinel_core::ports::outbound::community::member_repository::MemberRepository;
use sentinel_core::domain::entities::community::guild_member::GuildMember;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    )
}
fn member(guild: &str, user: &str, name: &str) -> GuildMember {
    GuildMember {
        guild_id: guild.into(),
        user_id: user.into(),
        username: name.into(),
        display_name: None,
        avatar: None,
        roles: serde_json::json!([]),
        joined_at: None,
        account_created: None,
        is_bot: false,
        last_seen_at: None,
        left_at: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_guild_empty() {
    let repo = PgMemberRepository::new(pool().await);
    let got = repo.find_by_guild(&fresh_id()).await.unwrap();
    assert!(got.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upsert_and_find_one() {
    let repo = PgMemberRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.upsert(&member(&g, &u, "Alice")).await.unwrap();
    let got = repo.find_one(&g, &u).await.unwrap().unwrap();
    assert_eq!(got.username, "Alice");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upsert_twice_updates_username() {
    let repo = PgMemberRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.upsert(&member(&g, &u, "OldName")).await.unwrap();
    repo.upsert(&member(&g, &u, "NewName")).await.unwrap();
    let got = repo.find_one(&g, &u).await.unwrap().unwrap();
    assert_eq!(got.username, "NewName");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_one_not_found_returns_none() {
    let repo = PgMemberRepository::new(pool().await);
    assert!(repo
        .find_one(&fresh_id(), &fresh_id())
        .await
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_guild_ordered_by_username() {
    let repo = PgMemberRepository::new(pool().await);
    let g = fresh_id();
    repo.upsert(&member(&g, &fresh_id(), "Charlie"))
        .await
        .unwrap();
    repo.upsert(&member(&g, &fresh_id(), "Alice"))
        .await
        .unwrap();
    repo.upsert(&member(&g, &fresh_id(), "Bob")).await.unwrap();
    let got = repo.find_by_guild(&g).await.unwrap();
    assert_eq!(got.len(), 3);
    assert_eq!(got[0].username, "Alice");
    assert_eq!(got[1].username, "Bob");
    assert_eq!(got[2].username, "Charlie");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upsert_many_inserts_batch_atomically() {
    let repo = PgMemberRepository::new(pool().await);
    let g = fresh_id();
    let members = vec![
        member(&g, &fresh_id(), "A"),
        member(&g, &fresh_id(), "B"),
        member(&g, &fresh_id(), "C"),
    ];
    let n = repo.upsert_many(&members).await.unwrap();
    assert_eq!(n, 3);
    assert_eq!(repo.find_by_guild(&g).await.unwrap().len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upsert_many_empty_returns_zero() {
    let repo = PgMemberRepository::new(pool().await);
    assert_eq!(repo.upsert_many(&[]).await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_removes_member() {
    let repo = PgMemberRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.upsert(&member(&g, &u, "Alice")).await.unwrap();
    repo.delete(&g, &u).await.unwrap();
    assert!(repo.find_one(&g, &u).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_last_seen_touches_timestamp() {
    let repo = PgMemberRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.upsert(&member(&g, &u, "Alice")).await.unwrap();
    let before = repo.find_one(&g, &u).await.unwrap().unwrap().last_seen_at;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    repo.update_last_seen(&g, &u).await.unwrap();
    let after = repo.find_one(&g, &u).await.unwrap().unwrap().last_seen_at;
    assert!(
        after > before,
        "last_seen_at didn't advance: before={before:?} after={after:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upsert_persists_roles_and_display_name() {
    let repo = PgMemberRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    let mut m = member(&g, &u, "Alice");
    m.display_name = Some("Alice (admin)".into());
    m.roles = serde_json::json!(["role1", "role2"]);
    m.is_bot = true;
    repo.upsert(&m).await.unwrap();
    let got = repo.find_one(&g, &u).await.unwrap().unwrap();
    assert_eq!(got.display_name.as_deref(), Some("Alice (admin)"));
    assert_eq!(got.roles, serde_json::json!(["role1", "role2"]));
    assert!(got.is_bot);
}
