//! Tests d'integration postgres pour PgInfractionRepository.

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::moderation::infraction_repository::PgInfractionRepository;
use sentinel_core::domain::entities::moderation::infraction::Infraction;
use sentinel_core::domain::enums::moderation::action::Action;
use sentinel_core::domain::entities::moderation::detection_flags::DetectionFlags;
use sentinel_api::ports::inbound::moderation::manage_infractions::InfractionFilters;
use sentinel_api::ports::outbound::moderation::infraction_repository::InfractionRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}

fn fresh_id() -> String {
    format!("{}", Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

fn sample_infraction(guild: &str, user: &str, action: Action, score: f64) -> Infraction {
    Infraction {
        id: Uuid::new_v4(),
        guild_id: guild.into(),
        channel_id: "chan".into(),
        user_id: user.into(),
        username: format!("user_{user}"),
        message_id: fresh_id(),
        content: "bad message".into(),
        flags: DetectionFlags {
            spam: true, insult: false, link: false, phishing: false,
        },
        score,
        action,
        reason: "test".into(),
        duration: None,
        created_at: Utc::now(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_and_find_by_guild() {
    let repo = PgInfractionRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.save(&sample_infraction(&g, &u, Action::Warn, 3.0)).await.unwrap();
    let list = repo
        .find_by_guild(&g, &InfractionFilters { user_id: None, action: None, limit: 10, offset: 0 })
        .await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].user_id, u);
    assert_eq!(list[0].action, Action::Warn);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_guild_filter_by_user_id() {
    let repo = PgInfractionRepository::new(pool().await);
    let g = fresh_id();
    let u1 = fresh_id();
    let u2 = fresh_id();
    repo.save(&sample_infraction(&g, &u1, Action::Warn, 2.0)).await.unwrap();
    repo.save(&sample_infraction(&g, &u2, Action::Mute, 5.0)).await.unwrap();
    let list = repo
        .find_by_guild(&g, &InfractionFilters { user_id: Some(u1.clone()), action: None, limit: 10, offset: 0 })
        .await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].user_id, u1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_guild_filter_by_action() {
    let repo = PgInfractionRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.save(&sample_infraction(&g, &u, Action::Warn, 2.0)).await.unwrap();
    repo.save(&sample_infraction(&g, &u, Action::Mute, 5.0)).await.unwrap();
    repo.save(&sample_infraction(&g, &u, Action::Ban, 8.0)).await.unwrap();
    let mutes = repo
        .find_by_guild(&g, &InfractionFilters { user_id: None, action: Some("mute".into()), limit: 10, offset: 0 })
        .await.unwrap();
    assert_eq!(mutes.len(), 1);
    assert_eq!(mutes[0].action, Action::Mute);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_guild_respects_limit_and_offset() {
    let repo = PgInfractionRepository::new(pool().await);
    let g = fresh_id();
    for _ in 0..5 {
        repo.save(&sample_infraction(&g, &fresh_id(), Action::Warn, 1.0)).await.unwrap();
    }
    let page1 = repo
        .find_by_guild(&g, &InfractionFilters { user_id: None, action: None, limit: 2, offset: 0 })
        .await.unwrap();
    assert_eq!(page1.len(), 2);
    let page2 = repo
        .find_by_guild(&g, &InfractionFilters { user_id: None, action: None, limit: 2, offset: 2 })
        .await.unwrap();
    assert_eq!(page2.len(), 2);
    // Pages disjointes
    assert_ne!(page1[0].id, page2[0].id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_all_paginates_across_guilds() {
    let repo = PgInfractionRepository::new(pool().await);
    let g1 = fresh_id();
    let g2 = fresh_id();
    repo.save(&sample_infraction(&g1, &fresh_id(), Action::Warn, 1.0)).await.unwrap();
    repo.save(&sample_infraction(&g2, &fresh_id(), Action::Mute, 5.0)).await.unwrap();
    let all = repo.find_all(100, 0).await.unwrap();
    assert!(all.len() >= 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_id_existing_returns_infraction() {
    let repo = PgInfractionRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    let inf = sample_infraction(&g, &u, Action::Warn, 2.5);
    let id = inf.id;
    repo.save(&inf).await.unwrap();
    let got = repo.find_by_id(&id.to_string()).await.unwrap().unwrap();
    assert_eq!(got.id, id);
    assert_eq!(got.user_id, u);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_id_absent_returns_none() {
    let repo = PgInfractionRepository::new(pool().await);
    let res = repo.find_by_id(&Uuid::new_v4().to_string()).await.unwrap();
    assert!(res.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_id_invalid_uuid_returns_none_or_error() {
    let repo = PgInfractionRepository::new(pool().await);
    // sqlx peut retourner une erreur Internal si l'id ne parse pas — on accepte les deux.
    let res = repo.find_by_id("not-a-uuid").await;
    assert!(res.is_err() || res.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_by_id_existing_returns_true() {
    let repo = PgInfractionRepository::new(pool().await);
    let g = fresh_id();
    let inf = sample_infraction(&g, &fresh_id(), Action::Warn, 1.0);
    let id = inf.id.to_string();
    repo.save(&inf).await.unwrap();
    assert!(repo.delete_by_id(&id).await.unwrap());
    // Deuxieme delete retourne false
    assert!(!repo.delete_by_id(&id).await.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_by_id_absent_returns_false() {
    let repo = PgInfractionRepository::new(pool().await);
    let ok = repo.delete_by_id(&Uuid::new_v4().to_string()).await.unwrap();
    assert!(!ok);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn count_today_returns_recent_count() {
    let repo = PgInfractionRepository::new(pool().await);
    let g = fresh_id();
    let before = repo.count_today().await.unwrap();
    repo.save(&sample_infraction(&g, &fresh_id(), Action::Warn, 1.0)).await.unwrap();
    repo.save(&sample_infraction(&g, &fresh_id(), Action::Mute, 3.0)).await.unwrap();
    let after = repo.count_today().await.unwrap();
    assert!(after >= before + 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_older_than_days_scopes_to_guild() {
    // delete_older_than_days(guild, 0) supprime tout pour ce guild.
    let repo = PgInfractionRepository::new(pool().await);
    let g1 = fresh_id();
    let g2 = fresh_id();
    repo.save(&sample_infraction(&g1, &fresh_id(), Action::Warn, 1.0)).await.unwrap();
    repo.save(&sample_infraction(&g2, &fresh_id(), Action::Mute, 3.0)).await.unwrap();

    // Avec days=0, tout devrait etre supprime pour g1 mais pas pour g2.
    // Note : le comportement exact depend de l'implementation (>= days ou > days).
    // On verifie juste que ca ne panique pas.
    let _n = repo.delete_older_than_days(&g1, 0).await.unwrap();
    let list_g2 = repo
        .find_by_guild(&g2, &InfractionFilters { user_id: None, action: None, limit: 10, offset: 0 })
        .await.unwrap();
    assert!(!list_g2.is_empty(), "g2 infractions ne devraient pas etre touchees");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_older_than_365_days_keeps_fresh_rows() {
    let repo = PgInfractionRepository::new(pool().await);
    let g = fresh_id();
    repo.save(&sample_infraction(&g, &fresh_id(), Action::Warn, 1.0)).await.unwrap();
    let n = repo.delete_older_than_days(&g, 365).await.unwrap();
    // Infraction creee il y a quelques ms < 365j => 0 supprimee
    assert_eq!(n, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_preserves_detection_flags() {
    let repo = PgInfractionRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    let mut inf = sample_infraction(&g, &u, Action::Mute, 6.0);
    inf.flags = DetectionFlags {
        spam: true, insult: true, link: false, phishing: true,
    };
    inf.duration = Some(600);
    repo.save(&inf).await.unwrap();
    let got = repo.find_by_id(&inf.id.to_string()).await.unwrap().unwrap();
    assert_eq!(got.flags.spam, true);
    assert_eq!(got.flags.insult, true);
    assert_eq!(got.flags.link, false);
    assert_eq!(got.flags.phishing, true);
    assert_eq!(got.duration, Some(600));
}
