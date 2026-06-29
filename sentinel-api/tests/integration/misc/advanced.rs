//! Tests d'integration avances : hourly_activity, watched_users, pending_mod_actions,
//! voice_sessions, discord_roles, sanction_reminders, voice_channel features.

use sqlx::PgPool;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}

/// Genere un guild_id unique au format snowflake (≤19 chiffres) pour
/// respecter VARCHAR(20) depuis la migration 101.
fn ugid() -> String {
    let u = uuid::Uuid::new_v4().as_u128();
    // modulo 10^18 garantit ≤18 chiffres (snowflake Discord = 18-19 digits)
    format!("{}", u % 1_000_000_000_000_000_000_u128)
}

// ══════════════════════════════════════════════════════════
//  Hourly activity — heatmap
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn hourly_activity_upsert() {
    let p = pool().await;
    let gid = ugid();
    let today = chrono::Utc::now().date_naive();

    sqlx::query(
        "INSERT INTO hourly_activity (guild_id, day, hour, messages) VALUES ($1, $2, 14, 50) ON CONFLICT (guild_id, day, hour) DO UPDATE SET messages = hourly_activity.messages + EXCLUDED.messages",
    ).bind(&gid).bind(today).execute(&p).await.unwrap();
    sqlx::query(
        "INSERT INTO hourly_activity (guild_id, day, hour, messages) VALUES ($1, $2, 14, 30) ON CONFLICT (guild_id, day, hour) DO UPDATE SET messages = hourly_activity.messages + EXCLUDED.messages",
    ).bind(&gid).bind(today).execute(&p).await.unwrap();

    let msgs = sqlx::query_as::<_, (i64,)>(
        "SELECT messages FROM hourly_activity WHERE guild_id = $1 AND hour = 14",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;
    assert_eq!(msgs, 80);
}

#[tokio::test]
async fn hourly_activity_check_constraint() {
    let p = pool().await;
    let gid = ugid();
    let today = chrono::Utc::now().date_naive();
    // hour = 24 doit etre rejete (CHECK hour >= 0 AND hour <= 23)
    let bad = sqlx::query(
        "INSERT INTO hourly_activity (guild_id, day, hour, messages) VALUES ($1, $2, 24, 1)",
    )
    .bind(&gid)
    .bind(today)
    .execute(&p)
    .await;
    assert!(bad.is_err(), "hour=24 doit violer le CHECK constraint");
}

#[tokio::test]
async fn hourly_heatmap_data() {
    let p = pool().await;
    let gid = ugid();
    let today = chrono::Utc::now().date_naive();
    for hour in 0..24i16 {
        sqlx::query(
            "INSERT INTO hourly_activity (guild_id, day, hour, messages) VALUES ($1, $2, $3, $4)",
        )
        .bind(&gid)
        .bind(today)
        .bind(hour)
        .bind((hour as i64) * 10)
        .execute(&p)
        .await
        .unwrap();
    }
    let rows = sqlx::query_as::<_, (i16, i64)>(
        "SELECT hour, messages FROM hourly_activity WHERE guild_id = $1 AND day = $2 ORDER BY hour",
    )
    .bind(&gid)
    .bind(today)
    .fetch_all(&p)
    .await
    .unwrap();
    assert_eq!(rows.len(), 24);
    assert_eq!(rows[0].1, 0); // midnight
    assert_eq!(rows[23].1, 230); // 23h
}

// ══════════════════════════════════════════════════════════
//  Watched users
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn watched_user_add_and_query() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO manual_watched_users (guild_id, user_id, username, reason) VALUES ($1, '444', 'Suspect', 'Comportement bizarre')")
        .bind(&gid).execute(&p).await.unwrap();
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT username, reason FROM manual_watched_users WHERE guild_id = $1 AND user_id = '444'",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(row.0, "Suspect");
    assert!(row.1.contains("bizarre"));
}

#[tokio::test]
async fn watched_user_unique_per_guild() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query(
        "INSERT INTO manual_watched_users (guild_id, user_id, username) VALUES ($1, '444', 'A')",
    )
    .bind(&gid)
    .execute(&p)
    .await
    .unwrap();
    let dup = sqlx::query(
        "INSERT INTO manual_watched_users (guild_id, user_id, username) VALUES ($1, '444', 'B')",
    )
    .bind(&gid)
    .execute(&p)
    .await;
    assert!(dup.is_err());
}

// ══════════════════════════════════════════════════════════
//  Pending mod actions (mode apprenti)
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn pending_action_lifecycle() {
    let p = pool().await;
    let gid = ugid();
    let id = sqlx::query_as::<_, (uuid::Uuid,)>(
        r#"INSERT INTO pending_mod_actions (guild_id, moderator_id, moderator_name, target_id, target_name, action_type, reason)
           VALUES ($1, '333', 'Mod', '444', 'User', 'warn', 'Spam') RETURNING id"#,
    ).bind(&gid).fetch_one(&p).await.unwrap().0;

    // Pending
    let status =
        sqlx::query_as::<_, (String,)>("SELECT status FROM pending_mod_actions WHERE id = $1")
            .bind(id)
            .fetch_one(&p)
            .await
            .unwrap()
            .0;
    assert_eq!(status, "pending");

    // Approve
    sqlx::query("UPDATE pending_mod_actions SET status = 'approved', reviewed_by = '555', updated_at = NOW() WHERE id = $1")
        .bind(id).execute(&p).await.unwrap();
    let row = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT status, reviewed_by FROM pending_mod_actions WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(row.0, "approved");
    assert_eq!(row.1.unwrap(), "555");
}

#[tokio::test]
async fn pending_action_reject() {
    let p = pool().await;
    let gid = ugid();
    let id = sqlx::query_as::<_, (uuid::Uuid,)>(
        "INSERT INTO pending_mod_actions (guild_id, moderator_id, moderator_name, target_id, target_name, action_type) VALUES ($1, '333', 'Mod', '444', 'User', 'ban') RETURNING id",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;

    sqlx::query(
        "UPDATE pending_mod_actions SET status = 'rejected', reviewed_by = '555' WHERE id = $1",
    )
    .bind(id)
    .execute(&p)
    .await
    .unwrap();
    let status =
        sqlx::query_as::<_, (String,)>("SELECT status FROM pending_mod_actions WHERE id = $1")
            .bind(id)
            .fetch_one(&p)
            .await
            .unwrap()
            .0;
    assert_eq!(status, "rejected");
}

// ══════════════════════════════════════════════════════════
//  Voice sessions
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn voice_session_record() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query(
        "INSERT INTO voice_sessions (guild_id, user_id, username, channel_id, channel_name, duration_secs, started_at, ended_at) VALUES ($1, '444', 'Alice', '555', 'General', 3600, NOW() - INTERVAL '1 hour', NOW())",
    ).bind(&gid).execute(&p).await.unwrap();

    let dur = sqlx::query_as::<_, (i64,)>(
        "SELECT duration_secs FROM voice_sessions WHERE guild_id = $1 AND user_id = '444'",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;
    assert_eq!(dur, 3600);
}

#[tokio::test]
async fn voice_session_total_per_user() {
    let p = pool().await;
    let gid = ugid();
    for dur in &[1800i64, 3600, 900] {
        sqlx::query(
            "INSERT INTO voice_sessions (guild_id, user_id, username, channel_id, duration_secs, started_at, ended_at) VALUES ($1, '444', 'A', '555', $2, NOW(), NOW())",
        ).bind(&gid).bind(dur).execute(&p).await.unwrap();
    }
    let total = sqlx::query_as::<_, (i64,)>(
        "SELECT COALESCE(SUM(duration_secs), 0)::bigint FROM voice_sessions WHERE guild_id = $1 AND user_id = '444'",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(total, 6300); // 1800+3600+900
}

// ══════════════════════════════════════════════════════════
//  Discord roles sync
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn discord_role_sync() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query(
        "INSERT INTO discord_roles (id, guild_id, name, color, position, permissions, member_count) VALUES ('111', $1, 'Admin', 16711680, 10, '8', 5)",
    ).bind(&gid).execute(&p).await.unwrap();
    sqlx::query(
        "INSERT INTO discord_roles (id, guild_id, name, color, position, permissions, member_count) VALUES ('222', $1, 'Mod', 65280, 5, '4', 10)",
    ).bind(&gid).execute(&p).await.unwrap();

    let roles = sqlx::query_as::<_, (String, String, i32)>(
        "SELECT id, name, position FROM discord_roles WHERE guild_id = $1 ORDER BY position DESC",
    )
    .bind(&gid)
    .fetch_all(&p)
    .await
    .unwrap();
    assert_eq!(roles.len(), 2);
    assert_eq!(roles[0].1, "Admin"); // position 10
    assert_eq!(roles[1].1, "Mod"); // position 5
}

// ══════════════════════════════════════════════════════════
//  Sanction reminders
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn reminder_pending_query() {
    let p = pool().await;
    let gid = ugid();
    let action_id = uuid::Uuid::new_v4();

    // Reminder qui arrive dans 1h
    sqlx::query(
        r#"INSERT INTO sanction_reminders (guild_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, action_id, remind_at, expires_at)
           VALUES ($1, '333', 'Mod', '444', 'User', 'unmute', 'Fin du mute', $2, NOW() + INTERVAL '1 hour', NOW() + INTERVAL '2 hours')"#,
    ).bind(&gid).bind(action_id).execute(&p).await.unwrap();

    // Reminder deja du (NOW - 1h)
    sqlx::query(
        r#"INSERT INTO sanction_reminders (guild_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, action_id, remind_at, expires_at)
           VALUES ($1, '333', 'Mod', '555', 'User2', 'unban', 'Fin du ban', $2, NOW() - INTERVAL '1 hour', NOW() + INTERVAL '1 hour')"#,
    ).bind(&gid).bind(uuid::Uuid::new_v4()).execute(&p).await.unwrap();

    let pending = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM sanction_reminders WHERE guild_id = $1 AND status = 'pending' AND remind_at <= NOW()",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(pending, 1, "Seul le reminder du doit apparaitre");
}

// ══════════════════════════════════════════════════════════
//  Voice channel invite links
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn invite_link_lifecycle() {
    let p = pool().await;
    let gid = ugid();
    let ch_id = format!(
        "ch_{}",
        uuid::Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(8)
            .collect::<String>()
    );

    let vc_id = sqlx::query_as::<_, (uuid::Uuid,)>(
        "INSERT INTO voice_channels (id, guild_id, owner_id, owner_name, channel_id, channel_name, kind) VALUES (gen_random_uuid(), $1, '444', 'O', $2, 'S', 'private') RETURNING id",
    ).bind(&gid).bind(&ch_id).fetch_one(&p).await.unwrap().0;

    let code = format!(
        "INV{}",
        uuid::Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(5)
            .collect::<String>()
    );
    sqlx::query(
        r#"INSERT INTO voice_channel_invite_links (voice_channel_id, guild_id, channel_id, created_by, created_by_name, code, max_uses, expires_at)
           VALUES ($1, $2, $3, '444', 'Owner', $4, 5, NOW() + INTERVAL '1 day')"#,
    ).bind(vc_id).bind(&gid).bind(&ch_id).bind(&code).execute(&p).await.unwrap();

    // Use the link
    sqlx::query(
        "UPDATE voice_channel_invite_links SET current_uses = current_uses + 1 WHERE code = $1",
    )
    .bind(&code)
    .execute(&p)
    .await
    .unwrap();

    let row = sqlx::query_as::<_, (i32, Option<i32>, bool)>(
        "SELECT current_uses, max_uses, revoked FROM voice_channel_invite_links WHERE code = $1",
    )
    .bind(&code)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(row.0, 1);
    assert_eq!(row.1, Some(5));
    assert!(!row.2); // not revoked
}

#[tokio::test]
async fn invite_link_unique_code() {
    let p = pool().await;
    let gid = ugid();
    let ch1 = format!(
        "ch1_{}",
        uuid::Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(6)
            .collect::<String>()
    );
    let ch2 = format!(
        "ch2_{}",
        uuid::Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(6)
            .collect::<String>()
    );

    let vc1 = sqlx::query_as::<_, (uuid::Uuid,)>(
        "INSERT INTO voice_channels (id, guild_id, owner_id, owner_name, channel_id, channel_name, kind) VALUES (gen_random_uuid(), $1, '444', 'O', $2, 'S', 'private') RETURNING id",
    ).bind(&gid).bind(&ch1).fetch_one(&p).await.unwrap().0;
    let vc2 = sqlx::query_as::<_, (uuid::Uuid,)>(
        "INSERT INTO voice_channels (id, guild_id, owner_id, owner_name, channel_id, channel_name, kind) VALUES (gen_random_uuid(), $1, '444', 'O', $2, 'S', 'private') RETURNING id",
    ).bind(&gid).bind(&ch2).fetch_one(&p).await.unwrap().0;

    let unique_code = format!(
        "UNIQ{}",
        uuid::Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(4)
            .collect::<String>()
    );
    sqlx::query(
        "INSERT INTO voice_channel_invite_links (voice_channel_id, guild_id, channel_id, created_by, created_by_name, code, expires_at) VALUES ($1, $2, $3, '444', 'O', $4, NOW() + INTERVAL '1 day')",
    ).bind(vc1).bind(&gid).bind(&ch1).bind(&unique_code).execute(&p).await.unwrap();

    let dup = sqlx::query(
        "INSERT INTO voice_channel_invite_links (voice_channel_id, guild_id, channel_id, created_by, created_by_name, code, expires_at) VALUES ($1, $2, $3, '444', 'O', $4, NOW() + INTERVAL '1 day')",
    ).bind(vc2).bind(&gid).bind(&ch2).bind(&unique_code).execute(&p).await;
    assert!(dup.is_err(), "Duplicate code doit etre rejete");
}

// ══════════════════════════════════════════════════════════
//  Coude combats — resolve guard
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn combat_resolve_only_pending() {
    let p = pool().await;
    let gid = ugid();

    let id = sqlx::query_as::<_, (uuid::Uuid,)>(
        "INSERT INTO coude_combats (guild_id, channel_id, attacker_id, attacker_name, defender_id, defender_name) VALUES ($1, '555', '111', 'A', '222', 'B') RETURNING id",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;

    // Resolve
    let result = sqlx::query(
        "UPDATE coude_combats SET status = 'resolved', winner_id = '111', resolved_at = NOW() WHERE id = $1 AND status IN ('pending', 'accepted', 'betting')",
    ).bind(id).execute(&p).await.unwrap();
    assert_eq!(result.rows_affected(), 1);

    // Double resolve — doit echouer (rows_affected = 0)
    let result2 = sqlx::query(
        "UPDATE coude_combats SET status = 'resolved', winner_id = '222', resolved_at = NOW() WHERE id = $1 AND status IN ('pending', 'accepted', 'betting')",
    ).bind(id).execute(&p).await.unwrap();
    assert_eq!(
        result2.rows_affected(),
        0,
        "Combat deja resolu ne doit pas etre re-resolu"
    );
}

#[tokio::test]
async fn coude_player_coins_never_negative() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query(
        "INSERT INTO coude_players (guild_id, user_id, username, coins) VALUES ($1, '444', 'A', 50) ON CONFLICT DO NOTHING",
    ).bind(&gid).execute(&p).await.unwrap();

    // LEAST(coins, 200) = 50 → perd 50, pas 200
    sqlx::query(
        "UPDATE coude_players SET coins = coins - LEAST(coins, $3) WHERE guild_id = $1 AND user_id = $2",
    ).bind(&gid).bind("444").bind(200i64).execute(&p).await.unwrap();

    let coins = sqlx::query_as::<_, (i64,)>(
        "SELECT coins FROM coude_players WHERE guild_id = $1 AND user_id = '444'",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;
    assert_eq!(coins, 0, "Coins ne doit pas etre negatif");
}
