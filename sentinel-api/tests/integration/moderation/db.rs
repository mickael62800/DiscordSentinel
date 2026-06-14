//! Tests d'integration REELS pour moderation/automod/audit (avec PostgreSQL).

use sqlx::PgPool;

async fn setup_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.expect("Impossible de se connecter a la base de test")
}

fn unique_guild() -> String {
    format!("{}", uuid::Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

// ══════════════════════════════════════════════════════════
//  Moderation actions
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn log_moderation_action_persists() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    let id = sqlx::query_as::<_, (uuid::Uuid,)>(
        r#"INSERT INTO moderation_actions
           (id, guild_id, channel_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, created_at)
           VALUES (gen_random_uuid(), $1, '555555555555555555', '333333333333333333', 'Mod', '444444444444444444', 'User', 'warn', 'Spam', NOW())
           RETURNING id"#,
    ).bind(&gid).fetch_one(&pool).await.unwrap();

    assert!(!id.0.is_nil());
}

#[tokio::test]
async fn get_history_returns_actions_for_target() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    for action_type in &["warn", "warn", "mute_temp"] {
        sqlx::query(
            r#"INSERT INTO moderation_actions
               (id, guild_id, channel_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, created_at)
               VALUES (gen_random_uuid(), $1, '555555555555555555', '333333333333333333', 'Mod', '444444444444444444', 'User', $2, 'Test', NOW())"#,
        ).bind(&gid).bind(action_type).execute(&pool).await.unwrap();
    }

    let count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM moderation_actions WHERE guild_id = $1 AND target_id = '444444444444444444'",
    ).bind(&gid).fetch_one(&pool).await.unwrap().0;

    assert_eq!(count, 3);
}

#[tokio::test]
async fn different_guilds_isolated() {
    let pool = setup_pool().await;
    let gid1 = unique_guild();
    let gid2 = unique_guild();

    sqlx::query(
        r#"INSERT INTO moderation_actions
           (id, guild_id, channel_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, created_at)
           VALUES (gen_random_uuid(), $1, '555555555555555555', '333333333333333333', 'Mod', '444444444444444444', 'User', 'ban_permanent', 'Grave', NOW())"#,
    ).bind(&gid1).execute(&pool).await.unwrap();

    let count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM moderation_actions WHERE guild_id = $1",
    ).bind(&gid2).fetch_one(&pool).await.unwrap().0;

    assert_eq!(count, 0);
}

// ══════════════════════════════════════════════════════════
//  Strikes (user_strikes)
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn strikes_accumulate_per_user() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    for _ in 0..3 {
        sqlx::query(
            r#"INSERT INTO user_strikes (guild_id, user_id, reason, source, created_at, expires_at)
               VALUES ($1, '444444444444444444', 'Warn', 'automod', NOW(), NOW() + INTERVAL '7 days')"#,
        ).bind(&gid).execute(&pool).await.unwrap();
    }

    let total = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM user_strikes WHERE guild_id = $1 AND user_id = '444444444444444444' AND expires_at > NOW()",
    ).bind(&gid).fetch_one(&pool).await.unwrap().0;

    assert_eq!(total, 3);
}

#[tokio::test]
async fn expired_strikes_excluded() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    // Strike expire
    sqlx::query(
        r#"INSERT INTO user_strikes (guild_id, user_id, reason, source, created_at, expires_at)
           VALUES ($1, '444444444444444444', 'Vieux', 'automod', NOW() - INTERVAL '30 days', NOW() - INTERVAL '1 day')"#,
    ).bind(&gid).execute(&pool).await.unwrap();

    // Strike actif
    sqlx::query(
        r#"INSERT INTO user_strikes (guild_id, user_id, reason, source, created_at, expires_at)
           VALUES ($1, '444444444444444444', 'Recent', 'automod', NOW(), NOW() + INTERVAL '7 days')"#,
    ).bind(&gid).execute(&pool).await.unwrap();

    let active = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM user_strikes WHERE guild_id = $1 AND user_id = '444444444444444444' AND expires_at > NOW()",
    ).bind(&gid).fetch_one(&pool).await.unwrap().0;

    assert_eq!(active, 1, "Seul le strike actif doit compter");
}

// ══════════════════════════════════════════════════════════
//  Infractions (automod) — flags en JSONB
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn infraction_persists_with_flags() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    let flags = serde_json::json!({"spam": true, "insult": false, "link": false, "phishing": false});

    sqlx::query(
        r#"INSERT INTO infractions (id, guild_id, channel_id, user_id, username, message_id, content, flags, score, action, reason, created_at)
           VALUES (gen_random_uuid(), $1, '555555555555555555', '444444444444444444', 'Spammer', 'msg123', 'buy buy buy', $2, 3.5, 'warn', 'Spam detecte', NOW())"#,
    ).bind(&gid).bind(&flags).execute(&pool).await.unwrap();

    let row = sqlx::query_as::<_, (f64, String, serde_json::Value)>(
        "SELECT score, action, flags FROM infractions WHERE guild_id = $1 AND user_id = '444444444444444444'",
    ).bind(&gid).fetch_one(&pool).await.unwrap();

    assert!((row.0 - 3.5).abs() < 0.01);
    assert_eq!(row.1, "warn");
    assert_eq!(row.2["spam"], true);
}

#[tokio::test]
async fn infractions_count_by_action() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let flags = serde_json::json!({"spam": true, "insult": false, "link": false, "phishing": false});

    for (action, count) in &[("warn", 3), ("delete", 2), ("mute", 1)] {
        for _ in 0..*count {
            sqlx::query(
                r#"INSERT INTO infractions (id, guild_id, channel_id, user_id, username, message_id, content, flags, score, action, reason, created_at)
                   VALUES (gen_random_uuid(), $1, '555', '444', 'User', 'msg', 'x', $2, 1.0, $3, 'r', NOW())"#,
            ).bind(&gid).bind(&flags).bind(action).execute(&pool).await.unwrap();
        }
    }

    let total = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM infractions WHERE guild_id = $1",
    ).bind(&gid).fetch_one(&pool).await.unwrap().0;

    assert_eq!(total, 6);
}

// ══════════════════════════════════════════════════════════
//  Rules
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn rules_crud() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    let id = sqlx::query_as::<_, (uuid::Uuid,)>(
        r#"INSERT INTO rules (id, guild_id, flag_type, weight, threshold_warn, threshold_delete, threshold_mute, threshold_ban, enabled, created_at, updated_at)
           VALUES (gen_random_uuid(), $1, 'spam', 5.0, 2.0, 4.0, 6.0, 9.0, true, NOW(), NOW())
           RETURNING id"#,
    ).bind(&gid).fetch_one(&pool).await.unwrap();

    let rule = sqlx::query_as::<_, (String, f64, bool)>(
        "SELECT flag_type, weight, enabled FROM rules WHERE id = $1",
    ).bind(id.0).fetch_one(&pool).await.unwrap();

    assert_eq!(rule.0, "spam");
    assert!((rule.1 - 5.0).abs() < 0.01);
    assert!(rule.2);

    sqlx::query("DELETE FROM rules WHERE id = $1").bind(id.0).execute(&pool).await.unwrap();
    let count = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM rules WHERE id = $1")
        .bind(id.0).fetch_one(&pool).await.unwrap().0;
    assert_eq!(count, 0);
}

// ══════════════════════════════════════════════════════════
//  Audit logs — details en JSONB
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn audit_log_persists() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    let details = serde_json::json!({"description": "Message supprime", "severity": "info"});

    sqlx::query(
        r#"INSERT INTO audit_logs (guild_id, event_type, actor_id, actor_name, target_id, target_name, details, created_at)
           VALUES ($1, 'message_delete', '333333333333333333', 'Mod', '444444444444444444', 'User', $2, NOW())"#,
    ).bind(&gid).bind(&details).execute(&pool).await.unwrap();

    let row = sqlx::query_as::<_, (String, serde_json::Value)>(
        "SELECT event_type, details FROM audit_logs WHERE guild_id = $1",
    ).bind(&gid).fetch_one(&pool).await.unwrap();

    assert_eq!(row.0, "message_delete");
    assert_eq!(row.1["severity"], "info");
}

// ══════════════════════════════════════════════════════════
//  Notes (user_notes)
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn notes_crud() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    let id = sqlx::query_as::<_, (uuid::Uuid,)>(
        r#"INSERT INTO user_notes (id, guild_id, user_id, author_id, author_name, content, category, created_at)
           VALUES (gen_random_uuid(), $1, '444444444444444444', '333333333333333333', 'Mod', 'Suspect', 'warning', NOW())
           RETURNING id"#,
    ).bind(&gid).fetch_one(&pool).await.unwrap();

    let note = sqlx::query_as::<_, (String, String)>(
        "SELECT content, category FROM user_notes WHERE guild_id = $1 AND user_id = '444444444444444444'",
    ).bind(&gid).fetch_one(&pool).await.unwrap();

    assert_eq!(note.0, "Suspect");
    assert_eq!(note.1, "warning");

    sqlx::query("DELETE FROM user_notes WHERE id = $1").bind(id.0).execute(&pool).await.unwrap();
}

// ══════════════════════════════════════════════════════════
//  Security events
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn security_event_persists() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    sqlx::query(
        r#"INSERT INTO security_events (id, guild_id, event_type, severity, description, created_at)
           VALUES (gen_random_uuid(), $1, 'raid_detected', 'critical', '15 joins en 10s', NOW())"#,
    ).bind(&gid).execute(&pool).await.unwrap();

    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT event_type, severity FROM security_events WHERE guild_id = $1",
    ).bind(&gid).fetch_one(&pool).await.unwrap();

    assert_eq!(row.0, "raid_detected");
    assert_eq!(row.1, "critical");
}

