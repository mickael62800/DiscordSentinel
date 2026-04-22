//! Tests de ManageSecurityService.
//!
//! Focus:
//! - report_event: persist + audit log + watcher + cache invalidation
//! - report_event: err sans audit_logs_uc injecté
//! - analyze_new_member: is_bot → decision default (no-op)
//! - analyze_new_member: < 3 joins + account old → decision default
//! - list_events: delegate vers repo (cache miss)
//!
//! Les chemins complexes (raid detection, alt detection) delegueraient
//! au domain::security_analyzer qui est deja teste en isolation.

use super::*;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::application::ManageSecurityService;
use crate::domain::entities::{
    AuditLog, BotDefinition, BotGuildConfig, ModerationAction, Rule, SecurityEvent, WatchedUser,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_audit_logs::{
    AuditLogFilters, CreateAuditLogCommand, ManageAuditLogsUseCase,
};
use crate::ports::inbound::{
    AnalyzeNewMemberCommand, ManageSecurityUseCase, ReportSecurityEventCommand,
};
#[allow(unused_imports)]
use crate::domain::services::security_analyzer::JoinInfo;
use crate::ports::outbound::{
    BotConfigRepository, CachePort, ModerationRepository, SecurityEventRepository,
    WatchedUserRepository,
};

// ── Mocks ──

#[derive(Default)]
struct MockSecurityRepo {
    saved: Mutex<Vec<SecurityEvent>>,
    all_returns: Mutex<Vec<SecurityEvent>>,
    by_guild_returns: Mutex<Vec<SecurityEvent>>,
}

#[async_trait]
impl SecurityEventRepository for MockSecurityRepo {
    async fn save(&self, event: &SecurityEvent) -> Result<(), DomainError> {
        self.saved.lock().unwrap().push(event.clone());
        Ok(())
    }
    async fn find_all(&self) -> Result<Vec<SecurityEvent>, DomainError> {
        Ok(self.all_returns.lock().unwrap().clone())
    }
    async fn find_by_guild(&self, _: &str) -> Result<Vec<SecurityEvent>, DomainError> {
        Ok(self.by_guild_returns.lock().unwrap().clone())
    }
}

#[derive(Default)]
struct MockCache {
    invalidate_calls: Mutex<Vec<String>>,
}

#[async_trait]
impl CachePort for MockCache {
    async fn get_rules(&self, _: &str) -> Result<Option<Vec<Rule>>, DomainError> { Ok(None) }
    async fn set_rules(&self, _: &str, _: &[Rule]) -> Result<(), DomainError> { Ok(()) }
    async fn invalidate_rules(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn get_json(&self, _: &str) -> Result<Option<String>, DomainError> { Ok(None) }
    async fn set_json(&self, _: &str, _: &str, _: u64) -> Result<(), DomainError> { Ok(()) }
    async fn invalidate(&self, key: &str) -> Result<(), DomainError> {
        self.invalidate_calls.lock().unwrap().push(key.into());
        Ok(())
    }
    async fn invalidate_pattern(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
}

#[derive(Default)]
struct MockWatchedRepo {
    watch_calls: Mutex<Vec<(String, String, String, String, String)>>,
}

#[async_trait]
impl WatchedUserRepository for MockWatchedRepo {
    async fn find_watched_users(&self, _: Option<&str>, _: i64, _: i64) -> Result<Vec<WatchedUser>, DomainError> { Ok(vec![]) }
    async fn add_manual_watch(
        &self, g: &str, u: &str, uname: &str, reason: &str, source: &str,
    ) -> Result<(), DomainError> {
        self.watch_calls.lock().unwrap().push((g.into(), u.into(), uname.into(), reason.into(), source.into()));
        Ok(())
    }
    async fn remove_manual_watch(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
}

#[derive(Default)]
struct MockAuditLogsUc {
    create_calls: Mutex<Vec<CreateAuditLogCommand>>,
}

#[async_trait]
impl ManageAuditLogsUseCase for MockAuditLogsUc {
    async fn create(&self, cmd: CreateAuditLogCommand) -> Result<AuditLog, DomainError> {
        let event_type = cmd.event_type.clone();
        let guild_id = cmd.guild_id.clone();
        self.create_calls.lock().unwrap().push(cmd);
        Ok(AuditLog {
            id: Uuid::new_v4(),
            guild_id,
            event_type,
            actor_id: None, actor_name: None,
            target_id: None, target_name: None,
            channel_id: None, channel_name: None,
            details: serde_json::json!({}),
            created_at: Utc::now(),
        })
    }
    async fn list(&self, _: Option<&str>, _: AuditLogFilters) -> Result<Vec<AuditLog>, DomainError> { Ok(vec![]) }
    async fn delete_older_than_days(&self, _: &str, _: i32) -> Result<u64, DomainError> { Ok(0) }
}

#[derive(Default)]
struct MockBotConfig {
    rows: Mutex<Vec<BotGuildConfig>>,
}

#[async_trait]
impl BotConfigRepository for MockBotConfig {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> { Ok(vec![]) }
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(self.rows.lock().unwrap().clone())
    }
    async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> { Ok(vec![]) }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
}

#[derive(Default)]
struct MockModerationRepo {
    bans: Mutex<Vec<ModerationAction>>,
}

#[async_trait]
impl ModerationRepository for MockModerationRepo {
    async fn save(&self, _: &ModerationAction) -> Result<(), DomainError> { Ok(()) }
    async fn find_by_id(&self, _: Uuid) -> Result<Option<ModerationAction>, DomainError> { Ok(None) }
    async fn find_by_target(&self, _: &str, _: &str, _: i64) -> Result<Vec<ModerationAction>, DomainError> { Ok(vec![]) }
    async fn find_bans(&self, _: Option<&str>, _: i64, _: i64) -> Result<Vec<ModerationAction>, DomainError> {
        Ok(self.bans.lock().unwrap().clone())
    }
    async fn find_all_for_guild(&self, _: Option<&str>, _: i64) -> Result<Vec<ModerationAction>, DomainError> { Ok(vec![]) }
    async fn delete_bans_for_user(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn delete_action(&self, _: Uuid) -> Result<bool, DomainError> { Ok(true) }
}

// ── Builder ──

fn build_service(with_audit: bool) -> (
    ManageSecurityService,
    Arc<MockSecurityRepo>,
    Arc<MockCache>,
    Arc<MockWatchedRepo>,
    Arc<MockAuditLogsUc>,
) {
    let repo = Arc::new(MockSecurityRepo::default());
    let cache = Arc::new(MockCache::default());
    let watched = Arc::new(MockWatchedRepo::default());
    let audit = Arc::new(MockAuditLogsUc::default());
    let bot_config = Arc::new(MockBotConfig::default());
    let moderation = Arc::new(MockModerationRepo::default());

    let mut svc = ManageSecurityService::new(
        repo.clone(), cache.clone(), watched.clone(), bot_config, moderation,
    );
    if with_audit {
        svc = svc.with_audit_logs_uc(audit.clone());
    }
    (svc, repo, cache, watched, audit)
}

// ═══════════════════════════════════════════════════════════════════
// report_event
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn report_event_errors_without_audit_logs_uc() {
    let (svc, _, _, _, _) = build_service(false);
    let err = svc.report_event(ReportSecurityEventCommand {
        guild_id: "g".into(),
        event_type: "raid".into(),
        severity: "high".into(),
        description: "x".into(),
        user_ids: vec!["u1".into()],
    }).await.unwrap_err();
    assert!(matches!(err, DomainError::Internal(_)));
    assert!(format!("{err:?}").contains("audit_logs_uc"));
}

#[tokio::test]
async fn report_event_persists_event_and_creates_audit_log() {
    let (svc, repo, _, _, audit) = build_service(true);
    let event = svc.report_event(ReportSecurityEventCommand {
        guild_id: "g1".into(),
        event_type: "raid".into(),
        severity: "critical".into(),
        description: "mass join".into(),
        user_ids: vec!["u1".into(), "u2".into()],
    }).await.unwrap();

    assert_eq!(event.event_type, "raid");
    assert_eq!(event.severity, "critical");

    // Event persisted
    assert_eq!(repo.saved.lock().unwrap().len(), 1);

    // Audit log prefixed with "security_"
    let audits = audit.create_calls.lock().unwrap();
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].event_type, "security_raid");
    assert_eq!(audits[0].guild_id, "g1");
    // Multi-user : target_id/name None (batch)
    assert!(audits[0].target_id.is_none());
}

#[tokio::test]
async fn report_event_single_user_sets_target_fields() {
    let (svc, _, _, _, audit) = build_service(true);
    svc.report_event(ReportSecurityEventCommand {
        guild_id: "g".into(),
        event_type: "alt".into(),
        severity: "medium".into(),
        description: "".into(),
        user_ids: vec!["u1".into()],
    }).await.unwrap();

    let audits = audit.create_calls.lock().unwrap();
    assert_eq!(audits[0].target_id.as_deref(), Some("u1"));
    assert_eq!(audits[0].target_name.as_deref(), Some("u1"));
}

#[tokio::test]
async fn report_event_adds_all_users_to_watch_list() {
    let (svc, _, _, watched, _) = build_service(true);
    svc.report_event(ReportSecurityEventCommand {
        guild_id: "g1".into(),
        event_type: "raid".into(),
        severity: "high".into(),
        description: "x".into(),
        user_ids: vec!["u1".into(), "u2".into(), "u3".into()],
    }).await.unwrap();

    let calls = watched.watch_calls.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].0, "g1"); // guild_id
    assert_eq!(calls[0].1, "u1"); // user_id
    assert_eq!(calls[0].4, "security_event"); // source
    assert!(calls[0].3.contains("raid")); // reason
    assert!(calls[0].3.contains("high"));
}

#[tokio::test]
async fn report_event_invalidates_both_cache_keys() {
    let (svc, _, cache, _, _) = build_service(true);
    svc.report_event(ReportSecurityEventCommand {
        guild_id: "guild-42".into(),
        event_type: "scan".into(),
        severity: "low".into(),
        description: "".into(),
        user_ids: vec![],
    }).await.unwrap();

    let invs = cache.invalidate_calls.lock().unwrap();
    assert!(invs.iter().any(|k| k == "security:all"));
    assert!(invs.iter().any(|k| k == "security:guild-42"));
}

// ═══════════════════════════════════════════════════════════════════
// analyze_new_member — early returns
// ═══════════════════════════════════════════════════════════════════

fn mk_member_cmd(is_bot: bool) -> AnalyzeNewMemberCommand {
    AnalyzeNewMemberCommand {
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "alice".into(),
        has_avatar: true,
        account_created_timestamp: Utc::now().timestamp() - 86400 * 365, // 1 an
        is_bot,
        recent_joins: vec![],
    }
}

#[tokio::test]
async fn analyze_new_member_returns_default_for_bots() {
    let (svc, _, _, watched, audit) = build_service(true);
    let decision = svc.analyze_new_member(mk_member_cmd(true)).await.unwrap();
    assert!(!decision.is_raid);
    assert!(!decision.is_suspicious_account);
    assert!(!decision.is_alt_account);
    assert!(decision.event_type.is_empty());
    // No side-effects
    assert!(watched.watch_calls.lock().unwrap().is_empty());
    assert!(audit.create_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn analyze_new_member_clean_account_returns_empty_decision() {
    let (svc, _, _, _, _) = build_service(true);
    // Compte ancien, pas de raid pattern (joins vides), config security-bot defaut (tout desactive).
    let decision = svc.analyze_new_member(mk_member_cmd(false)).await.unwrap();
    assert!(!decision.is_raid);
    assert!(decision.event_type.is_empty());
}

// ═══════════════════════════════════════════════════════════════════
// list_events
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn list_events_with_guild_calls_find_by_guild() {
    let (svc, repo, _, _, _) = build_service(false);
    repo.by_guild_returns.lock().unwrap().push(SecurityEvent {
        id: Uuid::new_v4(),
        guild_id: "g1".into(),
        event_type: "raid".into(),
        severity: "high".into(),
        description: "x".into(),
        user_ids: vec!["u1".into()],
        created_at: Utc::now(),
    });
    let events = svc.list_events(Some("g1")).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].guild_id, "g1");
}

#[tokio::test]
async fn list_events_without_guild_calls_find_all() {
    let (svc, repo, _, _, _) = build_service(false);
    repo.all_returns.lock().unwrap().push(SecurityEvent {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        event_type: "scan".into(),
        severity: "info".into(),
        description: "".into(),
        user_ids: vec![],
        created_at: Utc::now(),
    });
    let events = svc.list_events(None).await.unwrap();
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn list_events_empty_on_fresh_repo() {
    let (svc, _, _, _, _) = build_service(false);
    let events = svc.list_events(Some("nonexistent")).await.unwrap();
    assert!(events.is_empty());
}
