//! Tests d'integration pour welcome-bot (avec PostgreSQL).

use sqlx::PgPool;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}

fn ugid() -> String { format!("test_{}", uuid::Uuid::new_v4().simple()) }

// ══════════════════════════════════════════════════════════
//  Config CRUD
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn welcome_config_defaults() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query("INSERT INTO welcome_config (guild_id) VALUES ($1)")
        .bind(&gid).execute(&p).await.unwrap();

    let row = sqlx::query_as::<_, (bool, bool, bool, bool, bool)>(
        "SELECT welcome_enabled, welcome_dm_enabled, leave_enabled, rules_enabled, counter_enabled FROM welcome_config WHERE guild_id = $1",
    ).bind(&gid).fetch_one(&p).await.unwrap();

    assert!(row.0);  // welcome_enabled = true
    assert!(!row.1); // dm_enabled = false
    assert!(row.2);  // leave_enabled = true
    assert!(!row.3); // rules_enabled = false
    assert!(!row.4); // counter_enabled = false
}

#[tokio::test]
async fn welcome_config_custom_message() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query(
        "INSERT INTO welcome_config (guild_id, welcome_message, welcome_channel_id) VALUES ($1, 'Hello {user} !', '123456789')",
    ).bind(&gid).execute(&p).await.unwrap();

    let row = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT welcome_message, welcome_channel_id FROM welcome_config WHERE guild_id = $1",
    ).bind(&gid).fetch_one(&p).await.unwrap();

    assert_eq!(row.0, "Hello {user} !");
    assert_eq!(row.1.unwrap(), "123456789");
}

#[tokio::test]
async fn welcome_config_update_partial() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query("INSERT INTO welcome_config (guild_id) VALUES ($1)")
        .bind(&gid).execute(&p).await.unwrap();

    // Update seulement le message
    sqlx::query("UPDATE welcome_config SET welcome_message = 'Yo {user}!', updated_at = NOW() WHERE guild_id = $1")
        .bind(&gid).execute(&p).await.unwrap();

    let msg = sqlx::query_as::<_, (String,)>(
        "SELECT welcome_message FROM welcome_config WHERE guild_id = $1",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;

    assert_eq!(msg, "Yo {user}!");
}

#[tokio::test]
async fn welcome_config_rules_setup() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query(
        "INSERT INTO welcome_config (guild_id, rules_enabled, rules_channel_id, rules_role_id, rules_button_label) VALUES ($1, true, '999', '888', 'Je valide')",
    ).bind(&gid).execute(&p).await.unwrap();

    let row = sqlx::query_as::<_, (bool, Option<String>, Option<String>, String)>(
        "SELECT rules_enabled, rules_channel_id, rules_role_id, rules_button_label FROM welcome_config WHERE guild_id = $1",
    ).bind(&gid).fetch_one(&p).await.unwrap();

    assert!(row.0);
    assert_eq!(row.1.unwrap(), "999");
    assert_eq!(row.2.unwrap(), "888");
    assert_eq!(row.3, "Je valide");
}

#[tokio::test]
async fn welcome_config_counter() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query(
        "INSERT INTO welcome_config (guild_id, counter_enabled, counter_channel_id, counter_format) VALUES ($1, true, '777', 'Total : {count} joueurs')",
    ).bind(&gid).execute(&p).await.unwrap();

    let row = sqlx::query_as::<_, (bool, Option<String>, String)>(
        "SELECT counter_enabled, counter_channel_id, counter_format FROM welcome_config WHERE guild_id = $1",
    ).bind(&gid).fetch_one(&p).await.unwrap();

    assert!(row.0);
    assert_eq!(row.1.unwrap(), "777");
    assert!(row.2.contains("{count}"));
}

#[tokio::test]
async fn welcome_config_anniversary() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query(
        "INSERT INTO welcome_config (guild_id, anniversary_enabled, anniversary_channel_id) VALUES ($1, true, '666')",
    ).bind(&gid).execute(&p).await.unwrap();

    let row = sqlx::query_as::<_, (bool, Option<String>)>(
        "SELECT anniversary_enabled, anniversary_channel_id FROM welcome_config WHERE guild_id = $1",
    ).bind(&gid).fetch_one(&p).await.unwrap();

    assert!(row.0);
    assert_eq!(row.1.unwrap(), "666");
}

#[tokio::test]
async fn welcome_config_guild_isolation() {
    let p = pool().await;
    let gid1 = ugid();
    let gid2 = ugid();

    sqlx::query("INSERT INTO welcome_config (guild_id, welcome_message) VALUES ($1, 'A')")
        .bind(&gid1).execute(&p).await.unwrap();

    let exists = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM welcome_config WHERE guild_id = $1",
    ).bind(&gid2).fetch_one(&p).await.unwrap().0;

    assert_eq!(exists, 0);
}

#[tokio::test]
async fn welcome_config_embed_color() {
    let p = pool().await;
    let gid = ugid();

    sqlx::query("INSERT INTO welcome_config (guild_id, welcome_embed_color) VALUES ($1, 'ff5733')")
        .bind(&gid).execute(&p).await.unwrap();

    let color = sqlx::query_as::<_, (String,)>(
        "SELECT welcome_embed_color FROM welcome_config WHERE guild_id = $1",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;

    assert_eq!(color, "ff5733");
    // Verifier le parsing
    let parsed = u32::from_str_radix(&color, 16).unwrap();
    assert_eq!(parsed, 0xff5733);
}

// ══════════════════════════════════════════════════════════
//  Rejoin message
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn welcome_rejoin_message_default() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO welcome_config (guild_id) VALUES ($1)").bind(&gid).execute(&p).await.unwrap();

    let msg = sqlx::query_as::<_, (String,)>("SELECT rejoin_message FROM welcome_config WHERE guild_id = $1")
        .bind(&gid).fetch_one(&p).await.unwrap().0;
    assert!(msg.contains("{user}"), "Le message de retour par defaut doit contenir le placeholder user");
}

#[tokio::test]
async fn welcome_rejoin_message_custom() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO welcome_config (guild_id, rejoin_message) VALUES ($1, 'Re {user} ! Tu reviens deja ?')")
        .bind(&gid).execute(&p).await.unwrap();

    let msg = sqlx::query_as::<_, (String,)>("SELECT rejoin_message FROM welcome_config WHERE guild_id = $1")
        .bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(msg, "Re {user} ! Tu reviens deja ?");
}

#[tokio::test]
async fn detect_rejoin_via_guild_members() {
    let p = pool().await;
    let gid = ugid();

    // Premiere visite — pas dans guild_members
    let exists = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM guild_members WHERE guild_id = $1 AND user_id = '444'",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(exists, 0);

    // Enregistrer le membre (simule le premier join)
    sqlx::query("INSERT INTO guild_members (guild_id, user_id, username) VALUES ($1, '444', 'Alice') ON CONFLICT DO NOTHING")
        .bind(&gid).execute(&p).await.unwrap();

    // Deuxieme visite — maintenant present dans guild_members
    let exists = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM guild_members WHERE guild_id = $1 AND user_id = '444'",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(exists, 1, "Le membre doit etre connu apres le premier join");
}
