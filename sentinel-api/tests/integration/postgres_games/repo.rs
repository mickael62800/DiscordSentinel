//! Tests d'integration postgres pour PgGameRepository.

use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::casino::game_repository::PgGameRepository;
use sentinel_api::ports::outbound::casino::game_repository::GameRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_empty_when_no_games() {
    let repo = PgGameRepository::new(pool().await);
    assert!(repo.list(&fresh_id()).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_and_list_returns_game() {
    let repo = PgGameRepository::new(pool().await);
    let g = fresh_id();
    let created = repo
        .create(&g, "Minecraft", "admin1", None, None, None)
        .await
        .unwrap();
    assert_eq!(created.game_name, "Minecraft");
    assert_eq!(created.guild_id, g);

    let list = repo.list(&g).await.unwrap();
    assert_eq!(list.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_with_emoji_category_role() {
    let repo = PgGameRepository::new(pool().await);
    let g = fresh_id();
    let created = repo
        .create(
            &g,
            "Valorant",
            "admin1",
            Some("emoji"),
            Some("FPS"),
            Some("role1"),
        )
        .await
        .unwrap();
    assert_eq!(created.emoji.as_deref(), Some("emoji"));
    assert_eq!(created.category.as_deref(), Some("FPS"));
    assert_eq!(created.role_id.as_deref(), Some("role1"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn find_by_name_returns_game() {
    let repo = PgGameRepository::new(pool().await);
    let g = fresh_id();
    repo.create(&g, "LoL", "admin1", None, None, None)
        .await
        .unwrap();
    let got = repo.find_by_name(&g, "LoL").await.unwrap().unwrap();
    assert_eq!(got.game_name, "LoL");
    // Cas absent
    assert!(repo
        .find_by_name(&g, "NonExistent")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_game_name() {
    let repo = PgGameRepository::new(pool().await);
    let g = fresh_id();
    let created = repo
        .create(&g, "OldName", "admin1", None, None, None)
        .await
        .unwrap();
    let updated = repo
        .update(&g, &created.id, Some("NewName"), None, None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.game_name, "NewName");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_emoji_set_and_clear() {
    let repo = PgGameRepository::new(pool().await);
    let g = fresh_id();
    let created = repo
        .create(&g, "G", "admin1", None, None, None)
        .await
        .unwrap();
    repo.update(&g, &created.id, None, Some(Some("emo")), None)
        .await
        .unwrap();
    let got = repo.find_by_name(&g, "G").await.unwrap().unwrap();
    assert_eq!(got.emoji.as_deref(), Some("emo"));
    // Clear : Some(None)
    repo.update(&g, &created.id, None, Some(None), None)
        .await
        .unwrap();
    let got = repo.find_by_name(&g, "G").await.unwrap().unwrap();
    assert!(got.emoji.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_not_found_returns_none() {
    let repo = PgGameRepository::new(pool().await);
    let g = fresh_id();
    let bogus_id = Uuid::new_v4().to_string();
    let got = repo
        .update(&g, &bogus_id, Some("X"), None, None)
        .await
        .unwrap();
    assert!(got.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_returns_true_on_existing() {
    let repo = PgGameRepository::new(pool().await);
    let g = fresh_id();
    let created = repo
        .create(&g, "G", "admin1", None, None, None)
        .await
        .unwrap();
    assert!(repo.delete(&g, &created.id).await.unwrap());
    // 2e fois -> false.
    assert!(!repo.delete(&g, &created.id).await.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_role_id_updates_and_clears() {
    let repo = PgGameRepository::new(pool().await);
    let g = fresh_id();
    let created = repo
        .create(&g, "G", "admin1", None, None, None)
        .await
        .unwrap();
    let got = repo
        .set_role_id(&g, &created.id, Some("role99"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.role_id.as_deref(), Some("role99"));
    let got = repo
        .set_role_id(&g, &created.id, None)
        .await
        .unwrap()
        .unwrap();
    assert!(got.role_id.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_by_category_filters() {
    let repo = PgGameRepository::new(pool().await);
    let g = fresh_id();
    repo.create(&g, "L", "a", None, Some("MOBA"), None)
        .await
        .unwrap();
    repo.create(&g, "V", "a", None, Some("FPS"), None)
        .await
        .unwrap();
    repo.create(&g, "M", "a", None, Some("FPS"), None)
        .await
        .unwrap();

    let moba = repo.list_by_category(&g, Some("MOBA")).await.unwrap();
    assert_eq!(moba.len(), 1);
    let fps = repo.list_by_category(&g, Some("FPS")).await.unwrap();
    assert_eq!(fps.len(), 2);
    // None = WHERE category IS NULL (uncategorized).
    repo.create(&g, "Uncategorized", "a", None, None, None)
        .await
        .unwrap();
    let null_cat = repo.list_by_category(&g, None).await.unwrap();
    assert_eq!(null_cat.len(), 1);
    assert_eq!(null_cat[0].game_name, "Uncategorized");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn save_and_find_panel_by_message() {
    let repo = PgGameRepository::new(pool().await);
    let g = fresh_id();
    let panel = repo
        .save_panel(&g, "chan1", "msg1", Some("MOBA"))
        .await
        .unwrap();
    assert_eq!(panel.channel_id, "chan1");
    assert_eq!(panel.message_id, "msg1");

    let got = repo
        .find_panel_by_message(&g, "msg1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.channel_id, "chan1");
    assert!(repo
        .find_panel_by_message(&g, "nonexistent")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_panels_returns_all_for_guild() {
    let repo = PgGameRepository::new(pool().await);
    let g = fresh_id();
    repo.save_panel(&g, "c1", "m1", None).await.unwrap();
    repo.save_panel(&g, "c2", "m2", Some("FPS")).await.unwrap();
    let panels = repo.list_panels(&g).await.unwrap();
    assert_eq!(panels.len(), 2);
}
