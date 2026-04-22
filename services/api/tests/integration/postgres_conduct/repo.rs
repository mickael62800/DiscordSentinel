//! Tests d'integration postgres pour PgConductRepository.

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::PgConductRepository;
use sentinel_api::domain::entities::{ConductConfig, ConductPointsLog, UserConductPoints};
use sentinel_api::ports::outbound::ConductRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

fn sample_config(guild: &str) -> ConductConfig {
    let now = Utc::now();
    ConductConfig {
        guild_id: guild.into(),
        max_points: 100, regen_amount: 5, regen_interval: "weekly".into(),
        penalty_warn: 5, penalty_delete: 10, penalty_mute: 20, penalty_ban: 50,
        created_at: now, updated_at: now,
    }
}
fn sample_points(guild: &str, user: &str, points: i32) -> UserConductPoints {
    let now = Utc::now();
    UserConductPoints {
        id: Uuid::new_v4(), guild_id: guild.into(), user_id: user.into(),
        username: user.into(), points,
        last_regen_at: now, created_at: now, updated_at: now,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_get_none_when_absent() {
    let repo = PgConductRepository::new(pool().await);
    assert!(repo.get_config(&fresh_id()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_save_and_get_roundtrip() {
    let repo = PgConductRepository::new(pool().await);
    let g = fresh_id();
    repo.save_config(&sample_config(&g)).await.unwrap();
    let got = repo.get_config(&g).await.unwrap().unwrap();
    assert_eq!(got.max_points, 100);
    assert_eq!(got.regen_interval, "weekly");
    assert_eq!(got.penalty_ban, 50);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_save_is_upsert_on_guild_id() {
    let repo = PgConductRepository::new(pool().await);
    let g = fresh_id();
    repo.save_config(&sample_config(&g)).await.unwrap();
    let mut cfg = sample_config(&g);
    cfg.max_points = 500;
    repo.save_config(&cfg).await.unwrap();
    let got = repo.get_config(&g).await.unwrap().unwrap();
    assert_eq!(got.max_points, 500);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn points_get_none_when_absent() {
    let repo = PgConductRepository::new(pool().await);
    assert!(repo.get_points(&fresh_id(), &fresh_id()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn points_save_and_get_roundtrip() {
    let repo = PgConductRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.save_points(&sample_points(&g, &u, 80)).await.unwrap();
    let got = repo.get_points(&g, &u).await.unwrap().unwrap();
    assert_eq!(got.points, 80);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn points_save_is_upsert() {
    let repo = PgConductRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.save_points(&sample_points(&g, &u, 100)).await.unwrap();
    repo.save_points(&sample_points(&g, &u, 42)).await.unwrap();
    assert_eq!(repo.get_points(&g, &u).await.unwrap().unwrap().points, 42);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_points_changes_value() {
    let repo = PgConductRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.save_points(&sample_points(&g, &u, 100)).await.unwrap();
    repo.update_points(&g, &u, 50).await.unwrap();
    assert_eq!(repo.get_points(&g, &u).await.unwrap().unwrap().points, 50);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_ordered_points_desc() {
    let repo = PgConductRepository::new(pool().await);
    let g = fresh_id();
    for (u, pts) in [("u1", 30), ("u2", 90), ("u3", 60)] {
        let user_id = format!("{}{}", fresh_id(), u);
        let user_id = &user_id[..18.min(user_id.len())];
        repo.save_points(&sample_points(&g, user_id, pts)).await.unwrap();
    }
    let board = repo.get_leaderboard(&g, 10).await.unwrap();
    assert_eq!(board.len(), 3);
    assert_eq!(board[0].points, 90);
    assert_eq!(board[1].points, 60);
    assert_eq!(board[2].points, 30);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_points_removes_row() {
    let repo = PgConductRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    repo.save_points(&sample_points(&g, &u, 50)).await.unwrap();
    repo.delete_points(&g, &u).await.unwrap();
    assert!(repo.get_points(&g, &u).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_regen_timestamp_moves_last_regen_at_forward() {
    let repo = PgConductRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    let mut p = sample_points(&g, &u, 100);
    p.last_regen_at = Utc::now() - chrono::Duration::days(30);
    repo.save_points(&p).await.unwrap();
    let before = repo.get_points(&g, &u).await.unwrap().unwrap().last_regen_at;
    repo.update_regen_timestamp(&g, &u).await.unwrap();
    let after = repo.get_points(&g, &u).await.unwrap().unwrap().last_regen_at;
    assert!(after > before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_save_and_get_ordered_desc() {
    let repo = PgConductRepository::new(pool().await);
    let g = fresh_id(); let u = fresh_id();
    let now = Utc::now();
    repo.save_log(&ConductPointsLog {
        id: Uuid::new_v4(), guild_id: g.clone(), user_id: u.clone(),
        delta: -5, reason: "warn".into(), points_before: 100, points_after: 95,
        created_at: now - chrono::Duration::hours(2),
    }).await.unwrap();
    repo.save_log(&ConductPointsLog {
        id: Uuid::new_v4(), guild_id: g.clone(), user_id: u.clone(),
        delta: -10, reason: "delete".into(), points_before: 95, points_after: 85,
        created_at: now - chrono::Duration::hours(1),
    }).await.unwrap();
    let logs = repo.get_log(&g, &u, 10).await.unwrap();
    assert_eq!(logs.len(), 2);
    // DESC : delete en premier.
    assert_eq!(logs[0].reason, "delete");
    assert_eq!(logs[1].reason, "warn");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_users_needing_regen_falls_back_to_7_days_on_unknown() {
    // Test uniquement que le fallback ne panique pas (le query tourne sans resultat).
    let repo = PgConductRepository::new(pool().await);
    let users = repo.find_users_needing_regen("garbage").await.unwrap();
    assert_eq!(users.len(), 0);
}
