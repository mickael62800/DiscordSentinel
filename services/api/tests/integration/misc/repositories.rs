//! Tests d'integration pour les nouveaux repositories (session refactor DRY).
//! Valide que les abstractions repository fonctionnent correctement sur une
//! vraie DB Postgres (docker-compose.test.yml).

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::{
    PgEvidenceRepository, PgGameRepository, PgPendingActionRepository,
    PgReviewRepository, PgSponsorshipRepository, PgTempRoleRepository,
};
use sentinel_api::ports::outbound::{
    EvidenceRepository, GameRepository, PendingActionRepository,
    ReviewRepository, SponsorshipRepository, TempRoleRepository,
};

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}

fn ugid() -> String { format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128) }

// ══════════════════════════════════════════════════════════
// GameRepository
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn game_repo_create_and_list() {
    let p = pool().await;
    let repo = PgGameRepository::new(p);
    let gid = ugid();

    repo.create(&gid, "Fortnite", "user1").await.unwrap();
    repo.create(&gid, "Valorant", "user1").await.unwrap();

    let games = repo.list(&gid).await.unwrap();
    assert_eq!(games.len(), 2);
}

#[tokio::test]
async fn game_repo_find_by_name() {
    let p = pool().await;
    let repo = PgGameRepository::new(p);
    let gid = ugid();

    repo.create(&gid, "Rocket League", "user1").await.unwrap();
    let found = repo.find_by_name(&gid, "rocket league").await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().game_name, "Rocket League");
}

#[tokio::test]
async fn game_repo_subscribe_and_get_subscribers() {
    let p = pool().await;
    let repo = PgGameRepository::new(p);
    let gid = ugid();

    let game = repo.create(&gid, "Apex", "user1").await.unwrap();
    repo.subscribe(&gid, &game.id, "u1").await.unwrap();
    repo.subscribe(&gid, &game.id, "u2").await.unwrap();

    let subs = repo.get_subscribers(&game.id).await.unwrap();
    assert_eq!(subs.len(), 2);
}

#[tokio::test]
async fn game_repo_delete_cascades() {
    let p = pool().await;
    let repo = PgGameRepository::new(p);
    let gid = ugid();

    let game = repo.create(&gid, "Minecraft", "user1").await.unwrap();
    repo.subscribe(&gid, &game.id, "u1").await.unwrap();
    assert!(repo.delete(&gid, &game.id).await.unwrap());

    let subs = repo.get_subscribers(&game.id).await.unwrap();
    assert_eq!(subs.len(), 0);
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

    let id = repo.create(&gid, "mod1", "Mod", "target1", "Target", "warn", "spam", None, None).await.unwrap();

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

    let id = repo.create(&gid, "mod1", "Mod", "t1", "T", "mute", "toxic", None, Some(600)).await.unwrap();
    let found = repo.get_guild_id(id).await.unwrap();
    assert_eq!(found, Some(gid));
}
