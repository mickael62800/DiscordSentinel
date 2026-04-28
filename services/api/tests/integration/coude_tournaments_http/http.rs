//! Tests d'integration HTTP pour les endpoints coude/tournaments.
//! Utilisent la vraie DB (sqlx direct dans le handler).

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::domain::entities::coude::tournament::current_week_bounds;

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (s, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

fn state() -> sentinel_api::adapters::inbound::http::state::AppState {
    test_helpers::build_test_state(Arc::new(test_helpers::StubVoiceChannels))
}

async fn pool() -> sqlx::PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    sqlx::PgPool::connect(&url).await.unwrap()
}

// Guild isole par test — max 20 chars (VARCHAR(20) sur les Discord IDs).
fn fresh_guild_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

// ── current ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn current_tournament_empty_guild_returns_empty_standings() {
    let app = router::build_for_test(state());
    let g = fresh_guild_id();
    let (s, j) = get(app, &format!("/api/coude/{g}/tournaments/current")).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["guild_id"], g);
    assert!(j["standings"].as_array().unwrap().is_empty());
    // Pas de cashbox -> prize_pool_estimated = 0
    assert_eq!(j["prize_pool_estimated"], 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn current_tournament_standings_ranked_by_net_gain() {
    let pool = pool().await;
    let g = fresh_guild_id();
    let (ws, _we) = current_week_bounds();
    let mid_week = ws + chrono::Duration::hours(6);

    // Seed user_wallets pour avoir des usernames.
    for (uid, name) in [("u1", "Alice"), ("u2", "Bob"), ("u3", "Carol")] {
        sqlx::query("INSERT INTO user_wallets (guild_id, user_id, username, coins) VALUES ($1, $2, $3, 0)")
            .bind(&g).bind(uid).bind(name).execute(&pool).await.unwrap();
    }
    // u1: +500, u2: +200, u3: -100 -> ordre attendu u1, u2, u3.
    for (uid, amt) in [("u1", 500i64), ("u1", -100), ("u2", 200), ("u3", -100)] {
        sqlx::query("INSERT INTO wallet_transactions \
            (guild_id, user_id, amount, balance_after, source, created_at) \
            VALUES ($1, $2, $3, 0, 'test', $4)")
            .bind(&g).bind(uid).bind(amt).bind(mid_week)
            .execute(&pool).await.unwrap();
    }
    // Cashbox : prize_pool = 10% * 10000 = 1000.
    sqlx::query("INSERT INTO coude_cashbox (guild_id, balance) VALUES ($1, 10000) ON CONFLICT (guild_id) DO UPDATE SET balance = EXCLUDED.balance")
        .bind(&g).execute(&pool).await.unwrap();

    let app = router::build_for_test(state());
    let (s, j) = get(app, &format!("/api/coude/{g}/tournaments/current")).await;
    assert_eq!(s, StatusCode::OK);
    let standings = j["standings"].as_array().unwrap();
    assert_eq!(standings.len(), 3);
    assert_eq!(standings[0]["user_id"], "u1");
    assert_eq!(standings[0]["username"], "Alice");
    assert_eq!(standings[0]["net_gain"], 400);
    assert_eq!(standings[0]["rank"], 1);
    assert_eq!(standings[1]["user_id"], "u2");
    assert_eq!(standings[1]["rank"], 2);
    assert_eq!(standings[2]["user_id"], "u3");
    assert_eq!(j["prize_pool_estimated"], 1000);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn current_tournament_falls_back_to_question_mark_when_no_wallet() {
    let pool = pool().await;
    let g = fresh_guild_id();
    let (ws, _we) = current_week_bounds();
    sqlx::query("INSERT INTO wallet_transactions \
        (guild_id, user_id, amount, balance_after, source, created_at) \
        VALUES ($1, 'stranger', 100, 0, 'test', $2)")
        .bind(&g).bind(ws + chrono::Duration::hours(1))
        .execute(&pool).await.unwrap();

    let app = router::build_for_test(state());
    let (_, j) = get(app, &format!("/api/coude/{g}/tournaments/current")).await;
    let standings = j["standings"].as_array().unwrap();
    assert_eq!(standings[0]["username"], "?");
}

// ── history ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tournament_history_empty_guild_returns_empty_array() {
    let app = router::build_for_test(state());
    let (s, j) = get(app, &format!("/api/coude/{}/tournaments/history", fresh_guild_id())).await;
    assert_eq!(s, StatusCode::OK);
    assert!(j.as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tournament_history_returns_seeded_rows_ordered_desc() {
    let pool = pool().await;
    let g = fresh_guild_id();
    let now = chrono::Utc::now();
    for (offset_weeks, status, prize) in [(4i64, "resolved", 500i64), (2, "resolved", 800), (1, "ongoing", 0)] {
        let ws = now - chrono::Duration::weeks(offset_weeks);
        sqlx::query(
            "INSERT INTO coude_weekly_tournaments \
             (guild_id, week_start, week_end, winner_user_id, winner_username, winner_net_gain, prize_amount, status) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(&g).bind(ws).bind(ws + chrono::Duration::weeks(1))
        .bind(Some("w")).bind(Some("Winner"))
        .bind(1000i64).bind(prize).bind(status)
        .execute(&pool).await.unwrap();
    }

    let app = router::build_for_test(state());
    let (s, j) = get(app, &format!("/api/coude/{g}/tournaments/history")).await;
    assert_eq!(s, StatusCode::OK);
    let arr = j.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    // ORDER BY week_start DESC -> ongoing (offset 1) en premier
    assert_eq!(arr[0]["status"], "ongoing");
    assert_eq!(arr[1]["status"], "resolved");
    assert_eq!(arr[1]["prize_amount"], 800);
    assert_eq!(arr[2]["prize_amount"], 500);
    assert_eq!(arr[0]["winner_username"], "Winner");
}
