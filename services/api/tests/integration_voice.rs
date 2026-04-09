//! Tests d'integration REELS pour les channels vocaux (avec PostgreSQL).

use sqlx::PgPool;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}

fn ugid() -> String { format!("test_{}", uuid::Uuid::new_v4().simple()) }

async fn create_channel(pool: &PgPool, gid: &str, owner: &str, channel_id: &str, kind: &str) -> uuid::Uuid {
    sqlx::query_as::<_, (uuid::Uuid,)>(
        r#"INSERT INTO voice_channels (id, guild_id, owner_id, owner_name, channel_id, channel_name, kind)
           VALUES (gen_random_uuid(), $1, $2, 'Owner', $3, 'Salon', $4) RETURNING id"#,
    ).bind(gid).bind(owner).bind(channel_id).bind(kind).fetch_one(pool).await.unwrap().0
}

#[tokio::test]
async fn voice_channel_create_defaults() {
    let p = pool().await;
    let gid = ugid();
    let ch_id = format!("ch_{}", uuid::Uuid::new_v4().simple().to_string().chars().take(8).collect::<String>());
    let id = create_channel(&p, &gid, "444", &ch_id, "public").await;
    let row = sqlx::query_as::<_, (String, bool, bool, String)>(
        "SELECT kind, locked, queue_enabled, channel_status FROM voice_channels WHERE id = $1",
    ).bind(id).fetch_one(&p).await.unwrap();
    assert_eq!(row.0, "public");
    assert!(!row.1); // not locked
    assert!(!row.2); // queue not enabled
    assert_eq!(row.3, "open");
}

#[tokio::test]
async fn voice_channel_unique_channel_id() {
    let p = pool().await;
    let gid = ugid();
    let ch_id = format!("ch_{}", uuid::Uuid::new_v4().simple().to_string().chars().take(8).collect::<String>());
    create_channel(&p, &gid, "444", &ch_id, "public").await;
    let dup = sqlx::query(
        "INSERT INTO voice_channels (id, guild_id, owner_id, owner_name, channel_id, channel_name, kind) VALUES (gen_random_uuid(), $1, '444', 'O', $2, 'S', 'public')",
    ).bind(&gid).bind(&ch_id).execute(&p).await;
    assert!(dup.is_err(), "Duplicate channel_id doit etre rejete");
}

#[tokio::test]
async fn voice_channel_close_sets_status() {
    let p = pool().await;
    let gid = ugid();
    let ch_id = format!("ch_{}", uuid::Uuid::new_v4().simple().to_string().chars().take(8).collect::<String>());
    let id = create_channel(&p, &gid, "444", &ch_id, "public").await;
    sqlx::query("UPDATE voice_channels SET channel_status = 'closed', closed_at = NOW() WHERE id = $1")
        .bind(id).execute(&p).await.unwrap();
    let status = sqlx::query_as::<_, (String,)>("SELECT channel_status FROM voice_channels WHERE id = $1")
        .bind(id).fetch_one(&p).await.unwrap().0;
    assert_eq!(status, "closed");
}

#[tokio::test]
async fn voice_channel_co_admins() {
    let p = pool().await;
    let gid = ugid();
    let ch_id = format!("ch_{}", uuid::Uuid::new_v4().simple().to_string().chars().take(8).collect::<String>());
    let vc_id = create_channel(&p, &gid, "444", &ch_id, "public").await;

    sqlx::query("INSERT INTO voice_channel_co_admins (id, voice_channel_id, user_id, user_name) VALUES (gen_random_uuid(), $1, '555', 'CoAdmin1')")
        .bind(vc_id).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO voice_channel_co_admins (id, voice_channel_id, user_id, user_name) VALUES (gen_random_uuid(), $1, '666', 'CoAdmin2')")
        .bind(vc_id).execute(&p).await.unwrap();

    let count = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM voice_channel_co_admins WHERE voice_channel_id = $1")
        .bind(vc_id).fetch_one(&p).await.unwrap().0;
    assert_eq!(count, 2);
}

#[tokio::test]
async fn voice_channel_bans_cascade() {
    let p = pool().await;
    let gid = ugid();
    let ch_id = format!("ch_{}", uuid::Uuid::new_v4().simple().to_string().chars().take(8).collect::<String>());
    let vc_id = create_channel(&p, &gid, "444", &ch_id, "private").await;

    sqlx::query("INSERT INTO voice_channel_bans (id, voice_channel_id, user_id, user_name, banned_by) VALUES (gen_random_uuid(), $1, '999', 'Banned', '444')")
        .bind(vc_id).execute(&p).await.unwrap();

    // Delete channel → bans cascade
    sqlx::query("DELETE FROM voice_channels WHERE id = $1").bind(vc_id).execute(&p).await.unwrap();
    let bans = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM voice_channel_bans WHERE voice_channel_id = $1")
        .bind(vc_id).fetch_one(&p).await.unwrap().0;
    assert_eq!(bans, 0);
}

#[tokio::test]
async fn voice_channel_lock_toggle() {
    let p = pool().await;
    let gid = ugid();
    let ch_id = format!("ch_{}", uuid::Uuid::new_v4().simple().to_string().chars().take(8).collect::<String>());
    let id = create_channel(&p, &gid, "444", &ch_id, "public").await;

    sqlx::query("UPDATE voice_channels SET locked = true WHERE id = $1").bind(id).execute(&p).await.unwrap();
    let locked = sqlx::query_as::<_, (bool,)>("SELECT locked FROM voice_channels WHERE id = $1")
        .bind(id).fetch_one(&p).await.unwrap().0;
    assert!(locked);

    sqlx::query("UPDATE voice_channels SET locked = false WHERE id = $1").bind(id).execute(&p).await.unwrap();
    let locked = sqlx::query_as::<_, (bool,)>("SELECT locked FROM voice_channels WHERE id = $1")
        .bind(id).fetch_one(&p).await.unwrap().0;
    assert!(!locked);
}
