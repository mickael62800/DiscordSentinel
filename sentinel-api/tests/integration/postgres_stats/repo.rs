//! Tests d'integration postgres pour PgStatsRepository.

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::audit::stats_repository::PgStatsRepository;
use sentinel_api::ports::outbound::audit::stats_repository::StatsRepository;
use sentinel_core::domain::entities::audit::user_stats::UserStats;

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

fn sample_stats(guild: &str, user: &str, msgs: u64, voice: u64) -> UserStats {
    UserStats {
        id: Uuid::new_v4(),
        guild_id: guild.into(),
        user_id: user.into(),
        username: user.into(),
        message_count: msgs,
        voice_seconds: voice,
        updated_at: Utc::now(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_user_none_when_absent() {
    let repo = PgStatsRepository::new(pool().await);
    assert!(repo
        .find_by_user(&fresh_id(), &fresh_id())
        .await
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upsert_and_find_by_user() {
    let repo = PgStatsRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.upsert(&sample_stats(&g, &u, 10, 200)).await.unwrap();
    let got = repo.find_by_user(&g, &u).await.unwrap().unwrap();
    assert_eq!(got.message_count, 10);
    assert_eq!(got.voice_seconds, 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upsert_replaces_counts_not_accumulates() {
    let repo = PgStatsRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.upsert(&sample_stats(&g, &u, 100, 500)).await.unwrap();
    repo.upsert(&sample_stats(&g, &u, 42, 10)).await.unwrap();
    let got = repo.find_by_user(&g, &u).await.unwrap().unwrap();
    // upsert = overwrite
    assert_eq!(got.message_count, 42);
    assert_eq!(got.voice_seconds, 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn increment_messages_creates_then_accumulates() {
    let repo = PgStatsRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.increment_messages(&g, &u, "Alice", 5).await.unwrap();
    repo.increment_messages(&g, &u, "Alice", 3).await.unwrap();
    let got = repo.find_by_user(&g, &u).await.unwrap().unwrap();
    assert_eq!(got.message_count, 8);
    assert_eq!(got.voice_seconds, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_voice_seconds_creates_then_accumulates() {
    let repo = PgStatsRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.add_voice_seconds(&g, &u, "Alice", 60).await.unwrap();
    repo.add_voice_seconds(&g, &u, "Alice", 120).await.unwrap();
    let got = repo.find_by_user(&g, &u).await.unwrap().unwrap();
    assert_eq!(got.voice_seconds, 180);
    assert_eq!(got.message_count, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_guild_ordered_by_message_count_desc() {
    let repo = PgStatsRepository::new(pool().await);
    let g = fresh_id();
    for (u, msgs) in [("u1", 10), ("u2", 50), ("u3", 30)] {
        let id = format!("{}{u}", fresh_id());
        let id = &id[..18.min(id.len())];
        repo.upsert(&sample_stats(&g, id, msgs, 0)).await.unwrap();
    }
    let got = repo.find_by_guild(&g, 10).await.unwrap();
    assert_eq!(got.len(), 3);
    assert_eq!(got[0].message_count, 50);
    assert_eq!(got[1].message_count, 30);
    assert_eq!(got[2].message_count, 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn count_distinct_guilds_and_users() {
    let repo = PgStatsRepository::new(pool().await);
    // Seed 2 guilds avec 2 users chacune
    let g1 = fresh_id();
    let g2 = fresh_id();
    repo.upsert(&sample_stats(&g1, &fresh_id(), 1, 0))
        .await
        .unwrap();
    repo.upsert(&sample_stats(&g1, &fresh_id(), 1, 0))
        .await
        .unwrap();
    repo.upsert(&sample_stats(&g2, &fresh_id(), 1, 0))
        .await
        .unwrap();
    // Les counts sont globaux (pas scope par test) — on verifie juste >= 2.
    assert!(repo.count_distinct_guilds().await.unwrap() >= 2);
    assert!(repo.count_distinct_users().await.unwrap() >= 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_voice_session_and_get_guild_voice_stats() {
    let repo = PgStatsRepository::new(pool().await);
    let g = fresh_id();
    let channel_id = fresh_id();
    let u1 = fresh_id();
    let u2 = fresh_id();
    repo.save_voice_session(&g, &u1, "Alice", &channel_id, "general", 120)
        .await
        .unwrap();
    repo.save_voice_session(&g, &u2, "Bob", &channel_id, "general", 60)
        .await
        .unwrap();

    let stats = repo.get_guild_voice_stats(&g, 7, 10).await.unwrap();
    assert_eq!(stats.len(), 1);
    let row = &stats[0];
    assert_eq!(row.channel_id, channel_id);
    assert_eq!(row.total_sessions, 2);
    assert_eq!(row.total_duration_secs, 180);
    assert_eq!(row.unique_users, 2);
    assert!(row.avg_duration_secs > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_guild_voice_stats_window_respects_days() {
    let pool = pool().await;
    let repo = PgStatsRepository::new(pool.clone());
    let g = fresh_id();
    let channel_id = fresh_id();
    let u = fresh_id();
    // Session d'il y a 30 jours — hors fenetre 7 jours.
    let old = Utc::now() - chrono::Duration::days(30);
    sqlx::query(
        "INSERT INTO voice_sessions (guild_id, user_id, username, channel_id, channel_name, duration_secs, started_at, ended_at) \
         VALUES ($1, $2, 'Alice', $3, 'g', 60, $4, $5)"
    ).bind(&g).bind(&u).bind(&channel_id).bind(old).bind(old + chrono::Duration::minutes(1))
    .execute(&pool).await.unwrap();

    let stats = repo.get_guild_voice_stats(&g, 7, 10).await.unwrap();
    assert_eq!(stats.len(), 0);

    let stats = repo.get_guild_voice_stats(&g, 60, 10).await.unwrap();
    assert_eq!(stats.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn count_unique_voice_users_scoped_and_windowed() {
    let repo = PgStatsRepository::new(pool().await);
    let g = fresh_id();
    let ch = fresh_id();
    repo.save_voice_session(&g, &fresh_id(), "A", &ch, "g", 30)
        .await
        .unwrap();
    repo.save_voice_session(&g, &fresh_id(), "B", &ch, "g", 30)
        .await
        .unwrap();
    // Meme user deux fois -> compte 1.
    let dup = fresh_id();
    repo.save_voice_session(&g, &dup, "C", &ch, "g", 30)
        .await
        .unwrap();
    repo.save_voice_session(&g, &dup, "C", &ch, "g", 30)
        .await
        .unwrap();
    assert_eq!(repo.count_unique_voice_users(&g, 7).await.unwrap(), 3);
}
