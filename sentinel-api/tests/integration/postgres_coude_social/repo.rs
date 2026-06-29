//! Tests d'integration postgres pour PgSocialRepository.

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::coude::social_repository::PgSocialRepository;
use sentinel_api::ports::outbound::coude::social_repository::SocialRepository;
use sentinel_core::domain::entities::coude::social::LeaderboardCategory;
use sentinel_core::domain::entities::coude::social::NewDailyChaos;

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

// ── Cooldowns ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_cooldown_none_when_absent() {
    let repo = PgSocialRepository::new(pool().await);
    let got = repo
        .get_cooldown(&fresh_id(), &fresh_id(), "steal")
        .await
        .unwrap();
    assert!(got.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_and_get_cooldown_roundtrip() {
    let repo = PgSocialRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.set_cooldown(&g, &u, "steal", 3600).await.unwrap();
    let got = repo.get_cooldown(&g, &u, "steal").await.unwrap().unwrap();
    // expires_at est dans ~1h (tolerance large : NOW() DB vs Utc::now() client).
    let delta = got.signed_duration_since(Utc::now()).num_seconds();
    assert!(delta > 3500 && delta <= 3700, "delta={delta}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_cooldown_upserts_on_conflict() {
    let repo = PgSocialRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.set_cooldown(&g, &u, "steal", 60).await.unwrap();
    repo.set_cooldown(&g, &u, "steal", 7200).await.unwrap();
    let got = repo.get_cooldown(&g, &u, "steal").await.unwrap().unwrap();
    let delta = got.signed_duration_since(Utc::now()).num_seconds();
    assert!(delta > 7000, "second set should win, got delta={delta}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expired_cooldown_returns_none() {
    let p = pool().await;
    let repo = PgSocialRepository::new(p.clone());
    let g = fresh_id();
    let u = fresh_id();
    // Seed direct : cooldown deja expire.
    sqlx::query(
        "INSERT INTO coude_cooldowns (guild_id, user_id, action, expires_at) \
         VALUES ($1, $2, 'steal', NOW() - INTERVAL '1 hour')",
    )
    .bind(&g)
    .bind(&u)
    .execute(&p)
    .await
    .unwrap();
    assert!(repo.get_cooldown(&g, &u, "steal").await.unwrap().is_none());
}

// ── Leaderboard ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_empty_for_fresh_guild() {
    let repo = PgSocialRepository::new(pool().await);
    let got = repo
        .leaderboard(&fresh_id(), LeaderboardCategory::Richest, 10)
        .await
        .unwrap();
    assert!(got.is_empty());
}

// ── Events ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_active_events_returns_only_active_and_not_expired() {
    let p = pool().await;
    let repo = PgSocialRepository::new(p.clone());
    let g = fresh_id();
    // Un event actif non expire, un inactif, un expire.
    sqlx::query(
        "INSERT INTO coude_events (id, guild_id, event_type, active, started_at, expires_at) \
         VALUES ($1, $2, 'raid', TRUE, NOW(), NOW() + INTERVAL '1 hour')",
    )
    .bind(Uuid::new_v4())
    .bind(&g)
    .execute(&p)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO coude_events (id, guild_id, event_type, active, started_at, expires_at) \
         VALUES ($1, $2, 'inactive_evt', FALSE, NOW(), NOW() + INTERVAL '1 hour')",
    )
    .bind(Uuid::new_v4())
    .bind(&g)
    .execute(&p)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO coude_events (id, guild_id, event_type, active, started_at, expires_at) \
         VALUES ($1, $2, 'expired_evt', TRUE, NOW() - INTERVAL '2 hours', NOW() - INTERVAL '1 hour')"
    ).bind(Uuid::new_v4()).bind(&g).execute(&p).await.unwrap();

    let got = repo.list_active_events(&g).await.unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].event_type, "raid");
}

// ── Daily chaos ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_daily_chaos_and_count() {
    let repo = PgSocialRepository::new(pool().await);
    let g = fresh_id();
    assert_eq!(repo.count_daily_chaos_today(&g).await.unwrap(), 0);
    repo.log_daily_chaos(NewDailyChaos {
        guild_id: g.clone().into(),
        loser_id: fresh_id(),
        loser_name: "Loser".into(),
        winner_id: fresh_id(),
        winner_name: "Winner".into(),
        amount: 100,
    })
    .await
    .unwrap();
    assert_eq!(repo.count_daily_chaos_today(&g).await.unwrap(), 1);
    repo.log_daily_chaos(NewDailyChaos {
        guild_id: g.clone().into(),
        loser_id: fresh_id(),
        loser_name: "L2".into(),
        winner_id: fresh_id(),
        winner_name: "W2".into(),
        amount: 50,
    })
    .await
    .unwrap();
    assert_eq!(repo.count_daily_chaos_today(&g).await.unwrap(), 2);
}

// ── Season ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_or_bootstrap_current_season_creates_first_season() {
    let repo = PgSocialRepository::new(pool().await);
    let g = fresh_id();
    let season = repo.get_or_bootstrap_current_season(&g).await.unwrap();
    assert!(season.season_number >= 1);
    assert!(season.days_remaining > 0);
    // Un 2e appel renvoie la meme saison (pas de creation).
    let season2 = repo.get_or_bootstrap_current_season(&g).await.unwrap();
    assert_eq!(season2.season_number, season.season_number);
}
