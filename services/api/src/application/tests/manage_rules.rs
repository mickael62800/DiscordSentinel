use super::*;
use crate::domain::entities::Rule;
use crate::domain::value_objects::FlagType;
use crate::ports::inbound::{CreateRuleCommand, ManageRulesUseCase};
use crate::ports::outbound::CachePort;
use std::sync::Mutex as StdMutex;

#[derive(Default)]
struct MockRuleRepo {
    saved: StdMutex<Vec<Rule>>,
}
#[async_trait]
impl RuleRepository for MockRuleRepo {
    async fn find_by_guild(&self, _: &str) -> Result<Vec<Rule>, DomainError> { Ok(vec![]) }
    async fn find_all(&self) -> Result<Vec<Rule>, DomainError> { Ok(vec![]) }
    async fn find_by_id(&self, _: uuid::Uuid) -> Result<Option<Rule>, DomainError> { Ok(None) }
    async fn save(&self, rule: &Rule) -> Result<Rule, DomainError> {
        self.saved.lock().unwrap().push(rule.clone());
        Ok(rule.clone())
    }
    async fn toggle(&self, _: uuid::Uuid, _: bool) -> Result<(), DomainError> { Ok(()) }
    async fn delete(&self, _: uuid::Uuid) -> Result<(), DomainError> { Ok(()) }
}

struct NoOpCache;
#[async_trait]
impl CachePort for NoOpCache {
    async fn get_rules(&self, _: &str) -> Result<Option<Vec<Rule>>, DomainError> { Ok(None) }
    async fn set_rules(&self, _: &str, _: &[Rule]) -> Result<(), DomainError> { Ok(()) }
    async fn invalidate_rules(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn get_json(&self, _: &str) -> Result<Option<String>, DomainError> { Ok(None) }
    async fn set_json(&self, _: &str, _: &str, _: u64) -> Result<(), DomainError> { Ok(()) }
    async fn invalidate(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn invalidate_pattern(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
}

fn make_svc() -> ManageRulesService {
    ManageRulesService::new(Arc::new(MockRuleRepo::default()), Arc::new(NoOpCache))
}

fn valid_cmd() -> CreateRuleCommand {
    CreateRuleCommand {
        guild_id: "g".into(),
        flag_type: FlagType::Spam,
        weight: 3.0,
        threshold_warn: 2.0,
        threshold_delete: 4.0,
        threshold_mute: 6.0,
        threshold_ban: 9.0,
        enabled: true,
    }
}

#[tokio::test]
async fn create_accepts_valid_command() {
    let svc = make_svc();
    let rule = svc.create_or_update_rule(valid_cmd()).await.unwrap();
    assert_eq!(rule.weight, 3.0);
    assert_eq!(rule.threshold_ban, 9.0);
    assert!(rule.enabled);
}

#[tokio::test]
async fn create_rejects_negative_weight() {
    let svc = make_svc();
    let mut cmd = valid_cmd();
    cmd.weight = -0.1;
    let err = svc.create_or_update_rule(cmd).await.unwrap_err();
    assert!(matches!(err, DomainError::InvalidRule(_)));
}

#[tokio::test]
async fn create_accepts_zero_weight() {
    let svc = make_svc();
    let mut cmd = valid_cmd();
    cmd.weight = 0.0;
    assert!(svc.create_or_update_rule(cmd).await.is_ok());
}

#[tokio::test]
async fn create_rejects_warn_gte_delete() {
    let svc = make_svc();
    let mut cmd = valid_cmd();
    cmd.threshold_warn = 4.0;
    cmd.threshold_delete = 4.0;
    assert!(matches!(svc.create_or_update_rule(cmd).await, Err(DomainError::InvalidRule(_))));
}

#[tokio::test]
async fn create_rejects_delete_gte_mute() {
    let svc = make_svc();
    let mut cmd = valid_cmd();
    cmd.threshold_delete = 6.0;
    cmd.threshold_mute = 6.0;
    assert!(matches!(svc.create_or_update_rule(cmd).await, Err(DomainError::InvalidRule(_))));
}

#[tokio::test]
async fn create_rejects_mute_gte_ban() {
    let svc = make_svc();
    let mut cmd = valid_cmd();
    cmd.threshold_mute = 9.0;
    cmd.threshold_ban = 9.0;
    assert!(matches!(svc.create_or_update_rule(cmd).await, Err(DomainError::InvalidRule(_))));
}

#[tokio::test]
async fn create_rejects_inverted_thresholds() {
    let svc = make_svc();
    let mut cmd = valid_cmd();
    cmd.threshold_warn = 10.0;
    cmd.threshold_delete = 8.0;
    cmd.threshold_mute = 6.0;
    cmd.threshold_ban = 4.0;
    assert!(matches!(svc.create_or_update_rule(cmd).await, Err(DomainError::InvalidRule(_))));
}
