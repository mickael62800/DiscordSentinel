//! Tests d'integration HTTP pour les endpoints dashboard qui agregent
//! plusieurs use-cases (get_dashboard_stats, get_all_infractions,
//! get_all_rules, toggle_rule).

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::adapters::inbound::http::state::AppState;
use sentinel_api::domain::entities::audit::dashboard_stats::DashboardStats;
use sentinel_api::domain::entities::audit::user_stats::GuildStatsOverview;
use sentinel_api::domain::entities::audit::user_stats::GuildVoiceStats;
use sentinel_api::domain::entities::moderation::infraction::Infraction;
use sentinel_api::domain::entities::moderation::action::action::ModerationAction;
use sentinel_api::domain::entities::system::rule::Rule;
use sentinel_api::domain::entities::moderation::action::action::UserModerationHistory;
use sentinel_api::domain::entities::audit::user_stats::UserStats;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::domain::enums::moderation::action::Action;
use sentinel_api::domain::entities::moderation::detection_flags::DetectionFlags;
use sentinel_api::domain::enums::moderation::flag_type::FlagType;
use sentinel_api::domain::enums::moderation::moderation_gravity::ModerationGravity;
use sentinel_api::ports::inbound::moderation::manage_rules::CreateRuleCommand;
use sentinel_api::ports::inbound::moderation::manage_infractions::InfractionFilters;
use sentinel_api::ports::inbound::moderation::manage_moderation::LogModerationCommand;
use sentinel_api::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use sentinel_api::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use sentinel_api::ports::inbound::moderation::manage_rules::ManageRulesUseCase;
use sentinel_api::ports::inbound::audit::manage_stats::ManageStatsUseCase;
use sentinel_api::ports::inbound::audit::manage_stats::RecordMessagesCommand;
use sentinel_api::ports::inbound::audit::manage_stats::RecordVoiceCommand;
use test_helpers::build_test_state_stats;

// ══════════════════════════════════════════════════════════
// Mocks
// ══════════════════════════════════════════════════════════

struct MockStatsUC {
    dashboard: DashboardStats,
}

#[async_trait]
impl ManageStatsUseCase for MockStatsUC {
    async fn record_messages(&self, _: RecordMessagesCommand) -> Result<(), DomainError> { Ok(()) }
    async fn record_voice(&self, _: RecordVoiceCommand) -> Result<(), DomainError> { Ok(()) }
    async fn get_user_stats(&self, _: &str, _: &str) -> Result<Option<UserStats>, DomainError> { Ok(None) }
    async fn get_guild_overview(&self, _: &str) -> Result<GuildStatsOverview, DomainError> { unimplemented!() }
    async fn get_leaderboard(&self, _: &str, _: u32) -> Result<Vec<UserStats>, DomainError> { Ok(vec![]) }
    async fn get_dashboard_stats(&self) -> Result<DashboardStats, DomainError> {
        Ok(self.dashboard.clone())
    }
    async fn get_guild_voice_stats(&self, _: &str, _: u32, _: u32) -> Result<GuildVoiceStats, DomainError> {
        unimplemented!()
    }
}

#[derive(Default)]
struct MockInfractionsUC {
    items: Mutex<Vec<Infraction>>,
}

#[async_trait]
impl ManageInfractionsUseCase for MockInfractionsUC {
    async fn list_infractions(&self, guild_id: &str, _: InfractionFilters) -> Result<Vec<Infraction>, DomainError> {
        Ok(self.items.lock().unwrap().iter().filter(|i| i.guild_id == guild_id).cloned().collect())
    }
    async fn list_all_infractions(&self, _: i64, _: i64) -> Result<Vec<Infraction>, DomainError> {
        Ok(self.items.lock().unwrap().clone())
    }
    async fn count_today(&self) -> Result<u64, DomainError> { Ok(0) }
    async fn find_by_id(&self, _: &str) -> Result<Option<Infraction>, DomainError> { Ok(None) }
    async fn delete_infraction(&self, _: &str) -> Result<bool, DomainError> { Ok(true) }
    async fn delete_older_than_days(&self, _: &str, _: i32) -> Result<u64, DomainError> { Ok(0) }
}

#[derive(Default)]
struct MockModerationUC {
    actions: Mutex<Vec<ModerationAction>>,
}

#[async_trait]
impl ManageModerationUseCase for MockModerationUC {
    async fn list_actions(&self, guild_id: Option<&str>, _: i64) -> Result<Vec<ModerationAction>, DomainError> {
        Ok(self.actions.lock().unwrap().iter()
            .filter(|a| guild_id.is_none_or(|g| a.guild_id == g))
            .cloned().collect())
    }
    async fn log_action(&self, _: LogModerationCommand) -> Result<ModerationAction, DomainError> {
        unimplemented!()
    }
    async fn get_history(&self, _: &str, _: &str) -> Result<UserModerationHistory, DomainError> {
        unimplemented!()
    }
    async fn list_bans(&self, _: Option<&str>, _: i64, _: i64) -> Result<Vec<ModerationAction>, DomainError> {
        Ok(vec![])
    }
    async fn delete_bans_for_user(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn delete_action(&self, _: Uuid) -> Result<bool, DomainError> { Ok(true) }
}

#[derive(Default)]
struct MockRulesUC {
    rules: Mutex<Vec<Rule>>,
    toggled: Mutex<Vec<(Uuid, bool)>>,
}

#[async_trait]
impl ManageRulesUseCase for MockRulesUC {
    async fn get_rules(&self, guild_id: &str) -> Result<Vec<Rule>, DomainError> {
        Ok(self.rules.lock().unwrap().iter().filter(|r| r.guild_id == guild_id).cloned().collect())
    }
    async fn get_all_rules(&self) -> Result<Vec<Rule>, DomainError> {
        Ok(self.rules.lock().unwrap().clone())
    }
    async fn toggle_rule(&self, id: Uuid, enabled: bool) -> Result<bool, DomainError> {
        self.toggled.lock().unwrap().push((id, enabled));
        Ok(enabled)
    }
    async fn create_or_update_rule(&self, _: CreateRuleCommand) -> Result<Rule, DomainError> {
        unimplemented!()
    }
    async fn delete_rule(&self, _: &str, _: Uuid) -> Result<(), DomainError> { Ok(()) }
}

// ══════════════════════════════════════════════════════════
// State builder agregeant les 4 mocks
// ══════════════════════════════════════════════════════════

struct TestMocks {
    stats: Arc<MockStatsUC>,
    infractions: Arc<MockInfractionsUC>,
    moderation: Arc<MockModerationUC>,
    rules: Arc<MockRulesUC>,
}

fn build_state(mocks: &TestMocks) -> AppState {
    let mut state = build_test_state_stats(mocks.stats.clone());
    state.infractions_uc = mocks.infractions.clone();
    state.moderation_uc = mocks.moderation.clone();
    state.rules_uc = mocks.rules.clone();
    state
}

fn sample_dashboard() -> DashboardStats {
    DashboardStats {
        total_servers: 5,
        total_users: 100,
        messages_today: 1234,
        infractions_today: 7,
        bots_online: 3, bots_total: 4,
        workers_online: 2, workers_total: 2,
        postgres_online: true, redis_online: true,
    }
}

fn sample_infraction(guild_id: &str, action: Action) -> Infraction {
    Infraction {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(), channel_id: "c".into(),
        user_id: "u".into(), username: "alice".into(),
        message_id: "m".into(), content: "x".into(),
        flags: DetectionFlags { spam: false, insult: false, link: false, phishing: false },
        score: 0.5, action, reason: "r".into(),
        duration: None, created_at: Utc::now(),
    }
}

fn sample_action(guild_id: &str, action_type: &str) -> ModerationAction {
    ModerationAction {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(), channel_id: "c".into(),
        moderator_id: "m".into(), moderator_name: "Mod".into(),
        target_id: "u".into(), target_name: "alice".into(),
        action_type: action_type.into(), reason: "r".into(),
        gravity: Some(ModerationGravity::Medium), duration: None,
        created_at: Utc::now(),
    }
}

fn sample_rule(guild_id: &str, flag: FlagType, enabled: bool) -> Rule {
    let now = Utc::now();
    Rule {
        id: Uuid::new_v4(), guild_id: guild_id.into(), flag_type: flag,
        weight: 3.0, threshold_warn: 2.0, threshold_delete: 4.0,
        threshold_mute: 6.0, threshold_ban: 9.0,
        enabled, created_at: now, updated_at: now,
    }
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("GET").uri(uri).body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

async fn patch_json(app: axum::Router, uri: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let req = Request::builder().method("PATCH").uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap())).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

// ══════════════════════════════════════════════════════════
// GET /api/stats (dashboard aggregate)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_dashboard_stats_returns_dto() {
    let mocks = TestMocks {
        stats: Arc::new(MockStatsUC { dashboard: sample_dashboard() }),
        infractions: Arc::new(MockInfractionsUC::default()),
        moderation: Arc::new(MockModerationUC::default()),
        rules: Arc::new(MockRulesUC::default()),
    };
    let app = router::build_for_test(build_state(&mocks));
    let (status, json) = get(app, "/api/stats").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["total_servers"], 5);
    assert_eq!(json["messages_today"], 1234);
    assert_eq!(json["postgres_online"], true);
}

// ══════════════════════════════════════════════════════════
// GET /api/infractions (merge infractions + moderation_actions)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_all_infractions_merges_and_sorts() {
    let infractions = MockInfractionsUC::default();
    infractions.items.lock().unwrap().push(sample_infraction("111111111111111111", Action::Warn));
    let moderation = MockModerationUC::default();
    moderation.actions.lock().unwrap().push(sample_action("111111111111111111", "ban_permanent"));

    let mocks = TestMocks {
        stats: Arc::new(MockStatsUC { dashboard: sample_dashboard() }),
        infractions: Arc::new(infractions),
        moderation: Arc::new(moderation),
        rules: Arc::new(MockRulesUC::default()),
    };
    let app = router::build_for_test(build_state(&mocks));
    let (status, json) = get(app, "/api/infractions").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    // Tri DESC par created_at — tous deux a Utc::now() => ordre variable, mais on doit avoir
    // 1 automod + 1 action humaine.
    let sources: Vec<&str> = arr.iter().map(|v| v["source"].as_str().unwrap()).collect();
    assert!(sources.contains(&"detection"));
    assert!(sources.contains(&"action"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_all_infractions_filter_by_guild() {
    let infractions = MockInfractionsUC::default();
    infractions.items.lock().unwrap().push(sample_infraction("111111111111111111", Action::Warn));
    infractions.items.lock().unwrap().push(sample_infraction("222222222222222222", Action::Mute));

    let mocks = TestMocks {
        stats: Arc::new(MockStatsUC { dashboard: sample_dashboard() }),
        infractions: Arc::new(infractions),
        moderation: Arc::new(MockModerationUC::default()),
        rules: Arc::new(MockRulesUC::default()),
    };
    let app = router::build_for_test(build_state(&mocks));
    let (status, json) = get(app, "/api/infractions?guild_id=111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["server"], "111111111111111111");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_all_infractions_empty() {
    let mocks = TestMocks {
        stats: Arc::new(MockStatsUC { dashboard: sample_dashboard() }),
        infractions: Arc::new(MockInfractionsUC::default()),
        moderation: Arc::new(MockModerationUC::default()),
        rules: Arc::new(MockRulesUC::default()),
    };
    let app = router::build_for_test(build_state(&mocks));
    let (status, json) = get(app, "/api/infractions").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

// ══════════════════════════════════════════════════════════
// GET /api/rules (dashboard)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_all_rules_without_filter_returns_all() {
    let rules = MockRulesUC::default();
    rules.rules.lock().unwrap().push(sample_rule("g1", FlagType::Spam, true));
    rules.rules.lock().unwrap().push(sample_rule("g2", FlagType::Insult, false));
    let mocks = TestMocks {
        stats: Arc::new(MockStatsUC { dashboard: sample_dashboard() }),
        infractions: Arc::new(MockInfractionsUC::default()),
        moderation: Arc::new(MockModerationUC::default()),
        rules: Arc::new(rules),
    };
    let app = router::build_for_test(build_state(&mocks));
    let (status, json) = get(app, "/api/rules").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_all_rules_filter_by_guild() {
    let rules = MockRulesUC::default();
    rules.rules.lock().unwrap().push(sample_rule("111111111111111111", FlagType::Spam, true));
    rules.rules.lock().unwrap().push(sample_rule("222222222222222222", FlagType::Insult, true));
    let mocks = TestMocks {
        stats: Arc::new(MockStatsUC { dashboard: sample_dashboard() }),
        infractions: Arc::new(MockInfractionsUC::default()),
        moderation: Arc::new(MockModerationUC::default()),
        rules: Arc::new(rules),
    };
    let app = router::build_for_test(build_state(&mocks));
    let (status, json) = get(app, "/api/rules?guild_id=111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
}

// ══════════════════════════════════════════════════════════
// PATCH /api/rules/{id} (toggle)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn toggle_rule_forwards_call_and_returns_new_state() {
    let rules = Arc::new(MockRulesUC::default());
    let mocks = TestMocks {
        stats: Arc::new(MockStatsUC { dashboard: sample_dashboard() }),
        infractions: Arc::new(MockInfractionsUC::default()),
        moderation: Arc::new(MockModerationUC::default()),
        rules: rules.clone(),
    };
    let app = router::build_for_test(build_state(&mocks));
    let id = Uuid::new_v4();
    let (status, json) = patch_json(app, &format!("/api/rules/{id}"), serde_json::json!({"enabled": false})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["enabled"], false);
    let toggled = rules.toggled.lock().unwrap();
    assert_eq!(toggled.len(), 1);
    assert_eq!(toggled[0].0, id);
    assert_eq!(toggled[0].1, false);
}
