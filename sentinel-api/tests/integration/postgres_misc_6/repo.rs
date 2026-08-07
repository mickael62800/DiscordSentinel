//! Tests d'integration postgres : evidence + pending_action + review + modstats.

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::audit::modstats_repository::PgModstatsRepository;
use sentinel_api::adapters::outbound::postgres::moderation::evidence_repository::PgEvidenceRepository;
use sentinel_api::adapters::outbound::postgres::moderation::pending_action_repository::PgPendingActionRepository;
use sentinel_api::adapters::outbound::postgres::moderation::review_repository::PgReviewRepository;
use sentinel_core::ports::outbound::audit::modstats_repository::ModstatsRepository;
use sentinel_core::ports::outbound::moderation::evidence_repository::EvidenceRepository;
use sentinel_core::ports::outbound::moderation::pending_action_repository::PendingActionRepository;
use sentinel_core::ports::outbound::moderation::review_repository::ReviewRepository;
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

async fn seed_moderation_action(p: &PgPool, guild: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO moderation_actions (id, guild_id, channel_id, moderator_id, moderator_name, \
          target_id, target_name, action_type, reason, gravity, duration, created_at) \
         VALUES ($1, $2, 'ch', 'mod', 'Mod', 't', 'Target', 'ban', 'test', NULL, NULL, NOW())",
    )
    .bind(id)
    .bind(guild)
    .execute(p)
    .await
    .unwrap();
    id
}

// ══════════════════════════════════════════════════════════
// Evidence
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn evidence_add_and_list() {
    let p = pool().await;
    let repo = PgEvidenceRepository::new(p.clone());
    let g = fresh_id();
    let action_id = seed_moderation_action(&p, &g).await;
    let e = repo
        .add(
            action_id,
            "https://example.com/evidence1.png",
            Some("desc"),
            "mod1",
            "Mod",
        )
        .await
        .unwrap();
    assert_eq!(e.action_id, action_id);
    assert_eq!(e.url, "https://example.com/evidence1.png");
    assert_eq!(e.description.as_deref(), Some("desc"));
    let list = repo.list(action_id).await.unwrap();
    assert_eq!(list.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn evidence_list_empty() {
    let repo = PgEvidenceRepository::new(pool().await);
    assert!(repo.list(Uuid::new_v4()).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn evidence_list_ordered_asc() {
    let p = pool().await;
    let repo = PgEvidenceRepository::new(p.clone());
    let g = fresh_id();
    let action_id = seed_moderation_action(&p, &g).await;
    repo.add(action_id, "https://a/1", None, "m", "M")
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    repo.add(action_id, "https://a/2", None, "m", "M")
        .await
        .unwrap();
    let list = repo.list(action_id).await.unwrap();
    assert_eq!(list[0].url, "https://a/1");
    assert_eq!(list[1].url, "https://a/2");
}

// ══════════════════════════════════════════════════════════
// PendingAction
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_create_and_list() {
    let repo = PgPendingActionRepository::new(pool().await);
    let g = fresh_id();
    let id = repo
        .create(
            &g,
            "mod1",
            "Mod",
            "target1",
            "Target",
            "ban",
            "reason",
            Some("high"),
            Some(3600),
        )
        .await
        .unwrap();
    assert!(!id.is_nil());
    let list = repo.list_pending(&g).await.unwrap();
    assert_eq!(list.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_get_guild_id() {
    let repo = PgPendingActionRepository::new(pool().await);
    let g = fresh_id();
    let id = repo
        .create(&g, "mod1", "Mod", "t", "T", "ban", "r", None, None)
        .await
        .unwrap();
    let got = repo.get_guild_id(id).await.unwrap().unwrap();
    assert_eq!(got, g);
    assert!(repo.get_guild_id(Uuid::new_v4()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_resolve_removes_from_pending() {
    let repo = PgPendingActionRepository::new(pool().await);
    let g = fresh_id();
    let id = repo
        .create(&g, "mod1", "Mod", "t", "T", "ban", "r", None, None)
        .await
        .unwrap();
    repo.resolve(id, "approved", "reviewer1").await.unwrap();
    let list = repo.list_pending(&g).await.unwrap();
    // N'apparait plus dans les pending.
    assert!(!list.iter().any(|a| a.id == id));
}

// ══════════════════════════════════════════════════════════
// Review
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn review_add_and_list_pending() {
    let p = pool().await;
    let repo = PgReviewRepository::new(p.clone());
    let g = fresh_id();
    let action_id = seed_moderation_action(&p, &g).await;
    let entry = repo
        .add(action_id, &g, "mod1", "Mod", Some("too harsh"))
        .await
        .unwrap();
    assert_eq!(entry.status, "pending");
    let list = repo.list_pending(&g).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].action_type.as_deref(), Some("ban"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn review_get_guild_id() {
    let p = pool().await;
    let repo = PgReviewRepository::new(p.clone());
    let g = fresh_id();
    let action_id = seed_moderation_action(&p, &g).await;
    let entry = repo.add(action_id, &g, "m", "M", None).await.unwrap();
    assert_eq!(repo.get_guild_id(entry.id).await.unwrap().unwrap(), g);
    assert!(repo.get_guild_id(Uuid::new_v4()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn review_resolve_updates_status() {
    let p = pool().await;
    let repo = PgReviewRepository::new(p.clone());
    let g = fresh_id();
    let action_id = seed_moderation_action(&p, &g).await;
    let entry = repo.add(action_id, &g, "m", "M", None).await.unwrap();
    assert!(repo
        .resolve(entry.id, "reviewer1", "Reviewer", Some("ok"), "approved")
        .await
        .unwrap());
    let pending = repo.list_pending(&g).await.unwrap();
    assert!(!pending.iter().any(|r| r.id == entry.id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn review_resolve_unknown_returns_false() {
    let repo = PgReviewRepository::new(pool().await);
    assert!(!repo
        .resolve(Uuid::new_v4(), "r", "R", None, "approved")
        .await
        .unwrap());
}

// ══════════════════════════════════════════════════════════
// Modstats
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn modstats_top_moderators_empty() {
    let repo = PgModstatsRepository::new(pool().await);
    let got = repo.top_moderators(&fresh_id(), 30, 10).await.unwrap();
    assert!(got.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn modstats_top_moderators_groups_and_orders() {
    let p = pool().await;
    let repo = PgModstatsRepository::new(p.clone());
    let g = fresh_id();
    // 3 actions par mod1, 1 par mod2
    for _ in 0..3 {
        sqlx::query(
            "INSERT INTO moderation_actions (id, guild_id, channel_id, moderator_id, moderator_name, \
              target_id, target_name, action_type, reason, gravity, duration, created_at) \
             VALUES ($1, $2, 'ch', 'mod1', 'Alice', 't', 'T', 'warn', 'x', NULL, NULL, NOW())"
        ).bind(Uuid::new_v4()).bind(&g).execute(&p).await.unwrap();
    }
    sqlx::query(
        "INSERT INTO moderation_actions (id, guild_id, channel_id, moderator_id, moderator_name, \
          target_id, target_name, action_type, reason, gravity, duration, created_at) \
         VALUES ($1, $2, 'ch', 'mod2', 'Bob', 't', 'T', 'warn', 'x', NULL, NULL, NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(&g)
    .execute(&p)
    .await
    .unwrap();

    let top = repo.top_moderators(&g, 30, 10).await.unwrap();
    assert_eq!(top.len(), 2);
    assert_eq!(top[0].moderator_id, "mod1");
    assert_eq!(top[0].action_count, 3);
    assert_eq!(top[1].action_count, 1);
}
