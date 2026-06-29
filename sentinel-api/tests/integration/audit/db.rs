//! Tests d'integration REELS pour le systeme audit (avec PostgreSQL).
//! Couvre : audit_logs, security_events, logs, daily_activity, guilds.

use sqlx::PgPool;

async fn setup_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url)
        .await
        .expect("Impossible de se connecter a la base de test")
}

fn unique_guild() -> String {
    format!(
        "{}",
        uuid::Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    )
}

fn short_guild_id() -> String {
    use rand::Rng;
    format!(
        "{}",
        rand::thread_rng().gen_range(10000000000000000u64..99999999999999999u64)
    )
}

// ══════════════════════════════════════════════════════════
//  Audit logs — CRUD + filtrage
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn audit_log_insert_and_read() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    let details = serde_json::json!({"old_name": "general", "new_name": "discussion"});

    sqlx::query(
        r#"INSERT INTO audit_logs (guild_id, event_type, actor_id, actor_name, target_id, channel_id, channel_name, details)
           VALUES ($1, 'channel_update', '333333333333333333', 'Admin', '555555555555555555', '555555555555555555', 'general', $2)"#,
    ).bind(&gid).bind(&details).execute(&pool).await.unwrap();

    let row = sqlx::query_as::<_, (String, serde_json::Value)>(
        "SELECT event_type, details FROM audit_logs WHERE guild_id = $1",
    )
    .bind(&gid)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.0, "channel_update");
    assert_eq!(row.1["old_name"], "general");
    assert_eq!(row.1["new_name"], "discussion");
}

#[tokio::test]
async fn audit_logs_filter_by_event_type() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    for event in &[
        "message_delete",
        "message_delete",
        "role_create",
        "member_ban",
    ] {
        sqlx::query("INSERT INTO audit_logs (guild_id, event_type, details) VALUES ($1, $2, '{}')")
            .bind(&gid)
            .bind(event)
            .execute(&pool)
            .await
            .unwrap();
    }

    let deletes = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM audit_logs WHERE guild_id = $1 AND event_type = 'message_delete'",
    )
    .bind(&gid)
    .fetch_one(&pool)
    .await
    .unwrap()
    .0;

    assert_eq!(deletes, 2);
}

#[tokio::test]
async fn audit_logs_filter_by_actor() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    sqlx::query(
        "INSERT INTO audit_logs (guild_id, event_type, actor_id, actor_name, details) VALUES ($1, 'member_kick', '111', 'Mod1', '{}')",
    ).bind(&gid).execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO audit_logs (guild_id, event_type, actor_id, actor_name, details) VALUES ($1, 'member_kick', '222', 'Mod2', '{}')",
    ).bind(&gid).execute(&pool).await.unwrap();

    let mod1_count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM audit_logs WHERE guild_id = $1 AND actor_id = '111'",
    )
    .bind(&gid)
    .fetch_one(&pool)
    .await
    .unwrap()
    .0;

    assert_eq!(mod1_count, 1);
}

#[tokio::test]
async fn audit_logs_filter_by_target() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    sqlx::query(
        "INSERT INTO audit_logs (guild_id, event_type, target_id, target_name, details) VALUES ($1, 'member_ban', '444', 'BadUser', '{}')",
    ).bind(&gid).execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO audit_logs (guild_id, event_type, target_id, target_name, details) VALUES ($1, 'member_warn', '444', 'BadUser', '{}')",
    ).bind(&gid).execute(&pool).await.unwrap();

    let target_events = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM audit_logs WHERE guild_id = $1 AND target_id = '444'",
    )
    .bind(&gid)
    .fetch_one(&pool)
    .await
    .unwrap()
    .0;

    assert_eq!(target_events, 2);
}

#[tokio::test]
async fn audit_logs_ordered_desc() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    for event in &["first", "second", "third"] {
        sqlx::query("INSERT INTO audit_logs (guild_id, event_type, details) VALUES ($1, $2, '{}')")
            .bind(&gid)
            .bind(event)
            .execute(&pool)
            .await
            .unwrap();
        // Petit delai pour ordre garanti
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    let events = sqlx::query_as::<_, (String,)>(
        "SELECT event_type FROM audit_logs WHERE guild_id = $1 ORDER BY created_at DESC",
    )
    .bind(&gid)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(events[0].0, "third");
    assert_eq!(events[2].0, "first");
}

#[tokio::test]
async fn audit_logs_guild_isolation() {
    let pool = setup_pool().await;
    let gid1 = unique_guild();
    let gid2 = unique_guild();

    sqlx::query("INSERT INTO audit_logs (guild_id, event_type, details) VALUES ($1, 'test', '{}')")
        .bind(&gid1)
        .execute(&pool)
        .await
        .unwrap();

    let count = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM audit_logs WHERE guild_id = $1")
        .bind(&gid2)
        .fetch_one(&pool)
        .await
        .unwrap()
        .0;

    assert_eq!(count, 0);
}

// ══════════════════════════════════════════════════════════
//  Security events
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn security_event_with_user_ids() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    let user_ids = serde_json::json!(["111", "222", "333"]);

    sqlx::query(
        r#"INSERT INTO security_events (id, guild_id, event_type, severity, description, user_ids)
           VALUES (gen_random_uuid(), $1, 'raid_detected', 'critical', '3 joins en 2s', $2)"#,
    )
    .bind(&gid)
    .bind(&user_ids)
    .execute(&pool)
    .await
    .unwrap();

    let row = sqlx::query_as::<_, (String, String, serde_json::Value)>(
        "SELECT event_type, severity, user_ids FROM security_events WHERE guild_id = $1",
    )
    .bind(&gid)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.0, "raid_detected");
    assert_eq!(row.1, "critical");
    assert_eq!(row.2.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn security_events_filter_by_severity() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    for (event, sev) in &[
        ("alt_detected", "warning"),
        ("raid_detected", "critical"),
        ("lockdown", "critical"),
    ] {
        sqlx::query(
            "INSERT INTO security_events (id, guild_id, event_type, severity, description) VALUES (gen_random_uuid(), $1, $2, $3, 'test')",
        ).bind(&gid).bind(event).bind(sev).execute(&pool).await.unwrap();
    }

    let critical = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM security_events WHERE guild_id = $1 AND severity = 'critical'",
    )
    .bind(&gid)
    .fetch_one(&pool)
    .await
    .unwrap()
    .0;

    assert_eq!(critical, 2);
}

// ══════════════════════════════════════════════════════════
//  Logs (system logs)
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn log_insert_and_filter_by_level() {
    let pool = setup_pool().await;
    let bot_name = format!(
        "test-bot-{}",
        uuid::Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(8)
            .collect::<String>()
    );

    for (level, msg) in &[
        ("info", "Bot demarre"),
        ("warn", "Config manquante"),
        ("error", "API timeout"),
    ] {
        sqlx::query(
            "INSERT INTO logs (level, bot, server, message) VALUES ($1, $2, 'test-server', $3)",
        )
        .bind(level)
        .bind(&bot_name)
        .bind(msg)
        .execute(&pool)
        .await
        .unwrap();
    }

    let warns =
        sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM logs WHERE bot = $1 AND level = 'warn'")
            .bind(&bot_name)
            .fetch_one(&pool)
            .await
            .unwrap()
            .0;

    assert_eq!(warns, 1);
}

#[tokio::test]
async fn log_filter_by_category() {
    let pool = setup_pool().await;
    let bot_name = format!(
        "cat-test-{}",
        uuid::Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(8)
            .collect::<String>()
    );

    sqlx::query("INSERT INTO logs (level, bot, server, message, category) VALUES ('info', $1, 's', 'msg', 'discord')")
        .bind(&bot_name).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO logs (level, bot, server, message, category) VALUES ('info', $1, 's', 'msg', 'api')")
        .bind(&bot_name).execute(&pool).await.unwrap();

    let discord_logs = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM logs WHERE bot = $1 AND category = 'discord'",
    )
    .bind(&bot_name)
    .fetch_one(&pool)
    .await
    .unwrap()
    .0;

    assert_eq!(discord_logs, 1);
}

// ══════════════════════════════════════════════════════════
//  Daily activity — stats quotidiennes
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn daily_activity_upsert() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let today = chrono::Utc::now().date_naive();

    // Premier insert
    sqlx::query(
        r#"INSERT INTO daily_activity (guild_id, day, messages, voice_minutes, active_members, new_members, infractions, warns, mutes, bans, leaves)
           VALUES ($1, $2, 100, 60, 20, 5, 3, 2, 1, 0, 1)
           ON CONFLICT (guild_id, day) DO UPDATE SET messages = daily_activity.messages + EXCLUDED.messages"#,
    ).bind(&gid).bind(today).execute(&pool).await.unwrap();

    // Deuxieme upsert — additionne les messages
    sqlx::query(
        r#"INSERT INTO daily_activity (guild_id, day, messages, voice_minutes, active_members, new_members, infractions, warns, mutes, bans, leaves)
           VALUES ($1, $2, 50, 0, 0, 0, 0, 0, 0, 0, 0)
           ON CONFLICT (guild_id, day) DO UPDATE SET messages = daily_activity.messages + EXCLUDED.messages"#,
    ).bind(&gid).bind(today).execute(&pool).await.unwrap();

    let messages = sqlx::query_as::<_, (i64,)>(
        "SELECT messages FROM daily_activity WHERE guild_id = $1 AND day = $2",
    )
    .bind(&gid)
    .bind(today)
    .fetch_one(&pool)
    .await
    .unwrap()
    .0;

    assert_eq!(messages, 150, "Messages doivent etre cumules");
}

#[tokio::test]
async fn daily_activity_multi_day_trend() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let today = chrono::Utc::now().date_naive();

    for offset in 0..7 {
        let day = today - chrono::Duration::days(offset);
        let msgs = 100 - offset * 10;
        sqlx::query(
            r#"INSERT INTO daily_activity (guild_id, day, messages, voice_minutes, active_members, new_members, infractions, warns, mutes, bans, leaves)
               VALUES ($1, $2, $3, 0, 0, 0, 0, 0, 0, 0, 0)"#,
        ).bind(&gid).bind(day).bind(msgs).execute(&pool).await.unwrap();
    }

    let rows = sqlx::query_as::<_, (chrono::NaiveDate, i64)>(
        "SELECT day, messages FROM daily_activity WHERE guild_id = $1 ORDER BY day DESC",
    )
    .bind(&gid)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 7);
    assert_eq!(rows[0].1, 100); // aujourd'hui = le plus actif
    assert_eq!(rows[6].1, 40); // il y a 6 jours
}

#[tokio::test]
async fn daily_activity_guild_isolation() {
    let pool = setup_pool().await;
    let gid1 = unique_guild();
    let gid2 = unique_guild();
    let today = chrono::Utc::now().date_naive();

    sqlx::query(
        r#"INSERT INTO daily_activity (guild_id, day, messages, voice_minutes, active_members, new_members, infractions, warns, mutes, bans, leaves)
           VALUES ($1, $2, 999, 0, 0, 0, 0, 0, 0, 0, 0)"#,
    ).bind(&gid1).bind(today).execute(&pool).await.unwrap();

    let count =
        sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM daily_activity WHERE guild_id = $1")
            .bind(&gid2)
            .fetch_one(&pool)
            .await
            .unwrap()
            .0;

    assert_eq!(count, 0);
}

// ══════════════════════════════════════════════════════════
//  Guilds — enregistrement serveurs
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn guild_register_and_read() {
    let pool = setup_pool().await;
    let gid = short_guild_id();

    sqlx::query(
        r#"INSERT INTO guilds (guild_id, name, member_count)
           VALUES ($1, 'TestServer', 42)
           ON CONFLICT (guild_id) DO UPDATE SET name = EXCLUDED.name, updated_at = NOW()"#,
    )
    .bind(&gid)
    .execute(&pool)
    .await
    .unwrap();

    let row = sqlx::query_as::<_, (String, i32)>(
        "SELECT name, member_count FROM guilds WHERE guild_id = $1",
    )
    .bind(&gid)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.0, "TestServer");
    assert_eq!(row.1, 42);
}

#[tokio::test]
async fn guild_update_name_on_conflict() {
    let pool = setup_pool().await;
    let gid = short_guild_id();

    sqlx::query(
        "INSERT INTO guilds (guild_id, name) VALUES ($1, 'OldName') ON CONFLICT DO NOTHING",
    )
    .bind(&gid)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO guilds (guild_id, name) VALUES ($1, 'NewName') ON CONFLICT (guild_id) DO UPDATE SET name = EXCLUDED.name, updated_at = NOW()",
    ).bind(&gid).execute(&pool).await.unwrap();

    let name = sqlx::query_as::<_, (String,)>("SELECT name FROM guilds WHERE guild_id = $1")
        .bind(&gid)
        .fetch_one(&pool)
        .await
        .unwrap()
        .0;

    assert_eq!(name, "NewName");
}

// ══════════════════════════════════════════════════════════
//  Purge — nettoyage vieux logs
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn purge_old_audit_logs() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    // Vieux log (90 jours)
    sqlx::query(
        "INSERT INTO audit_logs (guild_id, event_type, details, created_at) VALUES ($1, 'old_event', '{}', NOW() - INTERVAL '90 days')",
    ).bind(&gid).execute(&pool).await.unwrap();

    // Log recent
    sqlx::query(
        "INSERT INTO audit_logs (guild_id, event_type, details) VALUES ($1, 'recent_event', '{}')",
    )
    .bind(&gid)
    .execute(&pool)
    .await
    .unwrap();

    // Purge > 30 jours
    let deleted = sqlx::query(
        "DELETE FROM audit_logs WHERE guild_id = $1 AND created_at < NOW() - INTERVAL '30 days'",
    )
    .bind(&gid)
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(deleted.rows_affected(), 1);

    let remaining =
        sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM audit_logs WHERE guild_id = $1")
            .bind(&gid)
            .fetch_one(&pool)
            .await
            .unwrap()
            .0;

    assert_eq!(remaining, 1);
}

#[tokio::test]
async fn purge_old_security_events() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    sqlx::query(
        "INSERT INTO security_events (id, guild_id, event_type, severity, description, created_at) VALUES (gen_random_uuid(), $1, 'old', 'info', 'old', NOW() - INTERVAL '60 days')",
    ).bind(&gid).execute(&pool).await.unwrap();

    sqlx::query(
        "INSERT INTO security_events (id, guild_id, event_type, severity, description) VALUES (gen_random_uuid(), $1, 'new', 'info', 'new')",
    ).bind(&gid).execute(&pool).await.unwrap();

    let deleted = sqlx::query(
        "DELETE FROM security_events WHERE guild_id = $1 AND created_at < NOW() - INTERVAL '30 days'",
    ).bind(&gid).execute(&pool).await.unwrap();

    assert_eq!(deleted.rows_affected(), 1);

    let remaining =
        sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM security_events WHERE guild_id = $1")
            .bind(&gid)
            .fetch_one(&pool)
            .await
            .unwrap()
            .0;

    assert_eq!(remaining, 1);
}
