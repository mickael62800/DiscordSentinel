use super::*;
use sentinel_core::domain::entities::community::level::LevelReward;
use sentinel_core::domain::entities::community::level::UserLevel;
use sentinel_core::domain::entities::community::level::XpSource;
use crate::ports::inbound::community::manage_levels::AddXpCommand;
use crate::ports::inbound::community::manage_levels::ManageLevelsUseCase;
use crate::ports::inbound::community::manage_levels::SaveLevelConfigCommand;
use std::sync::Mutex as StdMutex;
use chrono::Utc as ChronoUtc;

struct MockRepo {
    user_level: StdMutex<Option<UserLevel>>,
}
impl MockRepo {
    fn new() -> Self {
        Self { user_level: StdMutex::new(None) }
    }
}

#[async_trait]
impl LevelRepository for MockRepo {
    async fn get_config(&self, _g: &str) -> Result<Option<LevelConfig>, DomainError> { Ok(None) }
    async fn upsert_config(&self, _c: &LevelConfig) -> Result<(), DomainError> { Ok(()) }
    async fn add_xp_atomic(&self, guild_id: &str, user_id: &str, username: &str, amount: i64, source: XpSource) -> Result<UserLevel, DomainError> {
        let now = ChronoUtc::now();
        let (xp_text, xp_voice) = match source {
            XpSource::Text => (amount, 0),
            XpSource::Voice => (0, amount),
            XpSource::Days => (0, 0),
        };
        Ok(UserLevel {
            id: uuid::Uuid::new_v4(),
            guild_id: guild_id.into(),
            user_id: user_id.into(),
            username: username.into(),
            xp: amount,
            level: 0,
            xp_text,
            level_text: 0,
            xp_voice,
            level_voice: 0,
            last_xp_at: now,
            created_at: now,
            updated_at: now,
        })
    }
    async fn upsert_user_level(&self, user_level: &UserLevel) -> Result<(), DomainError> {
        *self.user_level.lock().unwrap() = Some(user_level.clone());
        Ok(())
    }
    async fn get_user_level(&self, _g: &str, _u: &str) -> Result<Option<UserLevel>, DomainError> {
        Ok(self.user_level.lock().unwrap().clone())
    }
    async fn get_leaderboard(&self, _: &str, _: i64) -> Result<Vec<UserLevel>, DomainError> { Ok(vec![]) }
    async fn get_leaderboard_by_source(&self, _: &str, _: XpSource, _: i64) -> Result<Vec<UserLevel>, DomainError> { Ok(vec![]) }
    async fn get_rewards(&self, _: &str) -> Result<Vec<LevelReward>, DomainError> { Ok(vec![]) }
    async fn get_rewards_by_source(&self, _: &str, _: XpSource) -> Result<Vec<LevelReward>, DomainError> { Ok(vec![]) }
    async fn upsert_reward(&self, _: &LevelReward) -> Result<(), DomainError> { Ok(()) }
    async fn delete_reward(&self, _: &str, _: i32, _: XpSource) -> Result<(), DomainError> { Ok(()) }
    async fn refresh_leaderboard_view(&self) -> Result<(), DomainError> { Ok(()) }
}

fn make_cmd(xp_per_msg: i32, xp_per_voice: i32, cooldown: i32) -> SaveLevelConfigCommand {
    SaveLevelConfigCommand {
        guild_id: "g".into(),
        xp_per_message: xp_per_msg,
        xp_per_voice_minute: xp_per_voice,
        xp_cooldown_secs: cooldown,
        level_up_channel_id: None,
        level_up_message: "up!".into(),
        excluded_channels: vec![],
        enabled: true,
    }
}

fn make_svc() -> ManageLevelsService {
    ManageLevelsService::new(std::sync::Arc::new(MockRepo::new()))
}

#[tokio::test]
async fn save_config_accepts_valid_values() {
    let svc = make_svc();
    let cfg = svc.save_config(make_cmd(10, 5, 60)).await.unwrap();
    assert_eq!(cfg.xp_per_message, 10);
    assert_eq!(cfg.xp_per_voice_minute, 5);
}

#[tokio::test]
async fn save_config_rejects_xp_per_message_out_of_range() {
    let svc = make_svc();
    assert!(svc.save_config(make_cmd(0, 5, 60)).await.is_err());
    assert!(svc.save_config(make_cmd(1001, 5, 60)).await.is_err());
    assert!(svc.save_config(make_cmd(-10, 5, 60)).await.is_err());
}

#[tokio::test]
async fn save_config_rejects_xp_per_voice_out_of_range() {
    let svc = make_svc();
    assert!(svc.save_config(make_cmd(10, 0, 60)).await.is_err());
    assert!(svc.save_config(make_cmd(10, 1001, 60)).await.is_err());
}

#[tokio::test]
async fn save_config_rejects_cooldown_out_of_range() {
    let svc = make_svc();
    assert!(svc.save_config(make_cmd(10, 5, -1)).await.is_err());
    assert!(svc.save_config(make_cmd(10, 5, 3601)).await.is_err());
}

#[tokio::test]
async fn save_config_accepts_boundary_values() {
    let svc = make_svc();
    assert!(svc.save_config(make_cmd(1, 1, 0)).await.is_ok());
    assert!(svc.save_config(make_cmd(1000, 1000, 3600)).await.is_ok());
}

#[tokio::test]
async fn add_xp_rejects_non_positive_amount() {
    let svc = make_svc();
    assert!(svc.add_xp(AddXpCommand {
        guild_id: "g".into(), user_id: "u".into(), username: "u".into(),
        amount: 0, source: XpSource::Text,
    }).await.is_err());
    assert!(svc.add_xp(AddXpCommand {
        guild_id: "g".into(), user_id: "u".into(), username: "u".into(),
        amount: -5, source: XpSource::Text,
    }).await.is_err());
}

#[tokio::test]
async fn add_xp_rejects_above_cap() {
    let svc = make_svc();
    assert!(svc.add_xp(AddXpCommand {
        guild_id: "g".into(), user_id: "u".into(), username: "u".into(),
        amount: 10001, source: XpSource::Text,
    }).await.is_err());
}

#[tokio::test]
async fn add_xp_accepts_within_range() {
    let svc = make_svc();
    let result = svc.add_xp(AddXpCommand {
        guild_id: "g".into(), user_id: "u".into(), username: "u".into(),
        amount: 100, source: XpSource::Text,
    }).await.unwrap();
    assert_eq!(result.user_level.xp, 100);
    assert_eq!(result.source, XpSource::Text);
}

#[tokio::test]
async fn add_xp_boundary_10000_accepted() {
    let svc = make_svc();
    assert!(svc.add_xp(AddXpCommand {
        guild_id: "g".into(), user_id: "u".into(), username: "u".into(),
        amount: 10000, source: XpSource::Voice,
    }).await.is_ok());
}

// ══════════════════════════════════════════════════════════
// get_config / get_user_level : NotFound
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn get_config_returns_defaults_when_repo_empty() {
    // Comportement : pas de 404 sur une guild jamais sauve, on retourne
    // les valeurs par defaut pour eviter de polluer la console frontend.
    let svc = make_svc();
    let cfg = svc.get_config("ghost").await.unwrap();
    assert_eq!(cfg.xp_per_message, 15);
    assert_eq!(cfg.xp_per_voice_minute, 5);
    assert_eq!(cfg.xp_cooldown_secs, 60);
    assert!(cfg.enabled);
    assert!(cfg.excluded_channels.is_empty());
    assert!(cfg.level_up_channel_id.is_none());
}

#[tokio::test]
async fn get_user_level_not_found_when_repo_empty() {
    let svc = make_svc();
    let err = svc.get_user_level("g", "ghost").await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test]
async fn get_user_level_found_after_add_xp() {
    let svc = make_svc();
    svc.add_xp(AddXpCommand {
        guild_id: "g".into(), user_id: "u1".into(), username: "u1".into(),
        amount: 50, source: XpSource::Text,
    }).await.unwrap();
    let ul = svc.get_user_level("g", "u1").await.unwrap();
    assert_eq!(ul.xp, 50);
    assert_eq!(ul.xp_text, 50);
}

// ══════════════════════════════════════════════════════════
// Delegates : leaderboard, rewards
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn get_leaderboard_passes_through_repo() {
    let svc = make_svc();
    let res = svc.get_leaderboard("g", 10).await.unwrap();
    assert!(res.is_empty());
}

#[tokio::test]
async fn get_leaderboard_by_source_voice() {
    let svc = make_svc();
    let res = svc.get_leaderboard_by_source("g", XpSource::Voice, 5).await.unwrap();
    assert!(res.is_empty());
}

#[tokio::test]
async fn get_rewards_passes_through() {
    let svc = make_svc();
    assert!(svc.get_rewards("g").await.unwrap().is_empty());
}

#[tokio::test]
async fn get_rewards_by_source_passes_through() {
    let svc = make_svc();
    assert!(svc.get_rewards_by_source("g", XpSource::Text).await.unwrap().is_empty());
}

// ══════════════════════════════════════════════════════════
// set_reward / delete_reward
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn set_reward_returns_constructed_reward() {
    let svc = make_svc();
    let r = svc.set_reward("g1", 10, "role-42", XpSource::Text).await.unwrap();
    assert_eq!(r.guild_id, "g1");
    assert_eq!(r.level, 10);
    assert_eq!(r.role_id, "role-42");
    assert_eq!(r.source, XpSource::Text);
    // UUID fresh généré
    assert_ne!(r.id, uuid::Uuid::nil());
}

#[tokio::test]
async fn delete_reward_passes_through() {
    let svc = make_svc();
    assert!(svc.delete_reward("g", 5, XpSource::Voice).await.is_ok());
}

// ══════════════════════════════════════════════════════════
// add_xp avec level-up : source mapping
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn add_xp_text_source_updates_only_xp_text() {
    let svc = make_svc();
    let res = svc.add_xp(AddXpCommand {
        guild_id: "g".into(), user_id: "u".into(), username: "u".into(),
        amount: 200, source: XpSource::Text,
    }).await.unwrap();
    assert_eq!(res.user_level.xp_text, 200);
    assert_eq!(res.user_level.xp_voice, 0);
    assert_eq!(res.source, XpSource::Text);
}

#[tokio::test]
async fn add_xp_voice_source_updates_only_xp_voice() {
    let svc = make_svc();
    let res = svc.add_xp(AddXpCommand {
        guild_id: "g".into(), user_id: "u".into(), username: "u".into(),
        amount: 300, source: XpSource::Voice,
    }).await.unwrap();
    assert_eq!(res.user_level.xp_text, 0);
    assert_eq!(res.user_level.xp_voice, 300);
    assert_eq!(res.source, XpSource::Voice);
}

#[tokio::test]
async fn add_xp_days_source_leaves_both_zero() {
    // Days n'a pas de colonne dédiée → xp_text et xp_voice restent 0.
    // Source None pour leveled_up (return (0, 0) case).
    let svc = make_svc();
    let res = svc.add_xp(AddXpCommand {
        guild_id: "g".into(), user_id: "u".into(), username: "u".into(),
        amount: 500, source: XpSource::Days,
    }).await.unwrap();
    assert_eq!(res.user_level.xp_text, 0);
    assert_eq!(res.user_level.xp_voice, 0);
    assert_eq!(res.user_level.xp, 500); // xp global mis a jour
    assert!(!res.leveled_up);
    assert_eq!(res.old_level, 0);
}
