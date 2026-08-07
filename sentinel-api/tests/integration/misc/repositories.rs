//! Tests d'integration pour les nouveaux repositories (session refactor DRY).
//! Valide que les abstractions repository fonctionnent correctement sur une
//! vraie DB Postgres (docker-compose.test.yml).

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::community::temp_role_repository::PgTempRoleRepository;
use sentinel_api::adapters::outbound::postgres::community::sponsorship_repository::PgSponsorshipRepository;
use sentinel_api::adapters::outbound::postgres::moderation::pending_action_repository::PgPendingActionRepository;
use sentinel_core::ports::outbound::community::temp_role_repository::TempRoleRepository;
use sentinel_core::ports::outbound::community::sponsorship_repository::SponsorshipRepository;
use sentinel_core::ports::outbound::moderation::pending_action_repository::PendingActionRepository;
async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}

fn ugid() -> String {
    format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    )
}

// ══════════════════════════════════════════════════════════
// SponsorshipRepository
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn sponsorship_repo_create_and_list() {
    let p = pool().await;
    let repo = PgSponsorshipRepository::new(p);
    let gid = ugid();

    repo.create(&gid, "sponsor1", "sponsored1").await.unwrap();
    let list = repo.list(&gid).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].sponsor_id, "sponsor1");
}

#[tokio::test]
async fn sponsorship_repo_idempotent() {
    let p = pool().await;
    let repo = PgSponsorshipRepository::new(p);
    let gid = ugid();

    repo.create(&gid, "s1", "sp1").await.unwrap();
    repo.create(&gid, "s2", "sp1").await.unwrap(); // ON CONFLICT DO NOTHING
    let list = repo.list(&gid).await.unwrap();
    assert_eq!(list.len(), 1); // sp1 ne peut avoir qu'un parrain
}

// ══════════════════════════════════════════════════════════
// TempRoleRepository
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn temp_role_repo_create_list_delete() {
    let p = pool().await;
    let repo = PgTempRoleRepository::new(p);
    let gid = ugid();

    let expires = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
    repo.create(&gid, "u1", "r1", &expires).await.unwrap();

    let list = repo.list_active(&gid).await.unwrap();
    assert_eq!(list.len(), 1);

    repo.delete(&gid, "u1", "r1").await.unwrap();
    let list2 = repo.list_active(&gid).await.unwrap();
    assert_eq!(list2.len(), 0);
}

// ══════════════════════════════════════════════════════════
// PendingActionRepository
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn pending_action_repo_create_list_resolve() {
    let p = pool().await;
    let repo = PgPendingActionRepository::new(p);
    let gid = ugid();

    let id = repo
        .create(
            &gid, "mod1", "Mod", "target1", "Target", "warn", "spam", None, None,
        )
        .await
        .unwrap();

    let list = repo.list_pending(&gid).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, id);

    repo.resolve(id, "approved", "reviewer1").await.unwrap();
    let list2 = repo.list_pending(&gid).await.unwrap();
    assert_eq!(list2.len(), 0); // plus pending
}

#[tokio::test]
async fn pending_action_repo_get_guild_id() {
    let p = pool().await;
    let repo = PgPendingActionRepository::new(p);
    let gid = ugid();

    let id = repo
        .create(
            &gid,
            "mod1",
            "Mod",
            "t1",
            "T",
            "mute",
            "toxic",
            None,
            Some(600),
        )
        .await
        .unwrap();
    let found = repo.get_guild_id(id).await.unwrap();
    assert_eq!(found, Some(gid));
}
