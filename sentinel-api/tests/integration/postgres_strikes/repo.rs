//! Tests d'integration postgres pour PgStrikeRepository.
//! Utilisent la vraie DB (DATABASE_URL) via sqlx.

use chrono::Duration;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::moderation::strike_repository::PgStrikeRepository;
use sentinel_core::domain::entities::moderation::action::strikes::StrikeConfig;
use sentinel_core::domain::entities::moderation::action::strikes::StrikeThreshold;
use sentinel_core::domain::entities::moderation::action::strikes::UserStrike;
use sentinel_api::ports::outbound::moderation::strike_repository::StrikeRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_|
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.unwrap()
}

fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

fn sample_strike(guild: &str, user: &str) -> UserStrike {
    UserStrike {
        id: Uuid::new_v4(),
        guild_id: guild.into(),
        user_id: user.into(),
        reason: "test strike".into(),
        source: "test".into(),
        infraction_id: None,
        expires_at: None,
        created_at: Utc::now(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_and_find_active_strikes() {
    let pool = pool().await;
    let repo = PgStrikeRepository::new(pool.clone());
    let guild = fresh_id();
    let user = fresh_id();

    let s1 = sample_strike(&guild, &user);
    let s2 = sample_strike(&guild, &user);
    repo.save_strike(&s1).await.unwrap();
    repo.save_strike(&s2).await.unwrap();

    let active = repo.find_active_strikes(&guild, &user, 3600).await.unwrap();
    assert_eq!(active.len(), 2);
    // ORDER BY created_at DESC — s2 inserted second (but Utc::now() resolution may tie).
    assert!(active.iter().any(|s| s.id == s1.id));
    assert!(active.iter().any(|s| s.id == s2.id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_active_strikes_excludes_expired() {
    let pool = pool().await;
    let repo = PgStrikeRepository::new(pool.clone());
    let guild = fresh_id();
    let user = fresh_id();

    let mut expired = sample_strike(&guild, &user);
    expired.expires_at = Some(Utc::now() - Duration::hours(1));
    let mut active = sample_strike(&guild, &user);
    active.expires_at = Some(Utc::now() + Duration::hours(1));
    repo.save_strike(&expired).await.unwrap();
    repo.save_strike(&active).await.unwrap();

    let found = repo.find_active_strikes(&guild, &user, 86_400).await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, active.id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_active_strikes_respects_window_secs() {
    let pool = pool().await;
    let repo = PgStrikeRepository::new(pool.clone());
    let guild = fresh_id();
    let user = fresh_id();

    let mut old = sample_strike(&guild, &user);
    old.created_at = Utc::now() - Duration::hours(2);
    repo.save_strike(&old).await.unwrap();

    // Fenetre 1h : doit exclure old (2h).
    let found = repo.find_active_strikes(&guild, &user, 3600).await.unwrap();
    assert_eq!(found.len(), 0);

    // Fenetre 24h : inclut old.
    let found = repo.find_active_strikes(&guild, &user, 86_400).await.unwrap();
    assert_eq!(found.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_active_strikes_scoped_to_user() {
    let pool = pool().await;
    let repo = PgStrikeRepository::new(pool.clone());
    let guild = fresh_id();
    let user_a = fresh_id();
    let user_b = fresh_id();

    repo.save_strike(&sample_strike(&guild, &user_a)).await.unwrap();
    repo.save_strike(&sample_strike(&guild, &user_b)).await.unwrap();

    let found_a = repo.find_active_strikes(&guild, &user_a, 3600).await.unwrap();
    assert_eq!(found_a.len(), 1);
    assert_eq!(found_a[0].user_id, user_a);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_strikes_removes_all_for_user() {
    let pool = pool().await;
    let repo = PgStrikeRepository::new(pool.clone());
    let guild = fresh_id();
    let user = fresh_id();

    repo.save_strike(&sample_strike(&guild, &user)).await.unwrap();
    repo.save_strike(&sample_strike(&guild, &user)).await.unwrap();
    repo.delete_strikes(&guild, &user).await.unwrap();

    let found = repo.find_active_strikes(&guild, &user, 3600).await.unwrap();
    assert_eq!(found.len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_strike_by_infraction_id_returns_count() {
    let pool = pool().await;
    let repo = PgStrikeRepository::new(pool.clone());
    let guild = fresh_id();
    let user = fresh_id();
    let infraction_id = Uuid::new_v4();

    let mut s = sample_strike(&guild, &user);
    s.infraction_id = Some(infraction_id);
    repo.save_strike(&s).await.unwrap();

    let n = repo.delete_strike_by_infraction_id(infraction_id).await.unwrap();
    assert_eq!(n, 1);

    // Deuxieme delete = 0 lignes.
    let n = repo.delete_strike_by_infraction_id(infraction_id).await.unwrap();
    assert_eq!(n, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_config_none_when_absent() {
    let pool = pool().await;
    let repo = PgStrikeRepository::new(pool);
    let got = repo.get_config(&fresh_id()).await.unwrap();
    assert!(got.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_and_get_config_roundtrip() {
    let pool = pool().await;
    let repo = PgStrikeRepository::new(pool.clone());
    let guild = fresh_id();

    let cfg = StrikeConfig {
        guild_id: guild.clone().into(),
        window_secs: 3600,
        thresholds: vec![
            StrikeThreshold { strikes: 3, action: "warn".into(), duration: None },
            StrikeThreshold { strikes: 5, action: "mute".into(), duration: Some(600) },
        ],
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    repo.save_config(&cfg).await.unwrap();

    let got = repo.get_config(&guild).await.unwrap().unwrap();
    assert_eq!(got.window_secs, 3600);
    assert_eq!(got.thresholds.len(), 2);
    assert_eq!(got.thresholds[0].strikes, 3);
    assert_eq!(got.thresholds[1].action, "mute");
    assert!(got.enabled);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_config_is_upsert_on_guild_id() {
    let pool = pool().await;
    let repo = PgStrikeRepository::new(pool.clone());
    let guild = fresh_id();

    let mut cfg = StrikeConfig {
        guild_id: guild.clone().into(),
        window_secs: 3600,
        thresholds: vec![],
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    repo.save_config(&cfg).await.unwrap();

    cfg.window_secs = 7200;
    cfg.enabled = false;
    repo.save_config(&cfg).await.unwrap();

    let got = repo.get_config(&guild).await.unwrap().unwrap();
    assert_eq!(got.window_secs, 7200);
    assert!(!got.enabled);
}
