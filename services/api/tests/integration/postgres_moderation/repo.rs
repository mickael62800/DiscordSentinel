//! Tests d'integration postgres pour PgModerationRepository.
//! La Phase 4 migration fait que toutes les lectures passent par audit_logs :
//! on seed directement cette table.

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::PgModerationRepository;
use sentinel_api::ports::outbound::ModerationRepository;

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
    let action = sentinel_api::domain::entities::ModerationAction {
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
