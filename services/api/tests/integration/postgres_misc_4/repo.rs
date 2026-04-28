//! Tests d'integration postgres pour 5 repos : guild, coude_heist,
//! coude_steal_protection, coude_steal_boost, blackjack_table.

use chrono::Duration;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::casino::blackjack_table_repository::PgBlackjackTableRepository;
use sentinel_api::adapters::outbound::postgres::coude::heist_repository::PgCoudeHeistRepository;
use sentinel_api::adapters::outbound::postgres::coude::steal_boost_repository::PgCoudeStealBoostRepository;
use sentinel_api::adapters::outbound::postgres::coude::steal_protection_repository::PgCoudeStealProtectionRepository;
use sentinel_api::adapters::outbound::postgres::system::guild_repository::PgGuildRepository;
use sentinel_api::domain::entities::system::guild::Guild;
use sentinel_api::ports::outbound::casino::blackjack_table_repository::BlackjackTableRepository;
use sentinel_api::ports::outbound::coude::heist_repository::CoudeHeistRepository;
use sentinel_api::ports::outbound::coude::steal_boost_repository::CoudeStealBoostRepository;
use sentinel_api::ports::outbound::coude::steal_protection_repository::CoudeStealProtectionRepository;
use sentinel_api::ports::outbound::system::guild_repository::GuildRepository;
async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

// ══════════════════════════════════════════════════════════
// Guild
// ══════════════════════════════════════════════════════════

fn sample_guild(id: &str, name: &str) -> Guild {
    let now = Utc::now();
    Guild {
        guild_id: id.into(), name: name.into(),
        icon: Some("icon-hash".into()),
        member_count: 42,
        registered_at: now, updated_at: now,
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
    assert!(all.iter().any(|g| g.guild_id == id));
}

// ══════════════════════════════════════════════════════════
// CoudeHeist
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn heist_last_attempt_none_when_absent() {
    let repo = PgCoudeHeistRepository::new(pool().await);
    assert!(repo.last_attempt(&fresh_id(), &fresh_id()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn heist_record_attempt_and_last() {
    let repo = PgCoudeHeistRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    let tools = vec!["crowbar".to_string(), "mask".to_string()];
    let a = repo.record_attempt(&g, &u, true, 500, 65, &tools).await.unwrap();
    assert!(a.success);
    assert_eq!(a.amount_stolen, 500);
    assert_eq!(a.chance_percent, 65);
    assert_eq!(a.tools_used.len(), 2);
    let last = repo.last_attempt(&g, &u).await.unwrap().unwrap();
    assert_eq!(last.id, a.id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn heist_prison_lifecycle() {
    let repo = PgCoudeHeistRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    assert!(repo.get_prison(&g, &u).await.unwrap().is_none());

    let released = Utc::now() + Duration::hours(4);
    repo.send_to_prison(&g, &u, released, "caught").await.unwrap();
    let state = repo.get_prison(&g, &u).await.unwrap().unwrap();
    assert_eq!(state.reason, "caught");
    assert!(state.is_active());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn heist_send_to_prison_is_upsert() {
    let repo = PgCoudeHeistRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    let t1 = Utc::now() + Duration::hours(1);
    let t2 = Utc::now() + Duration::hours(10);
    repo.send_to_prison(&g, &u, t1, "first").await.unwrap();
    repo.send_to_prison(&g, &u, t2, "second").await.unwrap();
    let state = repo.get_prison(&g, &u).await.unwrap().unwrap();
    assert_eq!(state.reason, "second");
}

// ══════════════════════════════════════════════════════════
// CoudeStealProtection
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protection_list_active_empty() {
    let repo = PgCoudeStealProtectionRepository::new(pool().await);
    assert!(repo.list_active(&fresh_id(), &fresh_id()).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protection_upsert_adds_days() {
    let repo = PgCoudeStealProtectionRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    let first = repo.upsert(&g, &u, "alarm", 3).await.unwrap();
    let protection = repo.list_active(&g, &u).await.unwrap();
    assert_eq!(protection.len(), 1);
    assert_eq!(protection[0].item_key, "alarm");
    // Cumul : +2 jours
    let extended = repo.upsert(&g, &u, "alarm", 2).await.unwrap();
    assert!(extended > first, "extension should push expires_at further");
}

// ══════════════════════════════════════════════════════════
// CoudeStealBoost
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn boost_list_active_empty() {
    let repo = PgCoudeStealBoostRepository::new(pool().await);
    assert!(repo.list_active(&fresh_id(), &fresh_id()).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn boost_upsert_and_list() {
    let repo = PgCoudeStealBoostRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.upsert(&g, &u, "lockpick", 7).await.unwrap();
    let boosts = repo.list_active(&g, &u).await.unwrap();
    assert_eq!(boosts.len(), 1);
    assert_eq!(boosts[0].item_key, "lockpick");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn boost_purge_expired_does_not_panic() {
    let repo = PgCoudeStealBoostRepository::new(pool().await);
    let _ = repo.purge_expired().await.unwrap();
}

// ══════════════════════════════════════════════════════════
// BlackjackTable
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bj_table_create_and_status() {
    let repo = PgBlackjackTableRepository::new(pool().await);
    let g = fresh_id();
    let deck = serde_json::json!([{"rank":"A","suit":"spades"}]);
    let ch = format!("s-{:08x}", Uuid::new_v4().as_u128() as u32);
    let table = repo.create(&g, &ch, "owner1", "Owner", &deck).await.unwrap();
    let sg = repo.get_status_and_guild(&table.id).await.unwrap().unwrap();
    assert_eq!(sg.1, g);
    let gid = repo.get_guild_id(&table.id).await.unwrap().unwrap();
    assert_eq!(gid, g);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bj_table_add_player_and_list() {
    let repo = PgBlackjackTableRepository::new(pool().await);
    let g = fresh_id();
    let deck = serde_json::json!([]);
    let ch = format!("c-{:08x}", Uuid::new_v4().as_u128() as u32);
    let table = repo.create(&g, &ch, "o", "Owner", &deck).await.unwrap();
    repo.add_player(&table.id, "u1", "Alice").await.unwrap();
    repo.add_player(&table.id, "u2", "Bob").await.unwrap();
    // L'owner peut aussi compter comme joueur — on verifie >= 2.
    assert!(repo.count_players(&table.id).await.unwrap() >= 2);
    assert!(repo.list_players(&table.id).await.unwrap().len() >= 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bj_table_find_open_by_channel() {
    let repo = PgBlackjackTableRepository::new(pool().await);
    let g = fresh_id();
    let ch = format!("ch-{:08x}", Uuid::new_v4().as_u128() as u32);
    let deck = serde_json::json!([]);
    repo.create(&g, &ch, "o", "O", &deck).await.unwrap();
    let found = repo.find_open_by_channel(&ch).await.unwrap();
    assert!(found.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bj_table_close_marks_closed() {
    let repo = PgBlackjackTableRepository::new(pool().await);
    let g = fresh_id();
    let ch = format!("c-{:08x}", Uuid::new_v4().as_u128() as u32);
    let deck = serde_json::json!([]);
    let table = repo.create(&g, &ch, "o", "O", &deck).await.unwrap();
    repo.close(&table.id).await.unwrap();
    // Plus visible comme "open by channel"
    assert!(repo.find_open_by_channel(&ch).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bj_table_touch_activity_does_not_panic() {
    let repo = PgBlackjackTableRepository::new(pool().await);
    let g = fresh_id();
    let deck = serde_json::json!([]);
    let ch = format!("t-{:08x}", Uuid::new_v4().as_u128() as u32);
    let table = repo.create(&g, &ch, "o", "O", &deck).await.unwrap();
    repo.touch_activity(&table.id).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bj_table_list_games_empty_initially() {
    let repo = PgBlackjackTableRepository::new(pool().await);
    let g = fresh_id();
    let deck = serde_json::json!([]);
    let ch = format!("lg-{:08x}", Uuid::new_v4().as_u128() as u32);
    let table = repo.create(&g, &ch, "o", "O", &deck).await.unwrap();
    let games = repo.list_games(&table.id).await.unwrap();
    assert!(games.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bj_table_unknown_id_returns_none() {
    let repo = PgBlackjackTableRepository::new(pool().await);
    let bogus = Uuid::new_v4().to_string();
    assert!(repo.get_status_and_guild(&bogus).await.unwrap().is_none());
    assert!(repo.get_guild_id(&bogus).await.unwrap().is_none());
}
