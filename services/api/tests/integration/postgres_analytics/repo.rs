//! Tests d'integration postgres pour PgAnalyticsRepository.

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::PgAnalyticsRepository;
use sentinel_api::ports::outbound::AnalyticsRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn heatmap_empty_for_fresh_guild() {
    let repo = PgAnalyticsRepository::new(pool().await);
    let got = repo.get_heatmap(Some(&fresh_id()), 7).await.unwrap();
    assert!(got.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn heatmap_none_guild_aggregates_across() {
    let repo = PgAnalyticsRepository::new(pool().await);
    // Pas d'assertion specifique — juste verifier que la query ne panique pas.
    let _ = repo.get_heatmap(None, 30).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_hourly_and_heatmap_shows_data() {
    let repo = PgAnalyticsRepository::new(pool().await);
    let g = fresh_id();
    repo.record_hourly(&g, 14, 100, 5).await.unwrap();
    let heatmap = repo.get_heatmap(Some(&g), 1).await.unwrap();
    assert!(!heatmap.is_empty());
    assert!(heatmap.iter().any(|h| h.hour == 14 && h.messages == 100 && h.infractions == 5));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_hourly_is_upsert_on_same_day_hour() {
    let repo = PgAnalyticsRepository::new(pool().await);
    let g = fresh_id();
    repo.record_hourly(&g, 10, 50, 2).await.unwrap();
    repo.record_hourly(&g, 10, 30, 1).await.unwrap();
    let heatmap = repo.get_heatmap(Some(&g), 1).await.unwrap();
    let h10 = heatmap.iter().find(|h| h.hour == 10).unwrap();
    // Increment → 50 + 30 = 80, 2 + 1 = 3.
    assert_eq!(h10.messages, 80);
    assert_eq!(h10.infractions, 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn action_distribution_empty_for_fresh_guild() {
    let repo = PgAnalyticsRepository::new(pool().await);
    let got = repo.get_action_distribution(Some(&fresh_id()), 30).await.unwrap();
    assert!(got.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn action_distribution_groups_with_percentage() {
    let p = pool().await;
    let repo = PgAnalyticsRepository::new(p.clone());
    let g = fresh_id();
    // Seed 3 infractions warn + 1 mute dans les derniers jours.
    for _ in 0..3 {
        sqlx::query(
            "INSERT INTO infractions (id, guild_id, channel_id, user_id, username, message_id, \
              content, flags, score, action, reason, duration, created_at) \
             VALUES ($1, $2, 'c', 'u', 'u', 'm', 'x', '{}'::jsonb, 0.5, 'warn', 'r', NULL, NOW())"
        ).bind(Uuid::new_v4()).bind(&g).execute(&p).await.unwrap();
    }
    sqlx::query(
        "INSERT INTO infractions (id, guild_id, channel_id, user_id, username, message_id, \
          content, flags, score, action, reason, duration, created_at) \
         VALUES ($1, $2, 'c', 'u', 'u', 'm', 'x', '{}'::jsonb, 0.8, 'mute', 'r', 300, NOW())"
    ).bind(Uuid::new_v4()).bind(&g).execute(&p).await.unwrap();

    let got = repo.get_action_distribution(Some(&g), 7).await.unwrap();
    assert_eq!(got.len(), 2);
    let warn = got.iter().find(|a| a.action == "warn").unwrap();
    assert_eq!(warn.count, 3);
    assert_eq!(warn.percentage, 75.0);
    let mute = got.iter().find(|a| a.action == "mute").unwrap();
    assert_eq!(mute.count, 1);
    assert_eq!(mute.percentage, 25.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn top_infractors_empty() {
    let repo = PgAnalyticsRepository::new(pool().await);
    let got = repo.get_top_infractors(Some(&fresh_id()), 30, 10).await.unwrap();
    assert!(got.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn top_infractors_ranks_by_count() {
    let p = pool().await;
    let repo = PgAnalyticsRepository::new(p.clone());
    let g = fresh_id();
    let u1 = fresh_id();
    let u2 = fresh_id();
    // u1 a 3 infractions, u2 en a 1
    for _ in 0..3 {
        sqlx::query(
            "INSERT INTO infractions (id, guild_id, channel_id, user_id, username, message_id, \
              content, flags, score, action, reason, duration, created_at) \
             VALUES ($1, $2, 'c', $3, 'Alice', 'm', 'x', '{}'::jsonb, 0.5, 'warn', 'r', NULL, NOW())"
        ).bind(Uuid::new_v4()).bind(&g).bind(&u1).execute(&p).await.unwrap();
    }
    sqlx::query(
        "INSERT INTO infractions (id, guild_id, channel_id, user_id, username, message_id, \
          content, flags, score, action, reason, duration, created_at) \
         VALUES ($1, $2, 'c', $3, 'Bob', 'm', 'x', '{}'::jsonb, 0.5, 'warn', 'r', NULL, NOW())"
    ).bind(Uuid::new_v4()).bind(&g).bind(&u2).execute(&p).await.unwrap();

    let top = repo.get_top_infractors(Some(&g), 7, 10).await.unwrap();
    assert_eq!(top.len(), 2);
    assert_eq!(top[0].user_id, u1);
    assert_eq!(top[0].total_infractions, 3);
    assert_eq!(top[1].total_infractions, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn moderation_trend_empty_ok() {
    let repo = PgAnalyticsRepository::new(pool().await);
    let _ = repo.get_moderation_trend(Some(&fresh_id()), 30).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn peak_hours_empty_ok() {
    let repo = PgAnalyticsRepository::new(pool().await);
    let got = repo.get_peak_hours(Some(&fresh_id()), 30).await.unwrap();
    assert!(got.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn peak_hours_after_recording_activity() {
    let repo = PgAnalyticsRepository::new(pool().await);
    let g = fresh_id();
    repo.record_hourly(&g, 20, 500, 2).await.unwrap();
    repo.record_hourly(&g, 21, 1000, 5).await.unwrap();
    let peaks = repo.get_peak_hours(Some(&g), 1).await.unwrap();
    assert!(!peaks.is_empty());
}
