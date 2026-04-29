//! Tests d'integration postgres pour PgModerationRepository.
//! La Phase 4 migration fait que toutes les lectures passent par audit_logs :
//! on seed directement cette table.

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::moderation::moderation_repository::PgModerationRepository;
use sentinel_api::ports::outbound::moderation::moderation_repository::ModerationRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

async fn seed_audit(
    pool: &PgPool, guild: &str, event_type: &str,
    target_id: &str, action_id: Option<Uuid>, reason: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    let details = if let Some(aid) = action_id {
        serde_json::json!({"reason": reason, "action_id": aid.to_string()})
    } else {
        serde_json::json!({"reason": reason})
    };
    sqlx::query(
        "INSERT INTO audit_logs (id, guild_id, event_type, actor_id, actor_name, target_id, target_name, channel_id, details) \
         VALUES ($1, $2, $3, 'mod1', 'Mod', $4, 'Target', 'ch', $5)"
    )
    .bind(id).bind(guild).bind(event_type).bind(target_id).bind(details)
    .execute(pool).await.unwrap();
    id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_is_noop_phase4() {
    // save() est un no-op (Phase 4).
    let repo = PgModerationRepository::new(pool().await);
    let action = sentinel_api::domain::entities::moderation::action::action::ModerationAction {
        id: Uuid::new_v4(), guild_id: "g".into(), channel_id: "c".into(),
        moderator_id: "m".into(), moderator_name: "M".into(),
        target_id: "t".into(), target_name: "T".into(),
        action_type: "warn".into(), reason: "test".into(),
        gravity: None, duration: None,
        created_at: chrono::Utc::now(),
    };
    repo.save(&action).await.unwrap(); // should not error
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_id_via_action_id_in_details() {
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let guild = fresh_id();
    let target = fresh_id();
    let action_id = Uuid::new_v4();
    seed_audit(&pool, &guild, "mod_warn", &target, Some(action_id), "spam").await;

    let got = repo.find_by_id(action_id).await.unwrap().unwrap();
    assert_eq!(got.action_type, "warn");
    assert_eq!(got.reason, "spam");
    assert_eq!(got.target_id, target);
    // action_id dans details prime sur id row.
    assert_eq!(got.id, action_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_id_unknown_returns_none() {
    let repo = PgModerationRepository::new(pool().await);
    let got = repo.find_by_id(Uuid::new_v4()).await.unwrap();
    assert!(got.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_target_scoped_and_ordered_desc() {
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let guild = fresh_id();
    let target = fresh_id();
    seed_audit(&pool, &guild, "mod_warn", &target, None, "first").await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    seed_audit(&pool, &guild, "mod_mute", &target, None, "second").await;

    let got = repo.find_by_target(&guild, &target, 50).await.unwrap();
    assert_eq!(got.len(), 2);
    // DESC : second d'abord
    assert_eq!(got[0].reason, "second");
    assert_eq!(got[1].reason, "first");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_target_limit_clamped_to_1_min() {
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let guild = fresh_id();
    let target = fresh_id();
    seed_audit(&pool, &guild, "mod_warn", &target, None, "x").await;
    // limit 0 -> clamp a 1
    let got = repo.find_by_target(&guild, &target, 0).await.unwrap();
    assert_eq!(got.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_bans_latest_per_user_excludes_superseded_unban() {
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let guild = fresh_id();
    let victim_a = fresh_id();
    let victim_b = fresh_id();

    seed_audit(&pool, &guild, "mod_ban", &victim_a, None, "banA").await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    // victim_a unban apres -> devrait disparaitre des bans actifs.
    seed_audit(&pool, &guild, "mod_unban", &victim_a, None, "unbanA").await;

    seed_audit(&pool, &guild, "mod_ban", &victim_b, None, "banB").await;

    let bans = repo.find_bans(Some(&guild), 50, 0).await.unwrap();
    assert_eq!(bans.len(), 1);
    assert_eq!(bans[0].target_id, victim_b);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_all_for_guild_returns_all_mod_events() {
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let guild = fresh_id();
    let target = fresh_id();
    seed_audit(&pool, &guild, "mod_warn", &target, None, "a").await;
    seed_audit(&pool, &guild, "mod_mute", &target, None, "b").await;
    seed_audit(&pool, &guild, "mod_ban", &target, None, "c").await;

    let all = repo.find_all_for_guild(Some(&guild), 100).await.unwrap();
    assert_eq!(all.len(), 3);
    let types: Vec<_> = all.iter().map(|a| a.action_type.as_str()).collect();
    assert!(types.contains(&"warn"));
    assert!(types.contains(&"mute"));
    assert!(types.contains(&"ban"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_bans_for_user_removes_only_bans() {
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let guild = fresh_id();
    let target = fresh_id();
    seed_audit(&pool, &guild, "mod_warn", &target, None, "w").await;
    seed_audit(&pool, &guild, "mod_ban", &target, None, "b1").await;
    seed_audit(&pool, &guild, "mod_ban", &target, None, "b2").await;

    repo.delete_bans_for_user(&guild, &target).await.unwrap();
    let rest = repo.find_by_target(&guild, &target, 50).await.unwrap();
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].action_type, "warn");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_action_by_action_id_returns_true_when_found() {
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let guild = fresh_id();
    let target = fresh_id();
    let aid = Uuid::new_v4();
    seed_audit(&pool, &guild, "mod_warn", &target, Some(aid), "x").await;

    let deleted = repo.delete_action(aid).await.unwrap();
    assert!(deleted);
    // 2e appel : plus rien.
    let deleted = repo.delete_action(aid).await.unwrap();
    assert!(!deleted);
}

// ── find_bans global (None guild) ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_bans_global_across_guilds() {
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let g1 = fresh_id();
    let g2 = fresh_id();
    seed_audit(&pool, &g1, "mod_ban", &fresh_id(), None, "ban-g1").await;
    seed_audit(&pool, &g2, "mod_ban", &fresh_id(), None, "ban-g2").await;

    let global = repo.find_bans(None, 50, 0).await.unwrap();
    // Au moins 2 bans (peut y en avoir d'autres d'autres tests).
    let g1_found = global.iter().any(|b| b.guild_id == g1);
    let g2_found = global.iter().any(|b| b.guild_id == g2);
    assert!(g1_found);
    assert!(g2_found);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_bans_respects_limit() {
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let guild = fresh_id();
    for _ in 0..3 {
        seed_audit(&pool, &guild, "mod_ban", &fresh_id(), None, "x").await;
    }
    let got = repo.find_bans(Some(&guild), 2, 0).await.unwrap();
    assert_eq!(got.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_bans_with_offset_skips_rows() {
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let guild = fresh_id();
    for _ in 0..3 {
        seed_audit(&pool, &guild, "mod_ban", &fresh_id(), None, "x").await;
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    let page1 = repo.find_bans(Some(&guild), 2, 0).await.unwrap();
    let page2 = repo.find_bans(Some(&guild), 2, 2).await.unwrap();
    assert_eq!(page1.len(), 2);
    assert_eq!(page2.len(), 1);
}

// ── find_by_target scoping ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_target_scoped_to_guild() {
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let g1 = fresh_id();
    let g2 = fresh_id();
    let target = fresh_id();
    seed_audit(&pool, &g1, "mod_warn", &target, None, "g1-warn").await;
    seed_audit(&pool, &g2, "mod_warn", &target, None, "g2-warn").await;

    let got = repo.find_by_target(&g1, &target, 50).await.unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].reason, "g1-warn");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_bans_excluded_by_unban_stays_excluded_after_new_event() {
    // unban -> nouveau ban doit reapparaitre.
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let guild = fresh_id();
    let target = fresh_id();
    seed_audit(&pool, &guild, "mod_ban", &target, None, "b1").await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    seed_audit(&pool, &guild, "mod_unban", &target, None, "u1").await;
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    // Rebanni
    seed_audit(&pool, &guild, "mod_ban", &target, None, "b2").await;

    let bans = repo.find_bans(Some(&guild), 50, 0).await.unwrap();
    assert!(bans.iter().any(|b| b.target_id == target));
}

// ── delete_action sans action_id dans details ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_action_no_action_id_returns_false() {
    // Un audit sans action_id dans details : delete_action cherche
    // details->>'action_id' = $1 et donc 0 row deleted.
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let guild = fresh_id();
    seed_audit(&pool, &guild, "mod_warn", &fresh_id(), None, "no-aid").await;
    let deleted = repo.delete_action(Uuid::new_v4()).await.unwrap();
    assert!(!deleted);
}

// ── find_all_for_guild global ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_all_for_guild_none_returns_cross_guild() {
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let g1 = fresh_id();
    let g2 = fresh_id();
    seed_audit(&pool, &g1, "mod_warn", &fresh_id(), None, "a").await;
    seed_audit(&pool, &g2, "mod_mute", &fresh_id(), None, "b").await;
    let all = repo.find_all_for_guild(None, 100).await.unwrap();
    assert!(all.iter().any(|a| a.guild_id == g1));
    assert!(all.iter().any(|a| a.guild_id == g2));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_all_for_guild_respects_limit() {
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let guild = fresh_id();
    for i in 0..5 {
        seed_audit(&pool, &guild, "mod_warn", &fresh_id(), None, &format!("r{i}")).await;
    }
    let got = repo.find_all_for_guild(Some(&guild), 3).await.unwrap();
    assert_eq!(got.len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_bans_for_user_no_bans_is_noop() {
    // User sans ban -> delete ne fail pas, retourne Ok(())
    let pool = pool().await;
    let repo = PgModerationRepository::new(pool.clone());
    let res = repo.delete_bans_for_user(&fresh_id(), &fresh_id()).await;
    assert!(res.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_target_empty_returns_empty() {
    let repo = PgModerationRepository::new(pool().await);
    let res = repo.find_by_target(&fresh_id(), &fresh_id(), 10).await.unwrap();
    assert!(res.is_empty());
}
