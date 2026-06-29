//! Tests d'integration REELS pour progression-bot (avec PostgreSQL).
//! Couvre : user_levels, user_stats, streaks.

use sqlx::PgPool;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}

fn ugid() -> String {
    format!(
        "{}",
        uuid::Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    )
}

// ══════════════════════════════════════════════════════════
//  User levels — XP, level, text/voice split
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn user_level_create_and_read() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query(
        r#"INSERT INTO user_levels (guild_id, user_id, username, xp, level, xp_text, xp_voice, level_text, level_voice)
           VALUES ($1, '444', 'Alice', 500, 3, 300, 200, 2, 1)
           ON CONFLICT (guild_id, user_id) DO NOTHING"#,
    ).bind(&gid).execute(&p).await.unwrap();

    let row = sqlx::query_as::<_, (i64, i32, i64, i64, i32, i32)>(
        "SELECT xp, level, xp_text, xp_voice, level_text, level_voice FROM user_levels WHERE guild_id = $1 AND user_id = '444'",
    ).bind(&gid).fetch_one(&p).await.unwrap();

    assert_eq!(row.0, 500); // xp total
    assert_eq!(row.1, 3); // level
    assert_eq!(row.2, 300); // xp_text
    assert_eq!(row.3, 200); // xp_voice
    assert_eq!(row.4, 2); // level_text
    assert_eq!(row.5, 1); // level_voice
}

#[tokio::test]
async fn user_level_xp_increment() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query(
        "INSERT INTO user_levels (guild_id, user_id, username, xp, level, xp_text, level_text) VALUES ($1, '444', 'A', 100, 1, 100, 1) ON CONFLICT DO NOTHING",
    ).bind(&gid).execute(&p).await.unwrap();

    sqlx::query("UPDATE user_levels SET xp = xp + $3, xp_text = xp_text + $3 WHERE guild_id = $1 AND user_id = $2")
        .bind(&gid).bind("444").bind(50i64).execute(&p).await.unwrap();

    let xp = sqlx::query_as::<_, (i64,)>(
        "SELECT xp FROM user_levels WHERE guild_id = $1 AND user_id = '444'",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;
    assert_eq!(xp, 150);
}

#[tokio::test]
async fn user_level_leaderboard_sorted() {
    let p = pool().await;
    let gid = ugid();
    for (user, xp) in &[("poor", 100i64), ("rich", 5000), ("mid", 1000)] {
        sqlx::query(
            "INSERT INTO user_levels (guild_id, user_id, username, xp, level) VALUES ($1, $2, $2, $3, 1) ON CONFLICT DO NOTHING",
        ).bind(&gid).bind(user).bind(xp).execute(&p).await.unwrap();
    }

    let lb = sqlx::query_as::<_, (String, i64)>(
        "SELECT username, xp FROM user_levels WHERE guild_id = $1 ORDER BY xp DESC LIMIT 10",
    )
    .bind(&gid)
    .fetch_all(&p)
    .await
    .unwrap();

    assert_eq!(lb.len(), 3);
    assert_eq!(lb[0].0, "rich");
    assert_eq!(lb[1].0, "mid");
    assert_eq!(lb[2].0, "poor");
}

#[tokio::test]
async fn user_level_unique_per_guild_user() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO user_levels (guild_id, user_id, username) VALUES ($1, '444', 'A') ON CONFLICT DO NOTHING")
        .bind(&gid).execute(&p).await.unwrap();
    let dup = sqlx::query(
        "INSERT INTO user_levels (guild_id, user_id, username) VALUES ($1, '444', 'B')",
    )
    .bind(&gid)
    .execute(&p)
    .await;
    assert!(dup.is_err(), "Duplicate guild+user doit etre rejete");
}

// ══════════════════════════════════════════════════════════
//  Streaks — persistance dans user_levels
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn streak_persisted_in_user_levels() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO user_levels (guild_id, user_id, username) VALUES ($1, '444', 'A') ON CONFLICT DO NOTHING")
        .bind(&gid).execute(&p).await.unwrap();

    sqlx::query(
        "UPDATE user_levels SET streak_current = 7, streak_best = 15, streak_last_day = 100, streak_last_year = 2025 WHERE guild_id = $1 AND user_id = '444'",
    ).bind(&gid).execute(&p).await.unwrap();

    let row = sqlx::query_as::<_, (i32, i32, i32, i32)>(
        "SELECT streak_current, streak_best, streak_last_day, streak_last_year FROM user_levels WHERE guild_id = $1 AND user_id = '444'",
    ).bind(&gid).fetch_one(&p).await.unwrap();

    assert_eq!(row.0, 7);
    assert_eq!(row.1, 15);
    assert_eq!(row.2, 100);
    assert_eq!(row.3, 2025);
}

// ══════════════════════════════════════════════════════════
//  User stats — messages + voice
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn user_stats_upsert() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query(
        r#"INSERT INTO user_stats (id, guild_id, user_id, username, message_count, voice_seconds)
           VALUES (gen_random_uuid(), $1, '444', 'Alice', 100, 3600)
           ON CONFLICT (guild_id, user_id) DO UPDATE SET message_count = user_stats.message_count + EXCLUDED.message_count"#,
    ).bind(&gid).execute(&p).await.unwrap();

    // Upsert — cumule les messages
    sqlx::query(
        r#"INSERT INTO user_stats (id, guild_id, user_id, username, message_count, voice_seconds)
           VALUES (gen_random_uuid(), $1, '444', 'Alice', 50, 0)
           ON CONFLICT (guild_id, user_id) DO UPDATE SET message_count = user_stats.message_count + EXCLUDED.message_count"#,
    ).bind(&gid).execute(&p).await.unwrap();

    let msgs = sqlx::query_as::<_, (i64,)>(
        "SELECT message_count FROM user_stats WHERE guild_id = $1 AND user_id = '444'",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;

    assert_eq!(msgs, 150);
}

#[tokio::test]
async fn user_stats_voice_seconds_tracked() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query(
        "INSERT INTO user_stats (id, guild_id, user_id, username, voice_seconds) VALUES (gen_random_uuid(), $1, '444', 'Alice', 7200) ON CONFLICT DO NOTHING",
    ).bind(&gid).execute(&p).await.unwrap();

    let secs = sqlx::query_as::<_, (i64,)>(
        "SELECT voice_seconds FROM user_stats WHERE guild_id = $1 AND user_id = '444'",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;
    assert_eq!(secs, 7200); // 2h
}

// ══════════════════════════════════════════════════════════
//  Level config — level_up_channel_id
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn level_config_channel_id_persisted() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query("INSERT INTO level_config (guild_id, level_up_channel_id) VALUES ($1, '987654321012345678')")
        .bind(&gid).execute(&p).await.unwrap();

    let ch_id = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT level_up_channel_id FROM level_config WHERE guild_id = $1",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;

    assert_eq!(ch_id.unwrap(), "987654321012345678");
}

#[tokio::test]
async fn level_config_channel_id_null_by_default() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query("INSERT INTO level_config (guild_id) VALUES ($1)")
        .bind(&gid)
        .execute(&p)
        .await
        .unwrap();

    let ch_id = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT level_up_channel_id FROM level_config WHERE guild_id = $1",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;

    assert!(
        ch_id.is_none(),
        "level_up_channel_id doit etre NULL par defaut"
    );
}

#[tokio::test]
async fn level_config_channel_id_via_bot_config() {
    let p = pool().await;
    // Utiliser un snowflake court pour bot_guild_config (varchar 20)
    use rand::Rng;
    let gid = format!(
        "{}",
        rand::thread_rng().gen_range(10000000000000000u64..99999999999999999u64)
    );

    // Le bot lit level_up_channel_id depuis bot_guild_config
    sqlx::query(
        "INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value) VALUES ($1, 'progression-bot', 'level_up_channel_id', '123456789012345678')",
    ).bind(&gid).execute(&p).await.unwrap();

    let val = sqlx::query_as::<_, (String,)>(
        "SELECT config_value FROM bot_guild_config WHERE guild_id = $1 AND config_key = 'level_up_channel_id'",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;

    assert_eq!(val, "123456789012345678");

    // Simuler ce que le bot fait : parse en u64
    let channel_id: u64 = val.parse().unwrap();
    assert_eq!(channel_id, 123456789012345678);
}

#[tokio::test]
async fn level_config_channel_id_update() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query("INSERT INTO level_config (guild_id, level_up_channel_id) VALUES ($1, '111111111111111111')")
        .bind(&gid).execute(&p).await.unwrap();

    // Update
    sqlx::query(
        "UPDATE level_config SET level_up_channel_id = '999999999999999999' WHERE guild_id = $1",
    )
    .bind(&gid)
    .execute(&p)
    .await
    .unwrap();

    let ch_id = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT level_up_channel_id FROM level_config WHERE guild_id = $1",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;

    assert_eq!(ch_id.unwrap(), "999999999999999999");
}

#[tokio::test]
async fn level_config_channel_id_set_to_null() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query("INSERT INTO level_config (guild_id, level_up_channel_id) VALUES ($1, '111111111111111111')")
        .bind(&gid).execute(&p).await.unwrap();

    // Remettre a null (desactiver le canal dedie)
    sqlx::query("UPDATE level_config SET level_up_channel_id = NULL WHERE guild_id = $1")
        .bind(&gid)
        .execute(&p)
        .await
        .unwrap();

    let ch_id = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT level_up_channel_id FROM level_config WHERE guild_id = $1",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;

    assert!(ch_id.is_none());
}
