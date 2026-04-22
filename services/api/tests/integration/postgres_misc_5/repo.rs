//! Tests d'integration postgres : ia_config + sponsorship + temp_role + daily_activity.

use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::{
    PgDailyActivityRepository, PgIaConfigRepository, PgSponsorshipRepository,
    PgTempRoleRepository,
};
use sentinel_api::domain::entities::IaConfig;
use sentinel_api::ports::outbound::{
    DailyActivityRepository, IaConfigRepository, SponsorshipRepository, TempRoleRepository,
};

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

// ══════════════════════════════════════════════════════════
// IaConfig
// ══════════════════════════════════════════════════════════

fn sample_ia_config(guild: &str) -> IaConfig {
    let now = Utc::now();
    IaConfig {
        guild_id: guild.into(),
        text_enabled: true, text_threshold: 0.7,
        vision_enabled: false, vision_threshold: 0.5,
        context_dampening: 0.3,
        context_format: "natural".into(),
        context_max_messages: 5,
        context_max_chars: 2000,
        created_at: now, updated_at: now,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ia_config_get_none_when_absent() {
    let repo = PgIaConfigRepository::new(pool().await);
    assert!(repo.get(&fresh_id()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ia_config_save_and_get_roundtrip() {
    let repo = PgIaConfigRepository::new(pool().await);
    let g = fresh_id();
    let saved = repo.save(&sample_ia_config(&g)).await.unwrap();
    assert_eq!(saved.guild_id, g);
    assert_eq!(saved.text_threshold, 0.7);
    assert_eq!(saved.context_max_messages, 5);
    let got = repo.get(&g).await.unwrap().unwrap();
    assert_eq!(got.context_format, "natural");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ia_config_save_is_upsert() {
    let repo = PgIaConfigRepository::new(pool().await);
    let g = fresh_id();
    repo.save(&sample_ia_config(&g)).await.unwrap();
    let mut updated = sample_ia_config(&g);
    updated.text_threshold = 0.95;
    updated.vision_enabled = true;
    repo.save(&updated).await.unwrap();
    let got = repo.get(&g).await.unwrap().unwrap();
    assert_eq!(got.text_threshold, 0.95);
    assert!(got.vision_enabled);
}

// ══════════════════════════════════════════════════════════
// Sponsorship
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sponsorship_create_and_list() {
    let repo = PgSponsorshipRepository::new(pool().await);
    let g = fresh_id();
    repo.create(&g, "sponsor1", "user1").await.unwrap();
    repo.create(&g, "sponsor2", "user2").await.unwrap();
    let list = repo.list(&g).await.unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sponsorship_conflict_on_duplicate_sponsored() {
    // ON CONFLICT (guild, sponsored) DO NOTHING → 2e create sur meme sponsored = no-op.
    let repo = PgSponsorshipRepository::new(pool().await);
    let g = fresh_id();
    repo.create(&g, "sponsor-a", "user1").await.unwrap();
    repo.create(&g, "sponsor-b", "user1").await.unwrap(); // doit etre no-op
    let list = repo.list(&g).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].sponsor_id, "sponsor-a");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sponsorship_list_scoped_to_guild() {
    let repo = PgSponsorshipRepository::new(pool().await);
    let g1 = fresh_id(); let g2 = fresh_id();
    repo.create(&g1, "s", "u").await.unwrap();
    repo.create(&g2, "s", "u").await.unwrap();
    assert_eq!(repo.list(&g1).await.unwrap().len(), 1);
    assert_eq!(repo.list(&g2).await.unwrap().len(), 1);
}

// ══════════════════════════════════════════════════════════
// TempRole
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn temp_role_create_and_list() {
    let repo = PgTempRoleRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    let future = (Utc::now() + Duration::hours(1)).to_rfc3339();
    repo.create(&g, &u, "role1", &future).await.unwrap();
    let list = repo.list_active(&g).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].user_id, u);
    assert_eq!(list[0].role_id, "role1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn temp_role_create_is_upsert() {
    let repo = PgTempRoleRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    let t1 = (Utc::now() + Duration::hours(1)).to_rfc3339();
    let t2 = (Utc::now() + Duration::hours(5)).to_rfc3339();
    repo.create(&g, &u, "role1", &t1).await.unwrap();
    repo.create(&g, &u, "role1", &t2).await.unwrap();
    let list = repo.list_active(&g).await.unwrap();
    assert_eq!(list.len(), 1); // meme role → upsert
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn temp_role_list_excludes_expired() {
    let repo = PgTempRoleRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    let past = (Utc::now() - Duration::hours(1)).to_rfc3339();
    repo.create(&g, &u, "expired", &past).await.unwrap();
    let list = repo.list_active(&g).await.unwrap();
    assert_eq!(list.len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn temp_role_delete_removes() {
    let repo = PgTempRoleRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    let future = (Utc::now() + Duration::hours(1)).to_rfc3339();
    repo.create(&g, &u, "role1", &future).await.unwrap();
    repo.delete(&g, &u, "role1").await.unwrap();
    assert!(repo.list_active(&g).await.unwrap().is_empty());
}

// ══════════════════════════════════════════════════════════
// DailyActivity
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn daily_activity_empty_when_no_snapshot() {
    let repo = PgDailyActivityRepository::new(pool().await);
    let got = repo.get_activity(Some(&fresh_id()), 7).await.unwrap();
    assert!(got.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn daily_activity_record_and_fetch() {
    let repo = PgDailyActivityRepository::new(pool().await);
    let g = fresh_id();
    repo.record_daily_snapshot(&g).await.unwrap();
    let got = repo.get_activity(Some(&g), 1).await.unwrap();
    assert_eq!(got.len(), 1);
    // Pas de stats seed → tout devrait etre 0.
    assert_eq!(got[0].messages, 0);
    assert_eq!(got[0].voice_minutes, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn daily_activity_snapshot_is_upsert_on_same_day() {
    let repo = PgDailyActivityRepository::new(pool().await);
    let g = fresh_id();
    repo.record_daily_snapshot(&g).await.unwrap();
    repo.record_daily_snapshot(&g).await.unwrap();
    let got = repo.get_activity(Some(&g), 1).await.unwrap();
    assert_eq!(got.len(), 1); // ON CONFLICT (guild, day) DO UPDATE
}
