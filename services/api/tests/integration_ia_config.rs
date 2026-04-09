//! Tests d'integration REELS pour la config IA (avec PostgreSQL).
//! Verifie les nouveaux champs de contexte conversationnel.

use sqlx::PgPool;
use sentinel_api::adapters::outbound::postgres::PgIaConfigRepository;
use sentinel_api::ports::outbound::IaConfigRepository;
use sentinel_api::domain::entities::IaConfig;

async fn setup_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.expect("Impossible de se connecter a la base de test")
}

fn unique_guild() -> String {
    format!("test_{}", uuid::Uuid::new_v4().simple())
}

#[tokio::test]
async fn ia_config_default_values() {
    let pool = setup_pool().await;
    let repo = PgIaConfigRepository::new(pool);
    let gid = unique_guild();

    // Pas de config enregistree — default
    let config = repo.get(&gid).await.unwrap();
    assert!(config.is_none());

    let default = IaConfig::default_for_guild(&gid);
    assert_eq!(default.context_dampening, 0.65);
    assert_eq!(default.context_format, "natural");
    assert_eq!(default.context_max_messages, 3);
    assert_eq!(default.context_max_chars, 200);
}

#[tokio::test]
async fn ia_config_save_and_read_context_fields() {
    let pool = setup_pool().await;
    let repo = PgIaConfigRepository::new(pool);
    let gid = unique_guild();

    let config = IaConfig {
        guild_id: gid.clone(),
        text_enabled: true,
        text_threshold: 0.6,
        vision_enabled: false,
        vision_threshold: 0.4,
        context_dampening: 0.8,
        context_format: "tagged".to_string(),
        context_max_messages: 5,
        context_max_chars: 400,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let saved = repo.save(&config).await.unwrap();
    assert_eq!(saved.context_dampening, 0.8);
    assert_eq!(saved.context_format, "tagged");
    assert_eq!(saved.context_max_messages, 5);
    assert_eq!(saved.context_max_chars, 400);

    // Relire
    let loaded = repo.get(&gid).await.unwrap().unwrap();
    assert_eq!(loaded.context_dampening, 0.8);
    assert_eq!(loaded.context_format, "tagged");
    assert_eq!(loaded.context_max_messages, 5);
    assert_eq!(loaded.context_max_chars, 400);
    assert!(!loaded.vision_enabled);
}

#[tokio::test]
async fn ia_config_update_preserves_context_fields() {
    let pool = setup_pool().await;
    let repo = PgIaConfigRepository::new(pool);
    let gid = unique_guild();

    // Premier save
    let config = IaConfig {
        guild_id: gid.clone(),
        text_enabled: true,
        text_threshold: 0.5,
        vision_enabled: true,
        vision_threshold: 0.5,
        context_dampening: 0.65,
        context_format: "natural".to_string(),
        context_max_messages: 3,
        context_max_chars: 200,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    repo.save(&config).await.unwrap();

    // Update avec nouvelles valeurs
    let updated = IaConfig {
        context_dampening: 0.5,
        context_format: "tagged".to_string(),
        context_max_messages: 7,
        context_max_chars: 300,
        ..config
    };
    let result = repo.save(&updated).await.unwrap();
    assert_eq!(result.context_dampening, 0.5);
    assert_eq!(result.context_format, "tagged");
    assert_eq!(result.context_max_messages, 7);
    assert_eq!(result.context_max_chars, 300);
}
