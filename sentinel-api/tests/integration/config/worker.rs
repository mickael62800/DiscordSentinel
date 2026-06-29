//! Tests d'integration pour la config DB des workers.
//! Verifie que les workers peuvent lire leur config depuis bot_guild_config.

use sqlx::PgPool;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}

fn short_gid() -> String {
    use rand::Rng;
    format!(
        "{}",
        rand::thread_rng().gen_range(10000000000000000u64..99999999999999999u64)
    )
}

// ══════════════════════════════════════════════════════════
//  Worker config loading from DB
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn worker_config_saved_and_readable() {
    let p = pool().await;
    let gid = short_gid();

    // Simuler ce que l'app bureau fait : sauver une config worker
    sqlx::query(
        "INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value) VALUES ($1, 'analytics-worker', 'daily_snapshot_interval', '2')",
    ).bind(&gid).execute(&p).await.unwrap();

    // Simuler ce que le worker fait : lire la config
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT config_key, config_value FROM bot_guild_config WHERE bot_name = 'analytics-worker' AND guild_id = $1",
    ).bind(&gid).fetch_all(&p).await.unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "daily_snapshot_interval");
    assert_eq!(rows[0].1, "2");
    let parsed: u64 = rows[0].1.parse().unwrap();
    assert_eq!(parsed, 2);
}

#[tokio::test]
async fn worker_config_multiple_keys() {
    let p = pool().await;
    let gid = short_gid();

    for (key, val) in &[
        ("strike_decay_interval", "2"),
        ("ban_cleanup_interval", "5"),
        ("send_reminders_interval", "60"),
    ] {
        sqlx::query(
            "INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value) VALUES ($1, 'moderation-worker', $2, $3)",
        ).bind(&gid).bind(key).bind(val).execute(&p).await.unwrap();
    }

    let count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM bot_guild_config WHERE guild_id = $1 AND bot_name = 'moderation-worker'",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;

    assert_eq!(count, 3);
}

#[tokio::test]
async fn worker_config_update_via_upsert() {
    let p = pool().await;
    let gid = short_gid();

    sqlx::query(
        "INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value) VALUES ($1, 'cache-worker', 'analytics_cache_refresh', '300')",
    ).bind(&gid).execute(&p).await.unwrap();

    // Update via upsert (comme l'app bureau fait)
    sqlx::query(
        r#"INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
           VALUES ($1, 'cache-worker', 'analytics_cache_refresh', '120')
           ON CONFLICT (guild_id, bot_name, config_key) DO UPDATE SET config_value = EXCLUDED.config_value, updated_at = NOW()"#,
    ).bind(&gid).execute(&p).await.unwrap();

    let val = sqlx::query_as::<_, (String,)>(
        "SELECT config_value FROM bot_guild_config WHERE guild_id = $1 AND config_key = 'analytics_cache_refresh'",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;

    assert_eq!(val, "120", "La valeur doit etre mise a jour");
}

#[tokio::test]
async fn worker_config_isolated_per_worker() {
    let p = pool().await;
    let gid = short_gid();

    sqlx::query(
        "INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value) VALUES ($1, 'cleanup-worker', 'logs_retention_days', '14')",
    ).bind(&gid).execute(&p).await.unwrap();

    // Le moderation-worker ne doit pas voir cette config
    let count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM bot_guild_config WHERE guild_id = $1 AND bot_name = 'moderation-worker'",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;

    assert_eq!(count, 0);
}

#[tokio::test]
async fn worker_config_all_workers_configurable() {
    let p = pool().await;
    let gid = short_gid();

    let workers = vec![
        ("analytics-worker", "daily_snapshot_interval", "3"),
        ("moderation-worker", "strike_decay_interval", "2"),
        ("cache-worker", "dashboard_cache_refresh", "900"),
        ("cleanup-worker", "voice_sessions_retention_days", "60"),
        ("coude-worker", "combat_expiry_check_secs", "43200"),
    ];

    for (worker, key, val) in &workers {
        sqlx::query(
            "INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value) VALUES ($1, $2, $3, $4)",
        ).bind(&gid).bind(worker).bind(key).bind(val).execute(&p).await.unwrap();
    }

    let total =
        sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM bot_guild_config WHERE guild_id = $1")
            .bind(&gid)
            .fetch_one(&p)
            .await
            .unwrap()
            .0;

    assert_eq!(total, 5, "5 workers doivent avoir leur config");
}

#[tokio::test]
async fn worker_config_enabled_toggle() {
    let p = pool().await;
    let gid = short_gid();

    // Desactiver un worker
    sqlx::query(
        "INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value) VALUES ($1, 'cleanup-worker', 'enabled', 'false')",
    ).bind(&gid).execute(&p).await.unwrap();

    let val = sqlx::query_as::<_, (String,)>(
        "SELECT config_value FROM bot_guild_config WHERE guild_id = $1 AND bot_name = 'cleanup-worker' AND config_key = 'enabled'",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;

    assert_eq!(val, "false");
    // Le worker common is_worker_enabled() retourne false pour "false"
    assert!(val != "true" && val != "1");
}

#[tokio::test]
async fn worker_definitions_seeded() {
    let p = pool().await;

    let workers = sqlx::query_as::<_, (String,)>(
        "SELECT bot_name FROM bot_definitions WHERE bot_name LIKE '%-worker' ORDER BY bot_name",
    )
    .fetch_all(&p)
    .await
    .unwrap();

    // Au moins 5 worker definitions doivent exister
    assert!(
        workers.len() >= 5,
        "Expected >= 5 worker definitions, got {}",
        workers.len()
    );

    let names: Vec<&str> = workers.iter().map(|w| w.0.as_str()).collect();
    assert!(names.contains(&"analytics-worker"));
    assert!(names.contains(&"moderation-worker"));
    assert!(names.contains(&"cache-worker"));
    assert!(names.contains(&"cleanup-worker"));
}
