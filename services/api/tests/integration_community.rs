//! Tests d'integration REELS pour community-bot (avec PostgreSQL).
//! Couvre : auto_roles, role_panels, sponsorships, temp_roles.

use sqlx::PgPool;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}

fn ugid() -> String { format!("test_{}", uuid::Uuid::new_v4().simple()) }

// ══════════════════════════════════════════════════════════
//  Auto-roles
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn auto_role_add_and_list() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO auto_roles (guild_id, role_id, role_name) VALUES ($1, '111', 'Member')")
        .bind(&gid).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO auto_roles (guild_id, role_id, role_name, delay_secs) VALUES ($1, '222', 'Verified', 300)")
        .bind(&gid).execute(&p).await.unwrap();

    let count = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM auto_roles WHERE guild_id = $1")
        .bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(count, 2);
}

#[tokio::test]
async fn auto_role_unique_per_guild() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO auto_roles (guild_id, role_id, role_name) VALUES ($1, '111', 'A')")
        .bind(&gid).execute(&p).await.unwrap();
    let dup = sqlx::query("INSERT INTO auto_roles (guild_id, role_id, role_name) VALUES ($1, '111', 'B')")
        .bind(&gid).execute(&p).await;
    assert!(dup.is_err(), "Duplicate guild+role doit etre rejete");
}

#[tokio::test]
async fn auto_role_delete() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO auto_roles (guild_id, role_id, role_name) VALUES ($1, '111', 'A')")
        .bind(&gid).execute(&p).await.unwrap();
    sqlx::query("DELETE FROM auto_roles WHERE guild_id = $1 AND role_id = '111'")
        .bind(&gid).execute(&p).await.unwrap();
    let count = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM auto_roles WHERE guild_id = $1")
        .bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(count, 0);
}

// ══════════════════════════════════════════════════════════
//  Role panels + entries
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn role_panel_with_entries() {
    let p = pool().await;
    let gid = ugid();
    let panel_id = sqlx::query_as::<_, (uuid::Uuid,)>(
        "INSERT INTO role_panels (guild_id, channel_id, title, description) VALUES ($1, '555', 'Roles', 'Choisis') RETURNING id",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;

    for (role, label, pos) in &[("111", "Joueur", 0), ("222", "Artiste", 1), ("333", "Dev", 2)] {
        sqlx::query(
            "INSERT INTO role_panel_entries (panel_id, role_id, role_name, label, position) VALUES ($1, $2, $3, $3, $4)",
        ).bind(panel_id).bind(role).bind(label).bind(pos).execute(&p).await.unwrap();
    }

    let entries = sqlx::query_as::<_, (String, i32)>(
        "SELECT label, position FROM role_panel_entries WHERE panel_id = $1 ORDER BY position",
    ).bind(panel_id).fetch_all(&p).await.unwrap();

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].0, "Joueur");
    assert_eq!(entries[2].0, "Dev");
}

#[tokio::test]
async fn role_panel_entries_cascade_delete() {
    let p = pool().await;
    let gid = ugid();
    let panel_id = sqlx::query_as::<_, (uuid::Uuid,)>(
        "INSERT INTO role_panels (guild_id, channel_id, title) VALUES ($1, '555', 'Test') RETURNING id",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;

    sqlx::query("INSERT INTO role_panel_entries (panel_id, role_id, label) VALUES ($1, '111', 'R')")
        .bind(panel_id).execute(&p).await.unwrap();

    sqlx::query("DELETE FROM role_panels WHERE id = $1").bind(panel_id).execute(&p).await.unwrap();
    let count = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM role_panel_entries WHERE panel_id = $1")
        .bind(panel_id).fetch_one(&p).await.unwrap().0;
    assert_eq!(count, 0);
}

// ══════════════════════════════════════════════════════════
//  Sponsorships
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn sponsorship_create() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO sponsorships (guild_id, sponsor_id, sponsored_id) VALUES ($1, '111', '222')")
        .bind(&gid).execute(&p).await.unwrap();
    let count = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM sponsorships WHERE guild_id = $1")
        .bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(count, 1);
}

#[tokio::test]
async fn sponsorship_unique_per_sponsored() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO sponsorships (guild_id, sponsor_id, sponsored_id) VALUES ($1, '111', '222')")
        .bind(&gid).execute(&p).await.unwrap();
    let dup = sqlx::query("INSERT INTO sponsorships (guild_id, sponsor_id, sponsored_id) VALUES ($1, '333', '222')")
        .bind(&gid).execute(&p).await;
    assert!(dup.is_err(), "Un membre ne peut avoir qu'un seul parrain");
}

#[tokio::test]
async fn sponsorship_count_per_sponsor() {
    let p = pool().await;
    let gid = ugid();
    for sponsored in &["a", "b", "c"] {
        sqlx::query("INSERT INTO sponsorships (guild_id, sponsor_id, sponsored_id) VALUES ($1, '111', $2)")
            .bind(&gid).bind(sponsored).execute(&p).await.unwrap();
    }
    let count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM sponsorships WHERE guild_id = $1 AND sponsor_id = '111'",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(count, 3);
}

// ══════════════════════════════════════════════════════════
//  Temp roles
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn temp_role_create_and_expire() {
    let p = pool().await;
    let gid = ugid();

    // Role qui expire dans 1h
    sqlx::query("INSERT INTO temp_roles (guild_id, user_id, role_id, expires_at) VALUES ($1, '444', '555', NOW() + INTERVAL '1 hour')")
        .bind(&gid).execute(&p).await.unwrap();

    // Role deja expire
    sqlx::query("INSERT INTO temp_roles (guild_id, user_id, role_id, expires_at) VALUES ($1, '444', '666', NOW() - INTERVAL '1 hour')")
        .bind(&gid).execute(&p).await.unwrap();

    let active = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM temp_roles WHERE guild_id = $1 AND expires_at > NOW()",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(active, 1);

    let expired = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM temp_roles WHERE guild_id = $1 AND expires_at <= NOW()",
    ).bind(&gid).fetch_one(&p).await.unwrap().0;
    assert_eq!(expired, 1);
}

#[tokio::test]
async fn temp_role_unique_constraint() {
    let p = pool().await;
    let gid = ugid();
    sqlx::query("INSERT INTO temp_roles (guild_id, user_id, role_id, expires_at) VALUES ($1, '444', '555', NOW() + INTERVAL '1 hour')")
        .bind(&gid).execute(&p).await.unwrap();
    let dup = sqlx::query("INSERT INTO temp_roles (guild_id, user_id, role_id, expires_at) VALUES ($1, '444', '555', NOW() + INTERVAL '2 hours')")
        .bind(&gid).execute(&p).await;
    assert!(dup.is_err(), "Duplicate guild+user+role doit etre rejete");
}
