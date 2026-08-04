//! Tests d'integration postgres pour 3 repos mid-size :
//! role_panel, welcome_config. Pure plomberie.

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::community::role_panel_repository::PgRolePanelRepository;
use sentinel_api::adapters::outbound::postgres::community::welcome_config_repository::PgWelcomeConfigRepository;
use sentinel_api::ports::outbound::community::role_panel_repository::RolePanelRepository;
use sentinel_api::ports::outbound::community::welcome_config_repository::WelcomeConfigData;
use sentinel_api::ports::outbound::community::welcome_config_repository::WelcomeConfigRepository;
use sentinel_core::domain::entities::community::role_panel::AutoRole;
use sentinel_core::domain::entities::community::role_panel::RolePanel;
use sentinel_core::domain::entities::community::role_panel::RolePanelEntry;
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

// ══════════════════════════════════════════════════════════
// RolePanel
// ══════════════════════════════════════════════════════════

fn sample_panel(guild: &str) -> RolePanel {
    let now = Utc::now();
    RolePanel {
        id: Uuid::new_v4(),
        guild_id: guild.into(),
        channel_id: "chan1".into(),
        message_id: None,
        title: "Role Picker".into(),
        description: "Pick your roles".into(),
        mode: "unique".into(),
        max_roles: Some(3),
        enabled: true,
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn role_panel_save_and_find() {
    let repo = PgRolePanelRepository::new(pool().await);
    let g = fresh_id();
    let p = sample_panel(&g);
    repo.save_panel(&p).await.unwrap();
    let detail = repo.find_panel(&p.id.to_string()).await.unwrap().unwrap();
    assert_eq!(detail.panel.title, "Role Picker");
    assert_eq!(detail.panel.max_roles, Some(3));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn role_panel_find_not_found_returns_none() {
    let repo = PgRolePanelRepository::new(pool().await);
    let id = Uuid::new_v4().to_string();
    assert!(repo.find_panel(&id).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn role_panel_save_entries_and_retrieve() {
    let repo = PgRolePanelRepository::new(pool().await);
    let g = fresh_id();
    let p = sample_panel(&g);
    repo.save_panel(&p).await.unwrap();
    let entries = vec![
        RolePanelEntry {
            id: Uuid::new_v4(),
            panel_id: p.id,
            role_id: "r1".into(),
            role_name: "Dev".into(),
            emoji: None,
            label: "Developer".into(),
            style: "primary".into(),
            position: 0,
        },
        RolePanelEntry {
            id: Uuid::new_v4(),
            panel_id: p.id,
            role_id: "r2".into(),
            role_name: "Gamer".into(),
            emoji: Some("game".into()),
            label: "Gaming".into(),
            style: "secondary".into(),
            position: 1,
        },
    ];
    repo.save_entries(&entries).await.unwrap();
    let detail = repo.find_panel(&p.id.to_string()).await.unwrap().unwrap();
    assert_eq!(detail.entries.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn role_panel_find_by_guild_returns_all() {
    let repo = PgRolePanelRepository::new(pool().await);
    let g = fresh_id();
    repo.save_panel(&sample_panel(&g)).await.unwrap();
    repo.save_panel(&sample_panel(&g)).await.unwrap();
    let panels = repo.find_panels_by_guild(&g).await.unwrap();
    assert_eq!(panels.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn role_panel_update_message_id() {
    let repo = PgRolePanelRepository::new(pool().await);
    let g = fresh_id();
    let p = sample_panel(&g);
    repo.save_panel(&p).await.unwrap();
    repo.update_message_id(&p.id.to_string(), "msg-123")
        .await
        .unwrap();
    let detail = repo.find_panel(&p.id.to_string()).await.unwrap().unwrap();
    assert_eq!(detail.panel.message_id.as_deref(), Some("msg-123"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn role_panel_delete_removes() {
    let repo = PgRolePanelRepository::new(pool().await);
    let g = fresh_id();
    let p = sample_panel(&g);
    repo.save_panel(&p).await.unwrap();
    repo.delete_panel(&p.id.to_string()).await.unwrap();
    assert!(repo.find_panel(&p.id.to_string()).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn role_panel_find_by_message() {
    let repo = PgRolePanelRepository::new(pool().await);
    let g = fresh_id();
    let p = sample_panel(&g);
    repo.save_panel(&p).await.unwrap();
    // message_id est VARCHAR(20) — 8 chars suffisent.
    let msg = format!("m-{:08x}", Uuid::new_v4().as_u128() as u32);
    repo.update_message_id(&p.id.to_string(), &msg)
        .await
        .unwrap();
    let found = repo.find_panel_by_message(&msg).await.unwrap().unwrap();
    assert_eq!(found.panel.id, p.id);
}

// ── Auto-roles ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_role_save_find_delete() {
    let repo = PgRolePanelRepository::new(pool().await);
    let g = fresh_id();
    let ar = AutoRole {
        id: Uuid::new_v4(),
        guild_id: g.clone().into(),
        role_id: "role1".into(),
        role_name: "Welcome".into(),
        delay_secs: 60,
        enabled: true,
    };
    repo.save_auto_role(&ar).await.unwrap();
    assert_eq!(repo.find_auto_roles(&g).await.unwrap().len(), 1);
    repo.delete_auto_role(&g, "role1").await.unwrap();
    assert!(repo.find_auto_roles(&g).await.unwrap().is_empty());
}

// ══════════════════════════════════════════════════════════
// WelcomeConfig
// ══════════════════════════════════════════════════════════

fn sample_welcome(guild: &str) -> WelcomeConfigData {
    WelcomeConfigData {
        guild_id: guild.into(),
        welcome_enabled: true,
        welcome_channel_id: Some("ch1".into()),
        welcome_message: "Hello!".into(),
        welcome_embed_color: "#7289DA".into(),
        welcome_dm_enabled: false,
        welcome_dm_message: "".into(),
        leave_enabled: false,
        leave_channel_id: None,
        leave_message: "".into(),
        rules_enabled: true,
        rules_channel_id: Some("rules-ch".into()),
        rules_message: "Read the rules".into(),
        rules_role_id: Some("verified".into()),
        rules_button_label: "Accept".into(),
        age_check_enabled: false,
        age_minimum: 0,
        unverified_role_id: None,
        age_modal_question: String::new(),
        age_ban_message: String::new(),
        age_min: 5,
        age_max: 120,
        age_ban_days_per_year: 365,
        age_ban_log_channel_id: None,
        leave_embed_color: "e74c3c".into(),
        rules_embed_color: "5865f2".into(),
        counter_enabled: false,
        counter_channel_id: None,
        counter_format: "{count}".into(),
        voice_counter_enabled: false,
        voice_counter_channel_id: None,
        voice_counter_format: "En Vocal : {count}".into(),
        anniversary_enabled: false,
        anniversary_channel_id: None,
        anniversary_message: "".into(),
        rejoin_message: "".into(),
        welcome_title: "".into(),
        welcome_image_url: "".into(),
        welcome_text_position: "below".into(),
        welcome_footer_text: "".into(),
        rejoin_title: "".into(),
        rejoin_image_url: "".into(),
        rejoin_footer_text: "".into(),
        leave_title: "".into(),
        leave_image_url: "".into(),
        leave_footer_text: "".into(),
        anniversary_title: "".into(),
        anniversary_image_url: "".into(),
        anniversary_footer_text: "".into(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn welcome_get_config_returns_defaults_when_absent() {
    let repo = PgWelcomeConfigRepository::new(pool().await);
    let g = fresh_id();
    // Pas de row — doit retourner une config par defaut (pas une erreur).
    let got = repo.get_config(&g).await;
    assert!(got.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn welcome_save_and_get_roundtrip() {
    let repo = PgWelcomeConfigRepository::new(pool().await);
    let g = fresh_id();
    let cfg = sample_welcome(&g);
    repo.save_config(&g, &cfg).await.unwrap();
    let got = repo.get_config(&g).await.unwrap();
    assert!(got.welcome_enabled);
    assert_eq!(got.welcome_channel_id.as_deref(), Some("ch1"));
    assert_eq!(got.welcome_message, "Hello!");
    assert!(got.rules_enabled);
    assert_eq!(got.rules_button_label, "Accept");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn welcome_save_is_upsert() {
    let repo = PgWelcomeConfigRepository::new(pool().await);
    let g = fresh_id();
    let mut cfg = sample_welcome(&g);
    repo.save_config(&g, &cfg).await.unwrap();
    cfg.welcome_message = "Bonjour!".into();
    cfg.welcome_enabled = false;
    repo.save_config(&g, &cfg).await.unwrap();
    let got = repo.get_config(&g).await.unwrap();
    assert!(!got.welcome_enabled);
    assert_eq!(got.welcome_message, "Bonjour!");
}

