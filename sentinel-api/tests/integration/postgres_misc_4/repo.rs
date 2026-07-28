//! Tests d'integration postgres pour 5 repos : guild, coude_heist,
//! coude_steal_protection, coude_steal_boost, blackjack_table.

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::system::guild_repository::PgGuildRepository;
use sentinel_api::ports::outbound::system::guild_repository::GuildRepository;
use sentinel_core::domain::entities::system::guild::Guild;
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

// ══════════════════════════════════════════════════════════
// Guild
// ══════════════════════════════════════════════════════════

fn sample_guild(id: &str, name: &str) -> Guild {
    let now = Utc::now();
    Guild {
        guild_id: id.into(),
        name: name.into(),
        icon: Some("icon-hash".into()),
        member_count: 42,
        registered_at: now,
        updated_at: now,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guild_upsert_and_find() {
    let repo = PgGuildRepository::new(pool().await);
    let id = fresh_id();
    repo.upsert(&sample_guild(&id, "MyGuild")).await.unwrap();
    let got = repo.find_by_id(&id).await.unwrap().unwrap();
    assert_eq!(got.name, "MyGuild");
    assert_eq!(got.member_count, 42);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guild_find_by_id_none() {
    let repo = PgGuildRepository::new(pool().await);
    assert!(repo.find_by_id(&fresh_id()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guild_upsert_updates_existing() {
    let repo = PgGuildRepository::new(pool().await);
    let id = fresh_id();
    repo.upsert(&sample_guild(&id, "OldName")).await.unwrap();
    let mut updated = sample_guild(&id, "NewName");
    updated.member_count = 100;
    repo.upsert(&updated).await.unwrap();
    let got = repo.find_by_id(&id).await.unwrap().unwrap();
    assert_eq!(got.name, "NewName");
    assert_eq!(got.member_count, 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guild_find_all_includes_inserted() {
    let repo = PgGuildRepository::new(pool().await);
    let id = fresh_id();
    repo.upsert(&sample_guild(&id, "X")).await.unwrap();
    let all = repo.find_all().await.unwrap();
    assert!(all.iter().any(|g| g.guild_id.as_str() == id));
}

// ══════════════════════════════════════════════════════════
// CoudeHeist
// ══════════════════════════════════════════════════════════

