//! Tests d'integration pour ManageStatsService — exerce les chemins
//! Redis (count_services, get_dashboard_stats) et sqlx (record_messages,
//! record_voice, get_user_stats, get_guild_overview, get_leaderboard,
//! get_guild_voice_stats) avec vrais Postgres + Redis.

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::moderation::infraction_repository::PgInfractionRepository;
use sentinel_api::adapters::outbound::postgres::audit::stats_repository::PgStatsRepository;
use sentinel_api::application::audit::manage_stats_service::ManageStatsService;
use sentinel_core::domain::entities::system::rule::Rule;
use sentinel_core::domain::errors::DomainError;
use sentinel_api::ports::inbound::audit::manage_stats::ManageStatsUseCase;
use sentinel_api::ports::inbound::audit::manage_stats::RecordMessagesCommand;
use sentinel_api::ports::inbound::audit::manage_stats::RecordVoiceCommand;
use sentinel_api::ports::outbound::system::cache::CachePort;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}

fn redis_client() -> redis::Client {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6380".into());
    redis::Client::open(url).unwrap()
}

fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

struct NoopCache;
#[async_trait]
impl CachePort for NoopCache {
    async fn get_rules(&self, _: &str) -> Result<Option<Vec<Rule>>, DomainError> { Ok(None) }
    async fn set_rules(&self, _: &str, _: &[Rule]) -> Result<(), DomainError> { Ok(()) }
    async fn invalidate_rules(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn get_json(&self, _: &str) -> Result<Option<String>, DomainError> { Ok(None) }
    async fn set_json(&self, _: &str, _: &str, _: u64) -> Result<(), DomainError> { Ok(()) }
    async fn invalidate(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn invalidate_pattern(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
}

async fn build() -> ManageStatsService {
    let p = pool().await;
    let stats = Arc::new(PgStatsRepository::new(p.clone()));
    let inf = Arc::new(PgInfractionRepository::new(p));
    ManageStatsService::new(stats, inf, Arc::new(NoopCache), redis_client())
}

// ── record_messages / record_voice ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_messages_increments_count() {
    let svc = build().await;
    let g = fresh_id();
    let u = fresh_id();
    svc.record_messages(RecordMessagesCommand {
        guild_id: g.clone().into(), user_id: u.clone().into(), username: "Alice".into(), count: 5,
    }).await.unwrap();
    let stats = svc.get_user_stats(&g, &u).await.unwrap().unwrap();
    assert_eq!(stats.message_count, 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_voice_accumulates_seconds_with_session() {
    let svc = build().await;
    let g = fresh_id();
    let u = fresh_id();
    svc.record_voice(RecordVoiceCommand {
        guild_id: g.clone().into(), user_id: u.clone().into(), username: "Alice".into(),
        channel_id: "c1".into(), channel_name: "Voice 1".into(),
        seconds: 120,
    }).await.unwrap();
    let stats = svc.get_user_stats(&g, &u).await.unwrap().unwrap();
    assert_eq!(stats.voice_seconds, 120);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn record_voice_without_channel_id_skips_session() {
    let svc = build().await;
    let g = fresh_id();
    let u = fresh_id();
    // channel_id vide -> skip save_voice_session
    svc.record_voice(RecordVoiceCommand {
        guild_id: g.clone().into(), user_id: u.clone().into(), username: "Alice".into(),
        channel_id: "".into(), channel_name: "".into(),
        seconds: 60,
    }).await.unwrap();
    let stats = svc.get_user_stats(&g, &u).await.unwrap().unwrap();
    assert_eq!(stats.voice_seconds, 60);
}

// ── get_user_stats / get_leaderboard / get_guild_overview ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_user_stats_none_for_unknown_user() {
    let svc = build().await;
    assert!(svc.get_user_stats(&fresh_id(), &fresh_id()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_leaderboard_returns_users() {
    let svc = build().await;
    let g = fresh_id();
    svc.record_messages(RecordMessagesCommand {
        guild_id: g.clone().into(), user_id: fresh_id(), username: "A".into(), count: 10,
    }).await.unwrap();
    svc.record_messages(RecordMessagesCommand {
        guild_id: g.clone().into(), user_id: fresh_id(), username: "B".into(), count: 20,
    }).await.unwrap();
    let lb = svc.get_leaderboard(&g, 10).await.unwrap();
    assert_eq!(lb.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_guild_overview_aggregates_stats() {
    let svc = build().await;
    let g = fresh_id();
    svc.record_messages(RecordMessagesCommand {
        guild_id: g.clone().into(), user_id: fresh_id(), username: "A".into(), count: 10,
    }).await.unwrap();
    svc.record_voice(RecordVoiceCommand {
        guild_id: g.clone().into(), user_id: fresh_id(), username: "B".into(),
        channel_id: "c1".into(), channel_name: "v".into(), seconds: 300,
    }).await.unwrap();
    let overview = svc.get_guild_overview(&g).await.unwrap();
    assert_eq!(overview.total_messages, 10);
    assert_eq!(overview.total_voice_seconds, 300);
    assert_eq!(overview.active_members, 2);
    assert_eq!(overview.total_infractions, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_guild_overview_empty_guild() {
    let svc = build().await;
    let o = svc.get_guild_overview(&fresh_id()).await.unwrap();
    assert_eq!(o.total_messages, 0);
    assert_eq!(o.active_members, 0);
    assert_eq!(o.total_infractions, 0);
}

// ── get_guild_voice_stats ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_guild_voice_stats_empty_returns_zeros() {
    let svc = build().await;
    let g = fresh_id();
    let s = svc.get_guild_voice_stats(&g, 7, 20).await.unwrap();
    assert_eq!(s.total_channels, 0);
    assert_eq!(s.total_sessions, 0);
    assert_eq!(s.total_duration_secs, 0);
    assert_eq!(s.avg_session_secs, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_guild_voice_stats_with_sessions() {
    let svc = build().await;
    let g = fresh_id();
    for i in 0..3 {
        svc.record_voice(RecordVoiceCommand {
            guild_id: g.clone().into(), user_id: fresh_id(), username: format!("u{i}"),
            channel_id: format!("c{i}"), channel_name: format!("ch{i}"),
            seconds: 100 * (i as u64 + 1),
        }).await.unwrap();
    }
    let s = svc.get_guild_voice_stats(&g, 7, 20).await.unwrap();
    assert!(s.total_sessions >= 3);
    assert!(s.total_duration_secs >= 600);
    assert!(s.unique_users >= 3);
}

// ── get_dashboard_stats (Redis + PG) ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_dashboard_stats_with_redis_available() {
    use redis::AsyncCommands;
    // Seed un bot ET un worker pour exercer les 2 branches de is_worker_service.
    let client = redis_client();
    let mut conn = client.get_multiplexed_async_connection().await.unwrap();
    let bot_online = format!("test-bot-online-{}", fresh_id());
    let bot_offline = format!("test-bot-offline-{}", fresh_id());
    let worker_online = format!("test-worker-online-{}", fresh_id());
    let worker_offline = format!("test-worker-offline-{}", fresh_id());
    let _: () = conn.sadd::<_, _, ()>("bots:known", &bot_online).await.unwrap();
    let _: () = conn.sadd::<_, _, ()>("bots:known", &bot_offline).await.unwrap();
    let _: () = conn.sadd::<_, _, ()>("bots:known", &worker_online).await.unwrap();
    let _: () = conn.sadd::<_, _, ()>("bots:known", &worker_offline).await.unwrap();
    let _: () = conn.set_ex::<_, _, ()>(format!("bot:online:{bot_online}"), "1", 60).await.unwrap();
    let _: () = conn.set_ex::<_, _, ()>(format!("bot:online:{worker_online}"), "1", 60).await.unwrap();
    // Pour couvrir la branche 'ping_test' du health check, on seed la cle.
    let _: () = conn.set_ex::<_, _, ()>("ping_test", "pong", 60).await.unwrap();

    let svc = build().await;
    let stats = svc.get_dashboard_stats().await.unwrap();
    assert!(stats.postgres_online);
    assert!(stats.redis_online);
    assert!(stats.bots_total >= 2);
    assert!(stats.workers_total >= 2);
    assert!(stats.bots_online >= 1);
    assert!(stats.workers_online >= 1);

    // Cleanup
    let _: () = conn.srem::<_, _, ()>("bots:known", &bot_online).await.unwrap();
    let _: () = conn.srem::<_, _, ()>("bots:known", &bot_offline).await.unwrap();
    let _: () = conn.srem::<_, _, ()>("bots:known", &worker_online).await.unwrap();
    let _: () = conn.srem::<_, _, ()>("bots:known", &worker_offline).await.unwrap();
    let _: () = conn.del::<_, ()>(format!("bot:online:{bot_online}")).await.unwrap();
    let _: () = conn.del::<_, ()>(format!("bot:online:{worker_online}")).await.unwrap();
}
