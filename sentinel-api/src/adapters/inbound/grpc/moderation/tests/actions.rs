use super::*;

use chrono::TimeZone;
use sentinel_core::domain::enums::moderation::moderation_gravity::ModerationGravity;
use uuid::Uuid;

fn ts() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
}

fn sample_action() -> ModerationAction {
    ModerationAction {
        id: Uuid::nil(),
        guild_id: "g".into(),
        channel_id: "c".into(),
        moderator_id: "mod".into(),
        moderator_name: "Mod".into(),
        target_id: "u".into(),
        target_name: "Joe".into(),
        target_display_name: None,
        action_type: "warn".into(),
        reason: "spam".into(),
        gravity: Some(ModerationGravity::High),
        duration: Some(3600),
        created_at: ts(),
    }
}

#[test]
fn moderation_action_to_proto_full_mapping() {
    let p = moderation_action_to_proto(sample_action());
    assert_eq!(p.guild_id, "g");
    assert_eq!(p.moderator_name, "Mod");
    assert_eq!(p.action_type, "warn");
    assert_eq!(p.reason, "spam");
    assert_eq!(p.gravity.as_deref(), Some("high"));
    assert_eq!(p.duration, Some(3600));
    assert_eq!(p.created_at, ts().to_rfc3339());
}

#[test]
fn moderation_action_to_proto_no_gravity_no_duration() {
    let mut a = sample_action();
    a.gravity = None;
    a.duration = None;
    let p = moderation_action_to_proto(a);
    assert!(p.gravity.is_none());
    assert!(p.duration.is_none());
}

#[test]
fn moderation_action_gravity_low_serialised() {
    let mut a = sample_action();
    a.gravity = Some(ModerationGravity::Low);
    let p = moderation_action_to_proto(a);
    assert_eq!(p.gravity.as_deref(), Some("low"));
}

#[test]
fn user_history_to_proto_full_mapping() {
    let h = UserModerationHistory {
        target_id: "u".into(),
        target_name: "Joe".into(),
        total_warns: 3,
        total_mutes: 1,
        total_bans: 0,
        actions: vec![sample_action(), sample_action()],
    };
    let p = user_history_to_proto(h);
    assert_eq!(p.target_id, "u");
    assert_eq!(p.total_warns, 3);
    assert_eq!(p.total_mutes, 1);
    assert_eq!(p.total_bans, 0);
    assert_eq!(p.actions.len(), 2);
}

#[test]
fn user_history_to_proto_empty_history() {
    let h = UserModerationHistory {
        target_id: "u".into(),
        target_name: "Clean".into(),
        total_warns: 0,
        total_mutes: 0,
        total_bans: 0,
        actions: vec![],
    };
    let p = user_history_to_proto(h);
    assert!(p.actions.is_empty());
}

// ── RPC tests avec mock ──

use crate::ports::inbound::moderation::manage_moderation::LogModerationCommand;
use crate::ports::inbound::moderation::manage_moderation::LoggedModerationAction;
use crate::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use async_trait::async_trait;
use chrono::Utc;
use sentinel_core::domain::entities::moderation::action::strikes::StrikeResult;
use sentinel_core::domain::entities::moderation::action::strikes::UserStrike;
use sentinel_core::domain::errors::DomainError;
use std::sync::Arc;
use std::sync::Mutex;
#[derive(Default)]
struct MockModerationUc {
    log_calls: Mutex<Vec<LogModerationCommand>>,
    history_return: Mutex<Option<UserModerationHistory>>,
    strike_result: Mutex<Option<StrikeResult>>,
}

fn sample_strike() -> StrikeResult {
    StrikeResult {
        strike: UserStrike {
            id: Uuid::new_v4(),
            guild_id: "g".into(),
            user_id: "u".into(),
            reason: "warn".into(),
            source: "moderation".into(),
            infraction_id: None,
            expires_at: None,
            created_at: Utc::now(),
        },
        active_count: 3,
        escalation_action: Some("mute".into()),
        escalation_duration: Some(1800),
    }
}

#[async_trait]
impl ManageModerationUseCase for MockModerationUc {
    async fn log_action(&self, cmd: LogModerationCommand) -> Result<ModerationAction, DomainError> {
        let action = ModerationAction {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            channel_id: cmd.channel_id.clone(),
            moderator_id: cmd.moderator_id.clone(),
            moderator_name: cmd.moderator_name.clone(),
            target_id: cmd.target_id.clone(),
            target_name: cmd.target_name.clone(),
            target_display_name: None,
            action_type: cmd.action_type.clone(),
            reason: cmd.reason.clone(),
            gravity: None,
            duration: cmd.duration,
            created_at: ts(),
        };
        self.log_calls.lock().unwrap().push(cmd);
        Ok(action)
    }
    async fn log_action_with_strike(
        &self,
        cmd: LogModerationCommand,
    ) -> Result<LoggedModerationAction, DomainError> {
        let action = self.log_action(cmd).await?;
        Ok(LoggedModerationAction {
            action,
            strike: self.strike_result.lock().unwrap().clone(),
        })
    }
    async fn get_history(
        &self,
        _: &str,
        target_id: &str,
    ) -> Result<UserModerationHistory, DomainError> {
        Ok(self
            .history_return
            .lock()
            .unwrap()
            .clone()
            .unwrap_or(UserModerationHistory {
                target_id: target_id.into(),
                target_name: "unknown".into(),
                total_warns: 0,
                total_mutes: 0,
                total_bans: 0,
                actions: vec![],
            }))
    }
    async fn list_bans(
        &self,
        _: Option<&str>,
        _: i64,
        _: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        Ok(vec![])
    }
    async fn list_actions(
        &self,
        _: Option<&str>,
        _: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        Ok(vec![])
    }
    async fn delete_bans_for_user(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete_action(&self, _: Uuid) -> Result<bool, DomainError> {
        Ok(true)
    }
}

/// Mock no-op du use case rappels : create_reminder renvoie un reminder factice.
#[derive(Default)]
struct MockRemindersUc;

#[async_trait]
impl ManageRemindersUseCase for MockRemindersUc {
    async fn create_reminder(
        &self,
        cmd: CreateReminderCommand,
    ) -> Result<
        sentinel_core::domain::entities::moderation::action::sanction_reminder::SanctionReminder,
        DomainError,
    > {
        Ok(
            sentinel_core::domain::entities::moderation::action::sanction_reminder::SanctionReminder {
                id: Uuid::new_v4(),
                guild_id: cmd.guild_id,
                moderator_id: cmd.moderator_id,
                moderator_name: cmd.moderator_name,
                target_id: cmd.target_id,
                target_name: cmd.target_name,
                action_type: cmd.action_type,
                reason: cmd.reason,
                action_id: cmd.action_id,
                remind_at: ts(),
                expires_at: ts(),
                status: "pending".into(),
                created_at: ts(),
            },
        )
    }
    async fn get_pending_reminders(
        &self,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::action::sanction_reminder::SanctionReminder>,
        DomainError,
    >{
        Ok(vec![])
    }
    async fn mark_sent(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn cancel_for_action(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_by_guild(
        &self,
        _: &str,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::action::sanction_reminder::SanctionReminder>,
        DomainError,
    >{
        Ok(vec![])
    }
}

/// Mock du use case copilote : renvoie un contexte fixe deterministe.
#[derive(Default)]
struct MockCopilotUc {
    calls: Mutex<Vec<(String, String, i64, u32)>>,
}

use sentinel_core::domain::entities::moderation::copilot::MemberModerationContext;
use sentinel_core::domain::entities::moderation::copilot::PrecedentDistribution;
use sentinel_core::domain::entities::moderation::copilot::SanctionSuggestion;
use sentinel_core::domain::entities::moderation::copilot::SuggestionBasis;
use sentinel_core::domain::entities::moderation::review::automod::AppliedAction;

#[async_trait]
impl ModerationCopilotUseCase for MockCopilotUc {
    async fn get_member_context(
        &self,
        guild_id: &str,
        user_id: &str,
        lookback_days: i64,
        min_precedents: u32,
    ) -> Result<MemberModerationContext, DomainError> {
        self.calls.lock().unwrap().push((
            guild_id.into(),
            user_id.into(),
            lookback_days,
            min_precedents,
        ));
        Ok(MemberModerationContext {
            active_strikes: 2,
            sanctions_by_type: vec![("warn".into(), 3), ("mute".into(), 1)],
            last_sanction_at: Some(ts()),
            open_reviews: 1,
            precedents: PrecedentDistribution {
                flag_category: "phishing".into(),
                counts_by_action: vec![("ban".into(), 4), ("mute".into(), 1)],
                total: 5,
            },
            suggestion: SanctionSuggestion {
                action: Some(AppliedAction::Ban),
                basis: SuggestionBasis::Both,
                rationale: "jurisprudence concordante".into(),
                precedent_count: 5,
            },
        })
    }
}

fn grpc(uc: Arc<MockModerationUc>) -> ModerationGrpc {
    ModerationGrpc {
        moderation_uc: uc,
        reminders_uc: Arc::new(MockRemindersUc),
        moderation_copilot_uc: Arc::new(MockCopilotUc::default()),
    }
}

fn make_log_request(action: &str) -> Request<proto::LogActionRequest> {
    Request::new(proto::LogActionRequest {
        guild_id: "g".into(),
        channel_id: "c".into(),
        moderator_id: "mod".into(),
        moderator_name: "Mod".into(),
        target_id: "t".into(),
        target_name: "Target".into(),
        action_type: action.into(),
        reason: "r".into(),
        gravity: None,
        duration: None,
        skip_strike: false,
    })
}

#[tokio::test]
async fn log_action_delegates_to_uc() {
    let uc = Arc::new(MockModerationUc::default());
    let g = grpc(uc.clone());
    let _ = g.log_action(make_log_request("warn")).await.unwrap();
    let calls = uc.log_calls.lock().unwrap();
    assert_eq!(calls[0].action_type, "warn");
    assert_eq!(calls[0].moderator_name, "Mod");
}

#[tokio::test]
async fn log_action_without_strike_has_none_escalation() {
    let uc = Arc::new(MockModerationUc::default());
    let g = grpc(uc);
    let resp = g.log_action(make_log_request("warn")).await.unwrap();
    let inner = resp.into_inner();
    assert!(inner.strikes_count.is_none());
    assert!(inner.escalation_action.is_none());
    assert!(inner.escalation_duration.is_none());
}

#[tokio::test]
async fn log_action_with_strike_populates_escalation() {
    let uc = Arc::new(MockModerationUc::default());
    *uc.strike_result.lock().unwrap() = Some(sample_strike());
    let g = grpc(uc);
    let resp = g.log_action(make_log_request("warn")).await.unwrap();
    let inner = resp.into_inner();
    assert_eq!(inner.strikes_count, Some(3));
    assert_eq!(inner.escalation_action.as_deref(), Some("mute"));
    assert_eq!(inner.escalation_duration, Some(1800));
}

#[tokio::test]
async fn get_history_returns_full_user_data() {
    let uc = Arc::new(MockModerationUc::default());
    *uc.history_return.lock().unwrap() = Some(UserModerationHistory {
        target_id: "u".into(),
        target_name: "Alice".into(),
        total_warns: 5,
        total_mutes: 2,
        total_bans: 1,
        actions: vec![sample_action()],
    });
    let g = grpc(uc);
    let resp = g
        .get_history(Request::new(proto::GetHistoryRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
        }))
        .await
        .unwrap();
    let h = resp.into_inner();
    assert_eq!(h.target_name, "Alice");
    assert_eq!(h.total_warns, 5);
    assert_eq!(h.actions.len(), 1);
}

#[tokio::test]
async fn get_member_context_maps_domain_to_proto() {
    let copilot = Arc::new(MockCopilotUc::default());
    let g = ModerationGrpc {
        moderation_uc: Arc::new(MockModerationUc::default()),
        reminders_uc: Arc::new(MockRemindersUc),
        moderation_copilot_uc: copilot.clone(),
    };
    let resp = g
        .get_member_context(Request::new(proto::GetMemberContextRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
            lookback_days: 90,
            min_precedents: 3,
        }))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(
        copilot.calls.lock().unwrap()[0],
        ("g".into(), "u".into(), 90, 3)
    );
    assert_eq!(resp.active_strikes, 2);
    assert_eq!(resp.sanctions_by_type.len(), 2);
    assert_eq!(resp.sanctions_by_type[0].action, "warn");
    assert_eq!(resp.sanctions_by_type[0].count, 3);
    assert_eq!(resp.open_reviews, 1);
    assert!(resp.last_sanction_at.is_some());
    let prec = resp.precedents.unwrap();
    assert_eq!(prec.flag_category, "phishing");
    assert_eq!(prec.total, 5);
    let sugg = resp.suggestion.unwrap();
    assert_eq!(sugg.action.as_deref(), Some("ban"));
    assert_eq!(sugg.basis, "both");
    assert_eq!(sugg.precedent_count, 5);
}

#[tokio::test]
async fn get_history_clean_user_has_zero_counters() {
    let uc = Arc::new(MockModerationUc::default());
    let g = grpc(uc);
    let resp = g
        .get_history(Request::new(proto::GetHistoryRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
        }))
        .await
        .unwrap();
    let h = resp.into_inner();
    assert_eq!(h.total_warns, 0);
    assert_eq!(h.total_mutes, 0);
    assert_eq!(h.total_bans, 0);
    assert!(h.actions.is_empty());
}
