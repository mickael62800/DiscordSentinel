//! Tests d'integration pour ManageSecurityService avec vrais repos PG.
//! Couvre notamment alt_detection (via find_bans réels) et report_event complet.

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::audit::security_event_repository::PgSecurityEventRepository;
use sentinel_api::adapters::outbound::postgres::audit::watched_user_repository::PgWatchedUserRepository;
use sentinel_api::adapters::outbound::postgres::moderation::moderation_repository::PgModerationRepository;
use sentinel_api::adapters::outbound::postgres::system::bot_config_repository::PgBotConfigRepository;
use sentinel_core::application::audit::manage_audit_logs_service::ManageAuditLogsService;
use sentinel_core::application::audit::manage_security_service::ManageSecurityService;
use sentinel_core::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase;
use sentinel_core::ports::inbound::audit::manage_security::AnalyzeNewMemberCommand;
use sentinel_core::ports::inbound::audit::manage_security::ManageSecurityUseCase;
use sentinel_core::ports::inbound::audit::manage_security::ReportSecurityEventCommand;
use sentinel_core::ports::outbound::system::cache::CachePort;
use sentinel_core::domain::entities::system::rule::Rule;
use sentinel_core::domain::errors::DomainError;

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

struct NoopCache;
#[async_trait]
impl CachePort for NoopCache {
    async fn get_rules(&self, _: &str) -> Result<Option<Vec<Rule>>, DomainError> {
        Ok(None)
    }
    async fn set_rules(&self, _: &str, _: &[Rule]) -> Result<(), DomainError> {
        Ok(())
    }
    async fn invalidate_rules(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_json(&self, _: &str) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
    async fn set_json(&self, _: &str, _: &str, _: u64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn invalidate(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn invalidate_pattern(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

async fn build() -> (ManageSecurityService, PgPool) {
    let p = pool().await;
    let repo = Arc::new(PgSecurityEventRepository::new(p.clone()));
    let watched = Arc::new(PgWatchedUserRepository::new(p.clone()));
    let bot_config = Arc::new(PgBotConfigRepository::new(p.clone()));
    let moderation = Arc::new(PgModerationRepository::new(p.clone()));
    let audit_uc = Arc::new(ManageAuditLogsService::new(
        Arc::new(sentinel_api::adapters::outbound::postgres::audit::audit_log_repository::PgAuditLogRepository::new(p.clone())),
    ));
    let svc =
        ManageSecurityService::new(repo, Arc::new(NoopCache), watched, bot_config, moderation)
            .with_audit_logs_uc(audit_uc as Arc<dyn ManageAuditLogsUseCase>);
    (svc, p)
}

async fn seed_ban(p: &PgPool, guild: &str, target_name: &str) {
    // Phase 4 : find_bans lit depuis audit_logs avec event_type='mod_ban*'.
    sqlx::query(
        "INSERT INTO audit_logs (id, guild_id, event_type, actor_id, actor_name, \
          target_id, target_name, channel_id, channel_name, details, created_at) \
         VALUES ($1, $2, 'mod_ban', 'mod', 'Mod', $3, $4, NULL, NULL, '{}'::jsonb, NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(guild)
    .bind(fresh_id())
    .bind(target_name)
    .execute(p)
    .await
    .unwrap();
}

async fn set_config(p: &PgPool, guild: &str, key: &str, value: &str) {
    sqlx::query(
        "INSERT INTO bot_guild_config (id, guild_id, bot_name, config_key, config_value, updated_at) \
         VALUES ($1, $2, 'security-bot', $3, $4, NOW()) \
         ON CONFLICT (guild_id, bot_name, config_key) DO UPDATE SET config_value = $4, updated_at = NOW()",
    )
    .bind(Uuid::new_v4()).bind(guild).bind(key).bind(value)
    .execute(p).await.unwrap();
}

// ── report_event + audit log persistance ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_event_persists_in_db() {
    let (svc, p) = build().await;
    let g = fresh_id();
    let event = svc
        .report_event(ReportSecurityEventCommand {
            guild_id: g.clone().into(),
            event_type: "raid".into(),
            severity: "high".into(),
            description: "pattern".into(),
            user_ids: vec![fresh_id(), fresh_id()],
        })
        .await
        .unwrap();
    assert_eq!(event.event_type, "raid");

    // Verifier persistence audit_logs
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE guild_id = $1 AND event_type = 'security_raid'",
    )
    .bind(&g)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_events_with_guild_returns_reported() {
    let (svc, _) = build().await;
    let g = fresh_id();
    svc.report_event(ReportSecurityEventCommand {
        guild_id: g.clone().into(),
        event_type: "scan".into(),
        severity: "low".into(),
        description: "".into(),
        user_ids: vec![fresh_id()],
    })
    .await
    .unwrap();
    let events = svc.list_events(Some(&g)).await.unwrap();
    assert!(!events.is_empty());
    assert!(events.iter().any(|e| e.event_type == "scan"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_events_without_guild_returns_all() {
    let (svc, _) = build().await;
    let g = fresh_id();
    svc.report_event(ReportSecurityEventCommand {
        guild_id: g.clone().into(),
        event_type: "raid".into(),
        severity: "high".into(),
        description: "".into(),
        user_ids: vec![],
    })
    .await
    .unwrap();
    let all = svc.list_events(None).await.unwrap();
    assert!(!all.is_empty());
}

// ── analyze_new_member alt detection ──

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analyze_new_member_alt_detection_flags_similar_username() {
    let (svc, p) = build().await;
    let g = fresh_id();
    // Activer alt detection avec distance de nom a 3 (assez permissif).
    set_config(&p, &g, "alt_detection_enabled", "true").await;
    set_config(&p, &g, "alt_name_distance", "3").await;
    set_config(&p, &g, "min_account_age_secs", "0").await;

    // Seed un ban recent avec username tres proche.
    seed_ban(&p, &g, "alice_bad").await;

    let cmd = AnalyzeNewMemberCommand {
        guild_id: g.clone().into(),
        user_id: fresh_id().into(),
        username: "alice_bad2".into(),
        has_avatar: true,
        account_created_timestamp: chrono::Utc::now().timestamp() - 86400 * 30,
        is_bot: false,
        recent_joins: vec![],
        is_velocity_raid: false,
    };
    let d = svc.analyze_new_member(cmd).await.unwrap();
    assert!(d.is_alt_account);
    assert_eq!(d.event_type, "alt_account_suspected");
    assert!(d.event_description.contains("alice_bad2"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analyze_new_member_alt_detection_no_match_when_names_differ() {
    let (svc, p) = build().await;
    let g = fresh_id();
    set_config(&p, &g, "alt_detection_enabled", "true").await;
    set_config(&p, &g, "min_account_age_secs", "0").await;
    seed_ban(&p, &g, "totally_different_name").await;

    let cmd = AnalyzeNewMemberCommand {
        guild_id: g.clone().into(),
        user_id: fresh_id().into(),
        username: "alice".into(),
        has_avatar: true,
        account_created_timestamp: chrono::Utc::now().timestamp() - 86400 * 30,
        is_bot: false,
        recent_joins: vec![],
        is_velocity_raid: false,
    };
    let d = svc.analyze_new_member(cmd).await.unwrap();
    assert!(!d.is_alt_account);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analyze_new_member_alt_detection_skipped_when_raid() {
    use sentinel_core::domain::services::audit::security_analyzer::JoinInfo;
    let (svc, p) = build().await;
    let g = fresh_id();
    set_config(&p, &g, "alt_detection_enabled", "true").await;
    set_config(&p, &g, "raid_pattern_enabled", "true").await;
    set_config(&p, &g, "raid_pattern_score_threshold", "0").await;
    seed_ban(&p, &g, "alice").await;

    let now = chrono::Utc::now().timestamp();
    let cmd = AnalyzeNewMemberCommand {
        guild_id: g.clone().into(),
        user_id: fresh_id().into(),
        username: "alice".into(),
        has_avatar: false,
        account_created_timestamp: now - 3600,
        is_bot: false,
        recent_joins: vec![
            JoinInfo {
                username: "alice01".into(),
                account_created_timestamp: now - 3600,
                has_avatar: false,
            },
            JoinInfo {
                username: "alice02".into(),
                account_created_timestamp: now - 3600,
                has_avatar: false,
            },
            JoinInfo {
                username: "alice03".into(),
                account_created_timestamp: now - 3600,
                has_avatar: false,
            },
        ],
        is_velocity_raid: false,
    };
    let d = svc.analyze_new_member(cmd).await.unwrap();
    assert!(d.is_raid);
    // raid -> alt detection skipped
    assert!(!d.is_alt_account);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn analyze_new_member_no_bans_still_runs_alt_path() {
    let (svc, p) = build().await;
    let g = fresh_id();
    set_config(&p, &g, "alt_detection_enabled", "true").await;
    set_config(&p, &g, "min_account_age_secs", "0").await;
    // Pas de bans → load_recent_ban_usernames retourne [] → pas de match possible
    let cmd = AnalyzeNewMemberCommand {
        guild_id: g.clone().into(),
        user_id: fresh_id().into(),
        username: "bob".into(),
        has_avatar: true,
        account_created_timestamp: chrono::Utc::now().timestamp() - 86400,
        is_bot: false,
        recent_joins: vec![],
        is_velocity_raid: false,
    };
    let d = svc.analyze_new_member(cmd).await.unwrap();
    assert!(!d.is_alt_account);
}
