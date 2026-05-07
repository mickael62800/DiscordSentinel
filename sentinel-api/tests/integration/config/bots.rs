//! Tests d'integration pour les configs per-guild : strike_config, conduct_config, level_config.

use sqlx::PgPool;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}

fn ugid() -> String { format!("{}", uuid::Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128) }

// ── Strike config ──

#[tokio::test]
async fn strike_config_defaults() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO strike_config (guild_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(&gid).execute(&p).await.unwrap();
    let row = sqlx::query_as::<_, (i64, bool)>("SELECT window_secs, enabled FROM strike_config WHERE guild_id = $1")
        .bind(&gid).fetch_one(&p).await.unwrap();
    assert_eq!(row.0, 3600);
    assert!(row.1);
}

#[tokio::test]
async fn strike_config_custom_thresholds() {
    let p = pool().await;
    let gid = ugid();
    let thresholds = serde_json::json!([
        {"points": 3, "action": "mute", "duration_secs": 600},
        {"points": 6, "action": "ban"}
    ]);
    sqlx::query("INSERT INTO strike_config (guild_id, thresholds, window_secs) VALUES ($1, $2, 7200)")
        .bind(&gid).bind(&thresholds).execute(&p).await.unwrap();
    let row = sqlx::query_as::<_, (i64, serde_json::Value)>("SELECT window_secs, thresholds FROM strike_config WHERE guild_id = $1")
        .bind(&gid).fetch_one(&p).await.unwrap();
    assert_eq!(row.0, 7200);
    assert_eq!(row.1.as_array().unwrap().len(), 2);
}

// ── Conduct config ──

#[tokio::test]
async fn conduct_config_defaults() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO conduct_config (guild_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(&gid).execute(&p).await.unwrap();
    let row = sqlx::query_as::<_, (i32, i32, i32, i32)>(
        "SELECT max_points, penalty_warn, penalty_mute, penalty_ban FROM conduct_config WHERE guild_id = $1",
    ).bind(&gid).fetch_one(&p).await.unwrap();
    assert_eq!(row.0, 12); // max_points
    assert_eq!(row.1, 1);  // penalty_warn
    assert_eq!(row.2, 3);  // penalty_mute
    assert_eq!(row.3, 6);  // penalty_ban
}

#[tokio::test]
async fn conduct_config_update() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO conduct_config (guild_id) VALUES ($1)").bind(&gid).execute(&p).await.unwrap();
    sqlx::query("UPDATE conduct_config SET max_points = 20, penalty_ban = 10 WHERE guild_id = $1")
        .bind(&gid).execute(&p).await.unwrap();
    let row = sqlx::query_as::<_, (i32, i32)>("SELECT max_points, penalty_ban FROM conduct_config WHERE guild_id = $1")
        .bind(&gid).fetch_one(&p).await.unwrap();
    assert_eq!(row.0, 20);
    assert_eq!(row.1, 10);
}

// ── Level config ──

#[tokio::test]
async fn level_config_defaults() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO level_config (guild_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(&gid).execute(&p).await.unwrap();
    let row = sqlx::query_as::<_, (i32, i32, i32, bool)>(
        "SELECT xp_per_message, xp_per_voice_minute, xp_cooldown_secs, enabled FROM level_config WHERE guild_id = $1",
    ).bind(&gid).fetch_one(&p).await.unwrap();
    assert_eq!(row.0, 15);
    assert_eq!(row.1, 5);
    assert_eq!(row.2, 60);
    assert!(row.3);
}

#[tokio::test]
async fn level_config_custom() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query(
        "INSERT INTO level_config (guild_id, xp_per_message, xp_cooldown_secs, level_up_message, enabled) VALUES ($1, 25, 30, 'Bravo {user} lvl {level}!', true)",
    ).bind(&gid).execute(&p).await.unwrap();
    let msg = sqlx::query_as::<_, (String,)>("SELECT level_up_message FROM level_config WHERE guild_id = $1")
        .bind(&gid).fetch_one(&p).await.unwrap().0;
    assert!(msg.contains("{user}"));
    assert!(msg.contains("{level}"));
}
