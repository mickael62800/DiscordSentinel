//! Tests d'integration pour les tables restantes :
//! guild_members, ticket_assignments, voice_channel_themes,
//! voice_channel_whitelists, bot_definitions, user_activity_log.

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

// ── Guild members ──

#[tokio::test]
async fn guild_member_register() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query(
        r#"INSERT INTO guild_members (guild_id, user_id, username, display_name, roles, joined_at)
           VALUES ($1, '444', 'Alice', 'Alice Cool', '["111","222"]'::jsonb, NOW())
           ON CONFLICT (guild_id, user_id) DO UPDATE SET username = EXCLUDED.username"#,
    )
    .bind(&gid)
    .execute(&p)
    .await
    .unwrap();

    let row = sqlx::query_as::<_, (String, Option<String>, serde_json::Value)>(
        "SELECT username, display_name, roles FROM guild_members WHERE guild_id = $1 AND user_id = '444'",
    ).bind(&gid).fetch_one(&p).await.unwrap();
    assert_eq!(row.0, "Alice");
    assert_eq!(row.1.unwrap(), "Alice Cool");
    assert_eq!(row.2.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn guild_member_unique_per_guild() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO guild_members (guild_id, user_id, username) VALUES ($1, '444', 'A') ON CONFLICT DO NOTHING")
        .bind(&gid).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO guild_members (guild_id, user_id, username) VALUES ($1, '444', 'B') ON CONFLICT (guild_id, user_id) DO UPDATE SET username = EXCLUDED.username")
        .bind(&gid).execute(&p).await.unwrap();
    let name = sqlx::query_as::<_, (String,)>(
        "SELECT username FROM guild_members WHERE guild_id = $1 AND user_id = '444'",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;
    assert_eq!(name, "B");
}

// ── Ticket assignments ──

#[tokio::test]
async fn ticket_assignment_history() {
    let p = pool().await;
    let gid = ugid();

    // Create ticket first
    let ticket_id = sqlx::query_as::<_, (uuid::Uuid,)>(
        "INSERT INTO tickets (id, title, author_id, author_name, server, category, ticket_type) VALUES (gen_random_uuid(), 'T', '444', 'U', $1, 'support', 'support') RETURNING id",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;

    sqlx::query("INSERT INTO ticket_assignments (id, ticket_id, assigned_to, assigned_by) VALUES (gen_random_uuid(), $1, '555', '333')")
        .bind(ticket_id).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO ticket_assignments (id, ticket_id, assigned_to, assigned_by) VALUES (gen_random_uuid(), $1, '666', '333')")
        .bind(ticket_id).execute(&p).await.unwrap();

    let count =
        sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM ticket_assignments WHERE ticket_id = $1")
            .bind(ticket_id)
            .fetch_one(&p)
            .await
            .unwrap()
            .0;
    assert_eq!(count, 2);
}

// ── Voice channel themes ──

#[tokio::test]
async fn voice_theme_crud() {
    let p = pool().await;
    let gid = ugid();

    let id = sqlx::query_as::<_, (uuid::Uuid,)>(
        "INSERT INTO voice_channel_themes (guild_id, name, bitrate, member_limit) VALUES ($1, 'Gaming', 96000, 10) RETURNING id",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;

    let row = sqlx::query_as::<_, (String, Option<i32>, Option<i32>)>(
        "SELECT name, bitrate, member_limit FROM voice_channel_themes WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(row.0, "Gaming");
    assert_eq!(row.1.unwrap(), 96000);
    assert_eq!(row.2.unwrap(), 10);
}

// ── Voice channel whitelists ──

#[tokio::test]
async fn whitelist_add_and_unique() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query("INSERT INTO voice_channel_whitelists (id, guild_id, owner_id, target_id, target_name) VALUES (gen_random_uuid(), $1, '111', '222', 'Friend')")
        .bind(&gid).execute(&p).await.unwrap();

    let count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM voice_channel_whitelists WHERE guild_id = $1 AND owner_id = '111'",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;
    assert_eq!(count, 1);

    // Duplicate doit etre rejete
    let dup = sqlx::query("INSERT INTO voice_channel_whitelists (id, guild_id, owner_id, target_id, target_name) VALUES (gen_random_uuid(), $1, '111', '222', 'F')")
        .bind(&gid).execute(&p).await;
    assert!(dup.is_err());
}

// ── Bot definitions ──

#[tokio::test]
async fn bot_definitions_seeded() {
    let p = pool().await;
    // Les definitions sont seedees par les migrations
    let count = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM bot_definitions")
        .fetch_one(&p)
        .await
        .unwrap()
        .0;
    assert!(
        count >= 5,
        "Au moins 5 bot definitions doivent etre seedees (got {})",
        count
    );
}

#[tokio::test]
async fn bot_definition_has_config_schema() {
    let p = pool().await;
    let automod = sqlx::query_as::<_, (String, serde_json::Value)>(
        "SELECT display_name, config_schema FROM bot_definitions WHERE bot_name = 'automod-bot'",
    )
    .fetch_optional(&p)
    .await
    .unwrap();

    if let Some((name, schema)) = automod {
        assert!(!name.is_empty());
        assert!(
            !schema.as_array().unwrap().is_empty(),
            "automod-bot doit avoir des config keys"
        );
    }
}

// ── User activity log (watched users tracking) ──

#[tokio::test]
async fn user_activity_log_record() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query(
        r#"INSERT INTO user_activity_log (id, guild_id, user_id, event_type, channel_id, content, metadata, created_at)
           VALUES (gen_random_uuid(), $1, '444', 'message_sent', '555', 'Salut', '{}', NOW())"#,
    ).bind(&gid).execute(&p).await.unwrap();

    let count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM user_activity_log WHERE guild_id = $1 AND user_id = '444'",
    )
    .bind(&gid)
    .fetch_one(&p)
    .await
    .unwrap()
    .0;
    assert_eq!(count, 1);
}

#[tokio::test]
async fn user_activity_log_filter_by_event() {
    let p = pool().await;
    let gid = ugid();

    for event in &["message_sent", "message_sent", "voice_join", "reaction_add"] {
        sqlx::query(
            "INSERT INTO user_activity_log (id, guild_id, user_id, event_type, metadata, created_at) VALUES (gen_random_uuid(), $1, '444', $2, '{}', NOW())",
        ).bind(&gid).bind(event).execute(&p).await.unwrap();
    }

    let msgs = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM user_activity_log WHERE guild_id = $1 AND event_type = 'message_sent'",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(msgs, 2);
}
