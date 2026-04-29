//! Tests d'integration REELS pour community-bot (avec PostgreSQL).
//! Couvre : auto_roles, role_panels, sponsorships, temp_roles.

use sqlx::PgPool;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}

fn ugid() -> String { format!("{}", uuid::Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128) }

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

// ══════════════════════════════════════════════════════════
//  CommunityGrpc handler (wire-up + validation + mapping)
// ══════════════════════════════════════════════════════════

mod community_grpc {
    use super::*;
    use sentinel_api::adapters::inbound::grpc::community::sponsorships::CommunityGrpc;
    use sentinel_proto::community::v1 as proto;
    use sentinel_proto::community::v1::community_service_server::CommunityService;
    use tonic::Request;

    fn grpc(p: PgPool) -> CommunityGrpc { CommunityGrpc { pg_pool: p } }

    #[tokio::test]
    async fn create_sponsorship_and_list_round_trip() {
        let p = pool().await;
        let g = grpc(p.clone());
        let gid = ugid();
        let sponsor = ugid();
        let sponsored = ugid();

        g.create_sponsorship(Request::new(proto::CreateSponsorshipRequest {
            guild_id: gid.clone(),
            sponsor_id: sponsor.clone(),
            sponsored_id: sponsored.clone(),
        })).await.unwrap();

        let resp = g.list_sponsorships(Request::new(proto::ListSponsorshipsRequest {
            guild_id: gid.clone(),
        })).await.unwrap();

        let list = resp.into_inner().sponsorships;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].sponsor_id, sponsor);
        assert_eq!(list[0].sponsored_id, sponsored);
        assert!(!list[0].created_at.is_empty());
    }

    #[tokio::test]
    async fn create_sponsorship_duplicate_is_noop_via_on_conflict() {
        let p = pool().await;
        let g = grpc(p.clone());
        let gid = ugid();
        let sponsored = ugid();

        // 1re insertion
        g.create_sponsorship(Request::new(proto::CreateSponsorshipRequest {
            guild_id: gid.clone(),
            sponsor_id: ugid(),
            sponsored_id: sponsored.clone(),
        })).await.unwrap();

        // 2e insertion même (guild, sponsored) → ON CONFLICT DO NOTHING → pas d'erreur
        let result = g.create_sponsorship(Request::new(proto::CreateSponsorshipRequest {
            guild_id: gid.clone(),
            sponsor_id: ugid(),
            sponsored_id: sponsored.clone(),
        })).await;
        assert!(result.is_ok());

        // Une seule row persistée
        let list = g.list_sponsorships(Request::new(proto::ListSponsorshipsRequest {
            guild_id: gid,
        })).await.unwrap().into_inner().sponsorships;
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn list_sponsorships_empty_for_fresh_guild() {
        let p = pool().await;
        let g = grpc(p);
        let list = g.list_sponsorships(Request::new(proto::ListSponsorshipsRequest {
            guild_id: ugid(),
        })).await.unwrap().into_inner().sponsorships;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn create_temp_role_rejects_bad_rfc3339() {
        let p = pool().await;
        let g = grpc(p);
        let err = g.create_temp_role(Request::new(proto::CreateTempRoleRequest {
            guild_id: ugid(),
            user_id: ugid(),
            role_id: ugid(),
            expires_at: "not-a-date".into(),
        })).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
        assert!(err.message().contains("RFC3339"));
    }

    #[tokio::test]
    async fn create_temp_role_and_list_only_active() {
        let p = pool().await;
        let g = grpc(p.clone());
        let gid = ugid();
        let uid = ugid();
        let role_future = ugid();
        let role_past = ugid();

        // Active : dans 1h
        let future = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        g.create_temp_role(Request::new(proto::CreateTempRoleRequest {
            guild_id: gid.clone(),
            user_id: uid.clone(),
            role_id: role_future.clone(),
            expires_at: future,
        })).await.unwrap();

        // Expire : -1h (insert direct car le handler ne valide pas le passé)
        let past = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        g.create_temp_role(Request::new(proto::CreateTempRoleRequest {
            guild_id: gid.clone(),
            user_id: uid.clone(),
            role_id: role_past.clone(),
            expires_at: past,
        })).await.unwrap();

        // list_temp_roles filtre `expires_at > NOW()` → doit ne renvoyer que le futur
        let list = g.list_temp_roles(Request::new(proto::ListTempRolesRequest {
            guild_id: gid.clone(),
        })).await.unwrap().into_inner().roles;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].role_id, role_future);
    }

    #[tokio::test]
    async fn create_temp_role_upsert_on_conflict() {
        let p = pool().await;
        let g = grpc(p.clone());
        let gid = ugid();
        let uid = ugid();
        let role = ugid();
        let first = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let second = (chrono::Utc::now() + chrono::Duration::hours(5)).to_rfc3339();

        g.create_temp_role(Request::new(proto::CreateTempRoleRequest {
            guild_id: gid.clone(), user_id: uid.clone(), role_id: role.clone(),
            expires_at: first,
        })).await.unwrap();
        g.create_temp_role(Request::new(proto::CreateTempRoleRequest {
            guild_id: gid.clone(), user_id: uid.clone(), role_id: role.clone(),
            expires_at: second.clone(),
        })).await.unwrap();

        let list = g.list_temp_roles(Request::new(proto::ListTempRolesRequest {
            guild_id: gid,
        })).await.unwrap().into_inner().roles;
        assert_eq!(list.len(), 1, "upsert doit garder une seule row");
        // expires_at a été mis à jour (second)
        assert!(list[0].expires_at.starts_with(&second[..19])); // compare date+heure (sans tz offset formatting)
    }

    #[tokio::test]
    async fn delete_temp_role_removes_only_target() {
        let p = pool().await;
        let g = grpc(p.clone());
        let gid = ugid();
        let uid = ugid();
        let role1 = ugid();
        let role2 = ugid();
        let future = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        for r in &[&role1, &role2] {
            g.create_temp_role(Request::new(proto::CreateTempRoleRequest {
                guild_id: gid.clone(), user_id: uid.clone(), role_id: (*r).clone(),
                expires_at: future.clone(),
            })).await.unwrap();
        }
        g.delete_temp_role(Request::new(proto::DeleteTempRoleRequest {
            guild_id: gid.clone(), user_id: uid.clone(), role_id: role1.clone(),
        })).await.unwrap();

        let list = g.list_temp_roles(Request::new(proto::ListTempRolesRequest {
            guild_id: gid,
        })).await.unwrap().into_inner().roles;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].role_id, role2);
    }
}
