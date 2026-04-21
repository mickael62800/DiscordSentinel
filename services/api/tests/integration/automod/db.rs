//! Tests d'integration REELS pour le pipeline automod (avec PostgreSQL).
//! Verifie le flow complet : config guild → rules → scoring → infractions → conduct.

use std::sync::Arc;
use sqlx::PgPool;

use sentinel_api::adapters::outbound::postgres::{
    PgRuleRepository, PgInfractionRepository, PgIaConfigRepository,
};
use sentinel_api::application::AnalyzeMessageService;
use sentinel_api::domain::entities::IaConfig;
use sentinel_api::adapters::outbound::{InferenceService, TextTokenizer};
use sentinel_api::domain::services::InferenceRateLimiter;
use sentinel_api::domain::value_objects::DetectionFlags;
use sentinel_api::ports::inbound::{AnalyzeMessageCommand, AnalyzeMessageUseCase, ContextMessageEntry};
use sentinel_api::ports::outbound::{IaConfigRepository, RuleRepository};

async fn setup_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into());
    PgPool::connect(&url).await.expect("Impossible de se connecter a la base de test")
}

fn unique_guild() -> String {
    format!("{}", uuid::Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128)
}

/// Guild ID court pour les tables avec varchar(20) comme bot_guild_config.
fn short_guild_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{}", rng.gen_range(10000000000000000u64..99999999999999999u64))
}

// ══════════════════════════════════════════════════════════
//  Bot config — guild configuration
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn bot_config_upsert_and_read() {
    let pool = setup_pool().await;
    let gid = short_guild_id();

    // Insert
    sqlx::query(
        r#"INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
           VALUES ($1, 'automod-bot', 'flood_max_messages', '10')
           ON CONFLICT (guild_id, bot_name, config_key) DO UPDATE SET config_value = EXCLUDED.config_value"#,
    ).bind(&gid).execute(&pool).await.unwrap();

    // Read
    let val = sqlx::query_as::<_, (String,)>(
        "SELECT config_value FROM bot_guild_config WHERE guild_id = $1 AND bot_name = 'automod-bot' AND config_key = 'flood_max_messages'",
    ).bind(&gid).fetch_one(&pool).await.unwrap().0;

    assert_eq!(val, "10");

    // Update
    sqlx::query(
        r#"INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
           VALUES ($1, 'automod-bot', 'flood_max_messages', '20')
           ON CONFLICT (guild_id, bot_name, config_key) DO UPDATE SET config_value = EXCLUDED.config_value"#,
    ).bind(&gid).execute(&pool).await.unwrap();

    let val = sqlx::query_as::<_, (String,)>(
        "SELECT config_value FROM bot_guild_config WHERE guild_id = $1 AND config_key = 'flood_max_messages'",
    ).bind(&gid).fetch_one(&pool).await.unwrap().0;

    assert_eq!(val, "20");
}

#[tokio::test]
async fn bot_config_multiple_keys_per_guild() {
    let pool = setup_pool().await;
    let gid = short_guild_id();

    for (key, val) in &[("enabled", "true"), ("flood_max_messages", "5"), ("mute_duration_secs", "600")] {
        sqlx::query(
            r#"INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
               VALUES ($1, 'automod-bot', $2, $3)"#,
        ).bind(&gid).bind(key).bind(val).execute(&pool).await.unwrap();
    }

    let count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM bot_guild_config WHERE guild_id = $1 AND bot_name = 'automod-bot'",
    ).bind(&gid).fetch_one(&pool).await.unwrap().0;

    assert_eq!(count, 3);
}

#[tokio::test]
async fn bot_config_isolated_per_guild() {
    let pool = setup_pool().await;
    let gid1 = short_guild_id();
    let gid2 = short_guild_id();

    sqlx::query(
        r#"INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
           VALUES ($1, 'automod-bot', 'enabled', 'false')"#,
    ).bind(&gid1).execute(&pool).await.unwrap();

    let exists = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM bot_guild_config WHERE guild_id = $1 AND config_key = 'enabled'",
    ).bind(&gid2).fetch_one(&pool).await.unwrap().0;

    assert_eq!(exists, 0, "Guild 2 ne doit pas heriter de guild 1");
}

// ══════════════════════════════════════════════════════════
//  Rules — scoring weights per guild
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn rules_per_guild_with_weights() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let repo = PgRuleRepository::new(pool.clone());

    // Creer 2 rules
    sqlx::query(
        r#"INSERT INTO rules (id, guild_id, flag_type, weight, threshold_warn, threshold_delete, threshold_mute, threshold_ban, enabled, created_at, updated_at)
           VALUES (gen_random_uuid(), $1, 'spam', 3.0, 2.0, 4.0, 6.0, 9.0, true, NOW(), NOW())"#,
    ).bind(&gid).execute(&pool).await.unwrap();
    sqlx::query(
        r#"INSERT INTO rules (id, guild_id, flag_type, weight, threshold_warn, threshold_delete, threshold_mute, threshold_ban, enabled, created_at, updated_at)
           VALUES (gen_random_uuid(), $1, 'insult', 5.0, 2.0, 4.0, 6.0, 9.0, true, NOW(), NOW())"#,
    ).bind(&gid).execute(&pool).await.unwrap();

    let rules = repo.find_by_guild(&gid).await.unwrap();
    assert_eq!(rules.len(), 2);

    let spam_rule = rules.iter().find(|r| r.flag_type.as_str() == "spam").unwrap();
    assert!((spam_rule.weight - 3.0).abs() < 0.01);
    assert!(spam_rule.enabled);
}

#[tokio::test]
async fn disabled_rule_still_returned() {
    let pool = setup_pool().await;
    let gid = unique_guild();
    let repo = PgRuleRepository::new(pool.clone());

    sqlx::query(
        r#"INSERT INTO rules (id, guild_id, flag_type, weight, threshold_warn, threshold_delete, threshold_mute, threshold_ban, enabled, created_at, updated_at)
           VALUES (gen_random_uuid(), $1, 'phishing', 8.0, 2.0, 4.0, 6.0, 9.0, false, NOW(), NOW())"#,
    ).bind(&gid).execute(&pool).await.unwrap();

    let rules = repo.find_by_guild(&gid).await.unwrap();
    assert_eq!(rules.len(), 1);
    assert!(!rules[0].enabled);
}

// ══════════════════════════════════════════════════════════
//  Full pipeline : flags → scoring → infraction persisted
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn full_analyze_spam_creates_infraction() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    // Setup rules
    sqlx::query(
        r#"INSERT INTO rules (id, guild_id, flag_type, weight, threshold_warn, threshold_delete, threshold_mute, threshold_ban, enabled, created_at, updated_at)
           VALUES (gen_random_uuid(), $1, 'spam', 3.0, 2.0, 4.0, 6.0, 9.0, true, NOW(), NOW())"#,
    ).bind(&gid).execute(&pool).await.unwrap();

    // Build service (no IA inference — just scoring)
    let rule_repo = Arc::new(PgRuleRepository::new(pool.clone()));
    let infraction_repo = Arc::new(PgInfractionRepository::new(pool.clone()));
    let ia_config_repo = Arc::new(PgIaConfigRepository::new(pool.clone()));

    // Mock conduct UC (minimal) et cache
    let conduct_uc = Arc::new(StubConductUC);
    let cache = Arc::new(NoCache);
    let inference = Arc::new(InferenceService::new(None, None));
    let tokenizer = Arc::new(TextTokenizer::new(None, 512));
    let limiter = Arc::new(InferenceRateLimiter::new(4, 0));

    let service = AnalyzeMessageService::new(
        rule_repo, infraction_repo, cache, conduct_uc, ia_config_repo, limiter,
    ).with_text_inference(inference, tokenizer);

    // Analyze a spam message
    let cmd = AnalyzeMessageCommand {
        guild_id: gid.clone(),
        channel_id: "555555555555555555".into(),
        user_id: "444444444444444444".into(),
        username: "Spammer".into(),
        content: "buy buy buy buy buy".into(),
        flags: DetectionFlags { spam: true, insult: false, link: false, phishing: false },
        message_id: "msg_test_1".into(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        context_messages: vec![],
    };

    let result = service.analyze(cmd).await.unwrap();
    assert_eq!(result.action.as_str(), "warn", "Spam seul devrait trigger warn (score=3.0, threshold_warn=2.0)");

    // Verifier que l'infraction est persistee en DB
    let count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM infractions WHERE guild_id = $1 AND action = 'warn'",
    ).bind(&gid).fetch_one(&pool).await.unwrap().0;

    assert_eq!(count, 1);
}

#[tokio::test]
async fn full_analyze_spam_plus_insult_escalates() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    sqlx::query(
        r#"INSERT INTO rules (id, guild_id, flag_type, weight, threshold_warn, threshold_delete, threshold_mute, threshold_ban, enabled, created_at, updated_at)
           VALUES (gen_random_uuid(), $1, 'spam', 3.0, 2.0, 4.0, 6.0, 9.0, true, NOW(), NOW())"#,
    ).bind(&gid).execute(&pool).await.unwrap();
    sqlx::query(
        r#"INSERT INTO rules (id, guild_id, flag_type, weight, threshold_warn, threshold_delete, threshold_mute, threshold_ban, enabled, created_at, updated_at)
           VALUES (gen_random_uuid(), $1, 'insult', 5.0, 2.0, 4.0, 6.0, 9.0, true, NOW(), NOW())"#,
    ).bind(&gid).execute(&pool).await.unwrap();

    let service = build_analyze_service(pool.clone());

    let cmd = AnalyzeMessageCommand {
        guild_id: gid.clone(),
        channel_id: "555555555555555555".into(),
        user_id: "444444444444444444".into(),
        username: "BadUser".into(),
        content: "connard connard connard".into(),
        flags: DetectionFlags { spam: true, insult: true, link: false, phishing: false },
        message_id: "msg_test_2".into(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        context_messages: vec![],
    };

    let result = service.analyze(cmd).await.unwrap();
    // spam(3.0) + insult(5.0) = 8.0 → mute (threshold 6.0)
    assert_eq!(result.action.as_str(), "mute", "Spam+Insult (score=8.0) devrait trigger mute");
}

#[tokio::test]
async fn full_analyze_no_flags_returns_none() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    // Desactiver l'IA pour cette guild
    let ia_repo = PgIaConfigRepository::new(pool.clone());
    let config = IaConfig {
        guild_id: gid.clone(),
        text_enabled: false,
        text_threshold: 0.5,
        vision_enabled: false,
        vision_threshold: 0.5,
        context_dampening: 0.65,
        context_format: "natural".into(),
        context_max_messages: 3,
        context_max_chars: 200,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    ia_repo.save(&config).await.unwrap();

    let service = build_analyze_service(pool.clone());

    let cmd = AnalyzeMessageCommand {
        guild_id: gid.clone(),
        channel_id: "555555555555555555".into(),
        user_id: "444444444444444444".into(),
        username: "NormalUser".into(),
        content: "Salut tout le monde".into(),
        flags: DetectionFlags { spam: false, insult: false, link: false, phishing: false },
        message_id: "msg_test_3".into(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        context_messages: vec![],
    };

    let result = service.analyze(cmd).await.unwrap();
    assert_eq!(result.action.as_str(), "none");
}

#[tokio::test]
async fn full_analyze_with_context_messages() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    sqlx::query(
        r#"INSERT INTO rules (id, guild_id, flag_type, weight, threshold_warn, threshold_delete, threshold_mute, threshold_ban, enabled, created_at, updated_at)
           VALUES (gen_random_uuid(), $1, 'spam', 3.0, 2.0, 4.0, 6.0, 9.0, true, NOW(), NOW())"#,
    ).bind(&gid).execute(&pool).await.unwrap();

    let service = build_analyze_service(pool.clone());

    let cmd = AnalyzeMessageCommand {
        guild_id: gid.clone(),
        channel_id: "555555555555555555".into(),
        user_id: "444444444444444444".into(),
        username: "User".into(),
        content: "spam spam spam".into(),
        flags: DetectionFlags { spam: true, insult: false, link: false, phishing: false },
        message_id: "msg_test_4".into(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        context_messages: vec![
            ContextMessageEntry { username: "Alice".into(), content: "Salut".into() },
            ContextMessageEntry { username: "Bob".into(), content: "Ca va ?".into() },
        ],
    };

    let result = service.analyze(cmd).await.unwrap();
    // Le scoring local n'utilise pas le contexte — il reste a warn
    assert_eq!(result.action.as_str(), "warn");

    // L'infraction est persistee avec le bon guild
    let inf = sqlx::query_as::<_, (String, String)>(
        "SELECT action, reason FROM infractions WHERE guild_id = $1 ORDER BY created_at DESC LIMIT 1",
    ).bind(&gid).fetch_one(&pool).await.unwrap();

    assert_eq!(inf.0, "warn");
    assert!(inf.1.contains("spam") || inf.1.contains("Spam"));
}

#[tokio::test]
async fn ia_config_dampening_applied() {
    let pool = setup_pool().await;
    let gid = unique_guild();

    // Config IA avec dampening fort
    let ia_repo = PgIaConfigRepository::new(pool.clone());
    let config = IaConfig {
        guild_id: gid.clone(),
        text_enabled: true,
        text_threshold: 0.5,
        vision_enabled: false,
        vision_threshold: 0.5,
        context_dampening: 0.3, // dampening tres fort
        context_format: "natural".into(),
        context_max_messages: 5,
        context_max_chars: 300,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let saved = ia_repo.save(&config).await.unwrap();

    assert!((saved.context_dampening - 0.3).abs() < 0.01);
    assert_eq!(saved.context_max_messages, 5);
}

// ══════════════════════════════════════════════════════════
//  Helpers
// ══════════════════════════════════════════════════════════

fn build_analyze_service(pool: PgPool) -> AnalyzeMessageService {
    let rule_repo = Arc::new(PgRuleRepository::new(pool.clone()));
    let infraction_repo = Arc::new(PgInfractionRepository::new(pool.clone()));
    let ia_config_repo = Arc::new(PgIaConfigRepository::new(pool.clone()));
    let conduct_uc = Arc::new(StubConductUC);
    let cache = Arc::new(NoCache);
    let inference = Arc::new(InferenceService::new(None, None));
    let tokenizer = Arc::new(TextTokenizer::new(None, 512));
    let limiter = Arc::new(InferenceRateLimiter::new(4, 0));

    AnalyzeMessageService::new(
        rule_repo, infraction_repo, cache, conduct_uc, ia_config_repo, limiter,
    ).with_text_inference(inference, tokenizer)
}

// Stub minimal pour ManageConductUseCase
struct StubConductUC;

#[async_trait::async_trait]
impl sentinel_api::ports::inbound::ManageConductUseCase for StubConductUC {
    async fn get_config(&self, _: &str) -> Result<sentinel_api::domain::entities::ConductConfig, sentinel_api::domain::errors::DomainError> {
        Ok(sentinel_api::domain::entities::ConductConfig::default_for_guild(""))
    }
    async fn save_config(&self, _: sentinel_api::ports::inbound::SaveConductConfigCommand) -> Result<sentinel_api::domain::entities::ConductConfig, sentinel_api::domain::errors::DomainError> {
        unimplemented!()
    }
    async fn get_points(&self, _: &str, _: &str) -> Result<sentinel_api::domain::entities::UserConductPoints, sentinel_api::domain::errors::DomainError> {
        unimplemented!()
    }
    async fn add_points(&self, _: sentinel_api::ports::inbound::AddPointsCommand) -> Result<sentinel_api::domain::entities::UserConductPoints, sentinel_api::domain::errors::DomainError> {
        unimplemented!()
    }
    async fn deduct_points(&self, _: sentinel_api::ports::inbound::DeductPointsCommand) -> Result<sentinel_api::domain::entities::UserConductPoints, sentinel_api::domain::errors::DomainError> {
        // Retourner un stub minimal — le test n'utilise pas la valeur
        Err(sentinel_api::domain::errors::DomainError::NotFound("stub".into()))
    }
    async fn get_leaderboard(&self, _: &str, _: i64) -> Result<Vec<sentinel_api::domain::entities::UserConductPoints>, sentinel_api::domain::errors::DomainError> {
        unimplemented!()
    }
    async fn get_points_log(&self, _: &str, _: &str, _: i64) -> Result<Vec<sentinel_api::domain::entities::ConductPointsLog>, sentinel_api::domain::errors::DomainError> {
        unimplemented!()
    }
    async fn run_regen(&self) -> Result<u64, sentinel_api::domain::errors::DomainError> {
        Ok(0)
    }
}

// Stub cache qui ne cache rien (force la lecture DB a chaque fois)
struct NoCache;

#[async_trait::async_trait]
impl sentinel_api::ports::outbound::CachePort for NoCache {
    async fn get_rules(&self, _: &str) -> Result<Option<Vec<sentinel_api::domain::entities::Rule>>, sentinel_api::domain::errors::DomainError> {
        Ok(None) // Force lecture DB a chaque fois
    }
    async fn set_rules(&self, _: &str, _: &[sentinel_api::domain::entities::Rule]) -> Result<(), sentinel_api::domain::errors::DomainError> {
        Ok(())
    }
    async fn invalidate_rules(&self, _: &str) -> Result<(), sentinel_api::domain::errors::DomainError> {
        Ok(())
    }
    async fn get_json(&self, _: &str) -> Result<Option<String>, sentinel_api::domain::errors::DomainError> {
        Ok(None)
    }
    async fn set_json(&self, _: &str, _: &str, _: u64) -> Result<(), sentinel_api::domain::errors::DomainError> {
        Ok(())
    }
    async fn invalidate(&self, _: &str) -> Result<(), sentinel_api::domain::errors::DomainError> {
        Ok(())
    }
    async fn invalidate_pattern(&self, _: &str) -> Result<(), sentinel_api::domain::errors::DomainError> {
        Ok(())
    }
}
