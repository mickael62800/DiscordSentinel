//! Tests de ManageConductService. Les flows sans HTTP Discord (config + CRUD points)
//! sont couverts ici. Le mute_user (HTTP Discord PATCH) n'est pas testable sans mock.

use std::sync::Arc;
use std::sync::Mutex;
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::application::community::manage_conduct_service::ManageConductService;
use crate::domain::entities::community::conduct::ConductConfig;
use crate::domain::entities::community::conduct::ConductPointsLog;
use crate::domain::entities::moderation::infraction::Infraction;
use crate::domain::entities::community::conduct::UserConductPoints;
use crate::domain::errors::DomainError;
use crate::ports::inbound::community::manage_conduct::AddPointsCommand;
use crate::ports::inbound::community::manage_conduct::DeductPointsCommand;
use crate::ports::inbound::community::manage_conduct::ManageConductUseCase;
use crate::ports::inbound::community::manage_conduct::SaveConductConfigCommand;
use crate::ports::inbound::moderation::manage_infractions::InfractionFilters;
use crate::ports::outbound::community::conduct_repository::ConductRepository;
use crate::ports::outbound::moderation::infraction_repository::InfractionRepository;
use crate::adapters::outbound::DiscordApi;
use crate::adapters::outbound::DiscordChannel;
use crate::adapters::outbound::DiscordMember;
use crate::adapters::outbound::DiscordUser;
use crate::adapters::outbound::discord_api::UserGuild;

// ── Mocks ──

#[derive(Default)]
struct SpyDiscordApi {
    timeout_calls: Mutex<Vec<(String, String, u64)>>,
    timeout_result: Mutex<Option<DomainError>>,
}

#[async_trait]
impl DiscordApi for SpyDiscordApi {
    async fn list_text_channels(&self, _: &str) -> Result<Vec<DiscordChannel>, DomainError> { Ok(vec![]) }
    async fn upload_emoji(&self, _: &str, _: &str, _: &[u8], _: &str) -> Result<(String, String, bool), DomainError> { unimplemented!() }
    async fn ban_user(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn list_members(&self, _: &str, _: u32) -> Result<Vec<DiscordMember>, DomainError> { Ok(vec![]) }
    async fn send_dm(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn create_role(&self, _: &str, _: &str, _: u32, _: Option<&str>) -> Result<serde_json::Value, DomainError> { unimplemented!() }
    async fn edit_role(&self, _: &str, _: &str, _: Option<&str>, _: Option<u32>, _: Option<&str>, _: Option<bool>, _: Option<bool>) -> Result<serde_json::Value, DomainError> { unimplemented!() }
    async fn delete_role(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn unban_user(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn remove_timeout(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn apply_timeout(&self, guild_id: &str, user_id: &str, duration_seconds: u64) -> Result<(), DomainError> {
        self.timeout_calls.lock().unwrap().push((guild_id.into(), user_id.into(), duration_seconds));
        let err_opt = {
            let guard = self.timeout_result.lock().unwrap();
            guard.as_ref().map(|e| match e {
                DomainError::Internal(s) => DomainError::Internal(s.clone()),
                DomainError::ValidationError(s) => DomainError::ValidationError(s.clone()),
                DomainError::NotFound(s) => DomainError::NotFound(s.clone()),
                _ => DomainError::Internal("test error".into()),
            })
        };
        if let Some(err) = err_opt { return Err(err); }
        Ok(())
    }
    async fn get_user_guilds(&self, _: &str) -> Result<Vec<UserGuild>, DomainError> { Ok(vec![]) }
    async fn get_user_me(&self, _: &str) -> Result<DiscordUser, DomainError> { unimplemented!() }
}

#[derive(Default)]
struct MockConductRepo {
    config: Mutex<Option<ConductConfig>>,
    points: Mutex<Option<UserConductPoints>>,
    saved_config: Mutex<Option<ConductConfig>>,
    saved_points: Mutex<Vec<UserConductPoints>>,
    update_calls: Mutex<Vec<(String, String, i32)>>,
    logs: Mutex<Vec<ConductPointsLog>>,
    leaderboard_returns: Mutex<Vec<UserConductPoints>>,
    deleted: Mutex<Vec<(String, String)>>,
    regen_users: Mutex<Vec<UserConductPoints>>,
}

#[async_trait]
impl ConductRepository for MockConductRepo {
    async fn get_config(&self, _: &str) -> Result<Option<ConductConfig>, DomainError> {
        Ok(self.config.lock().unwrap().clone())
    }
    async fn save_config(&self, c: &ConductConfig) -> Result<(), DomainError> {
        *self.saved_config.lock().unwrap() = Some(c.clone());
        Ok(())
    }
    async fn get_points(&self, _: &str, _: &str) -> Result<Option<UserConductPoints>, DomainError> {
        Ok(self.points.lock().unwrap().clone())
    }
    async fn save_points(&self, p: &UserConductPoints) -> Result<(), DomainError> {
        self.saved_points.lock().unwrap().push(p.clone());
        *self.points.lock().unwrap() = Some(p.clone());
        Ok(())
    }
    async fn update_points(&self, g: &str, u: &str, pts: i32) -> Result<(), DomainError> {
        self.update_calls.lock().unwrap().push((g.into(), u.into(), pts));
        if let Some(p) = self.points.lock().unwrap().as_mut() {
            p.points = pts;
        }
        Ok(())
    }
    async fn get_leaderboard(&self, _: &str, _: i64) -> Result<Vec<UserConductPoints>, DomainError> {
        Ok(self.leaderboard_returns.lock().unwrap().clone())
    }
    async fn find_users_needing_regen(&self, _: &str) -> Result<Vec<UserConductPoints>, DomainError> {
        Ok(self.regen_users.lock().unwrap().clone())
    }
    async fn update_regen_timestamp(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn delete_points(&self, g: &str, u: &str) -> Result<(), DomainError> {
        self.deleted.lock().unwrap().push((g.into(), u.into()));
        Ok(())
    }
    async fn save_log(&self, l: &ConductPointsLog) -> Result<(), DomainError> {
        self.logs.lock().unwrap().push(l.clone()); Ok(())
    }
    async fn get_log(&self, _: &str, _: &str, _: i64) -> Result<Vec<ConductPointsLog>, DomainError> {
        Ok(self.logs.lock().unwrap().clone())
    }
}

#[derive(Default)]
struct MockInfRepo {
    saved: Mutex<Vec<Infraction>>,
}
#[async_trait]
impl InfractionRepository for MockInfRepo {
    async fn save(&self, i: &Infraction) -> Result<(), DomainError> {
        self.saved.lock().unwrap().push(i.clone()); Ok(())
    }
    async fn find_by_guild(&self, _: &str, _: &InfractionFilters) -> Result<Vec<Infraction>, DomainError> { Ok(vec![]) }
    async fn find_all(&self, _: i64, _: i64) -> Result<Vec<Infraction>, DomainError> { Ok(vec![]) }
    async fn count_today(&self) -> Result<u64, DomainError> { Ok(0) }
    async fn find_by_id(&self, _: &str) -> Result<Option<Infraction>, DomainError> { Ok(None) }
    async fn delete_by_id(&self, _: &str) -> Result<bool, DomainError> { Ok(false) }
    async fn delete_older_than_days(&self, _: &str, _: i32) -> Result<u64, DomainError> { Ok(0) }
}

fn make_svc(repo: Arc<MockConductRepo>, inf: Arc<MockInfRepo>) -> ManageConductService {
    ManageConductService::new(repo, inf, Arc::new(EventBroadcaster::new()), Arc::new(SpyDiscordApi::default()))
}

fn make_svc_with_spy(
    repo: Arc<MockConductRepo>,
    inf: Arc<MockInfRepo>,
    spy: Arc<SpyDiscordApi>,
) -> ManageConductService {
    ManageConductService::new(repo, inf, Arc::new(EventBroadcaster::new()), spy)
}

// ── Tests ──

#[tokio::test]
async fn get_config_falls_back_to_default_when_missing() {
    let svc = make_svc(Arc::new(MockConductRepo::default()), Arc::new(MockInfRepo::default()));
    let got = svc.get_config("g1").await.unwrap();
    assert_eq!(got.guild_id, "g1");
    assert_eq!(got.max_points, 12); // defaut
}

#[tokio::test]
async fn get_config_returns_existing() {
    let repo = Arc::new(MockConductRepo::default());
    *repo.config.lock().unwrap() = Some(ConductConfig {
        guild_id: "g".into(), max_points: 50, regen_amount: 5,
        regen_interval: "monthly".into(),
        penalty_warn: 2, penalty_delete: 4, penalty_mute: 8, penalty_ban: 20,
        created_at: Utc::now(), updated_at: Utc::now(),
    });
    let svc = make_svc(repo, Arc::new(MockInfRepo::default()));
    let got = svc.get_config("g").await.unwrap();
    assert_eq!(got.max_points, 50);
    assert_eq!(got.regen_interval, "monthly");
}

#[tokio::test]
async fn save_config_persists() {
    let repo = Arc::new(MockConductRepo::default());
    let svc = make_svc(repo.clone(), Arc::new(MockInfRepo::default()));
    let saved = svc.save_config(SaveConductConfigCommand {
        guild_id: "g".into(), max_points: 20,
        regen_amount: 2, regen_interval: "weekly".into(),
        penalty_warn: 1, penalty_delete: 2, penalty_mute: 3, penalty_ban: 6,
    }).await.unwrap();
    assert_eq!(saved.max_points, 20);
    assert!(repo.saved_config.lock().unwrap().is_some());
}

#[tokio::test]
async fn get_points_creates_at_max_when_absent() {
    let repo = Arc::new(MockConductRepo::default());
    let svc = make_svc(repo.clone(), Arc::new(MockInfRepo::default()));
    let got = svc.get_points("g", "u").await.unwrap();
    assert_eq!(got.points, 12); // max par defaut
    assert_eq!(repo.saved_points.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn deduct_points_zero_penalty_is_noop_but_creates_row() {
    let repo = Arc::new(MockConductRepo::default());
    let svc = make_svc(repo.clone(), Arc::new(MockInfRepo::default()));
    let out = svc.deduct_points(DeductPointsCommand {
        guild_id: "g".into(), user_id: "u".into(), username: "Alice".into(),
        action: "unknown_action".into(),
    }).await.unwrap();
    // Action inconnue -> penalty=0 -> juste creer la row sans update.
    assert_eq!(out.points, 12);
    assert!(repo.update_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn deduct_points_applies_penalty_and_logs() {
    let repo = Arc::new(MockConductRepo::default());
    let svc = make_svc(repo.clone(), Arc::new(MockInfRepo::default()));
    // Warn = penalty 1 (defaut).
    let out = svc.deduct_points(DeductPointsCommand {
        guild_id: "g".into(), user_id: "u".into(), username: "A".into(),
        action: "warn".into(),
    }).await.unwrap();
    assert_eq!(out.points, 11); // 12 - 1
    assert_eq!(repo.update_calls.lock().unwrap()[0].2, 11);
    assert_eq!(repo.logs.lock().unwrap()[0].delta, -1);
}

#[tokio::test]
async fn deduct_points_to_zero_creates_ban_infraction() {
    let repo = Arc::new(MockConductRepo::default());
    // Points a 6 deja -> ban (penalty 6 defaut) -> 0.
    let now = Utc::now();
    *repo.points.lock().unwrap() = Some(UserConductPoints {
        id: Uuid::new_v4(), guild_id: "g".into(), user_id: "u".into(),
        username: "A".into(), points: 6,
        last_regen_at: now, created_at: now, updated_at: now,
    });
    let inf = Arc::new(MockInfRepo::default());
    let svc = make_svc(repo.clone(), inf.clone());
    let out = svc.deduct_points(DeductPointsCommand {
        guild_id: "g".into(), user_id: "u".into(), username: "A".into(),
        action: "ban".into(),
    }).await.unwrap();
    assert_eq!(out.points, 0);
    // Une infraction ban auto doit etre saved.
    let saved = inf.saved.lock().unwrap();
    assert_eq!(saved.len(), 1);
    assert!(saved[0].reason.contains("tombes a 0"));
}

#[tokio::test]
async fn add_points_clamps_at_max() {
    let repo = Arc::new(MockConductRepo::default());
    let svc = make_svc(repo.clone(), Arc::new(MockInfRepo::default()));
    // Points crees au max (12).
    let out = svc.add_points(AddPointsCommand {
        guild_id: "g".into(), user_id: "u".into(),
        amount: 100, reason: "amnistie".into(),
    }).await.unwrap();
    assert_eq!(out.points, 12); // clampe au max
    assert_eq!(repo.logs.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn add_points_below_max_increments() {
    let now = Utc::now();
    let repo = Arc::new(MockConductRepo::default());
    *repo.points.lock().unwrap() = Some(UserConductPoints {
        id: Uuid::new_v4(), guild_id: "g".into(), user_id: "u".into(),
        username: "A".into(), points: 5,
        last_regen_at: now, created_at: now, updated_at: now,
    });
    let svc = make_svc(repo.clone(), Arc::new(MockInfRepo::default()));
    let out = svc.add_points(AddPointsCommand {
        guild_id: "g".into(), user_id: "u".into(),
        amount: 3, reason: "good".into(),
    }).await.unwrap();
    assert_eq!(out.points, 8);
}

#[tokio::test]
async fn get_leaderboard_forwards_to_repo() {
    let repo = Arc::new(MockConductRepo::default());
    let now = Utc::now();
    *repo.leaderboard_returns.lock().unwrap() = vec![
        UserConductPoints {
            id: Uuid::new_v4(), guild_id: "g".into(), user_id: "u1".into(),
            username: "Alice".into(), points: 12,
            last_regen_at: now, created_at: now, updated_at: now,
        },
    ];
    let svc = make_svc(repo, Arc::new(MockInfRepo::default()));
    let board = svc.get_leaderboard("g", 10).await.unwrap();
    assert_eq!(board.len(), 1);
}

#[tokio::test]
async fn get_points_log_forwards() {
    let repo = Arc::new(MockConductRepo::default());
    let svc = make_svc(repo, Arc::new(MockInfRepo::default()));
    let logs = svc.get_points_log("g", "u", 50).await.unwrap();
    assert!(logs.is_empty());
}

// ── mute_user via DiscordApi (timeout) ──

#[tokio::test]
async fn deduct_points_to_zero_calls_discord_timeout() {
    let repo = Arc::new(MockConductRepo::default());
    let now = Utc::now();
    *repo.points.lock().unwrap() = Some(UserConductPoints {
        id: Uuid::new_v4(), guild_id: "g".into(), user_id: "u".into(),
        username: "A".into(), points: 6,
        last_regen_at: now, created_at: now, updated_at: now,
    });
    let spy = Arc::new(SpyDiscordApi::default());
    let svc = make_svc_with_spy(repo, Arc::new(MockInfRepo::default()), spy.clone());
    svc.deduct_points(DeductPointsCommand {
        guild_id: "g".into(), user_id: "u".into(), username: "A".into(),
        action: "ban".into(),
    }).await.unwrap();
    let calls = spy.timeout_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "g");
    assert_eq!(calls[0].1, "u");
    // Duration = MUTE_AT_ZERO_POINTS_DURATION_MINS * 60 secondes.
    assert!(calls[0].2 > 0);
}

#[tokio::test]
async fn deduct_points_to_zero_swallows_discord_error() {
    let repo = Arc::new(MockConductRepo::default());
    let now = Utc::now();
    *repo.points.lock().unwrap() = Some(UserConductPoints {
        id: Uuid::new_v4(), guild_id: "g".into(), user_id: "u".into(),
        username: "A".into(), points: 6,
        last_regen_at: now, created_at: now, updated_at: now,
    });
    let spy = Arc::new(SpyDiscordApi::default());
    *spy.timeout_result.lock().unwrap() = Some(DomainError::Internal("discord 401".into()));
    let inf = Arc::new(MockInfRepo::default());
    let svc = make_svc_with_spy(repo, inf.clone(), spy.clone());
    // L'erreur Discord doit etre avalee, deduct_points reussit quand meme.
    let out = svc.deduct_points(DeductPointsCommand {
        guild_id: "g".into(), user_id: "u".into(), username: "A".into(),
        action: "ban".into(),
    }).await.unwrap();
    assert_eq!(out.points, 0);
    assert_eq!(spy.timeout_calls.lock().unwrap().len(), 1);
    // L'infraction auto-ban est quand meme persistee malgre l'erreur Discord.
    assert_eq!(inf.saved.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn deduct_points_non_zero_does_not_call_discord() {
    let repo = Arc::new(MockConductRepo::default());
    let spy = Arc::new(SpyDiscordApi::default());
    let svc = make_svc_with_spy(repo, Arc::new(MockInfRepo::default()), spy.clone());
    // Warn : 12 → 11, pas de mute
    svc.deduct_points(DeductPointsCommand {
        guild_id: "g".into(), user_id: "u".into(), username: "A".into(),
        action: "warn".into(),
    }).await.unwrap();
    assert!(spy.timeout_calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn run_regen_processes_users_below_max_and_deletes_maxed() {
    let repo = Arc::new(MockConductRepo::default());
    let now = Utc::now();
    // Un user deja au max (doit etre delete).
    // Un user a 5 (doit etre +1 -> 6).
    *repo.regen_users.lock().unwrap() = vec![
        UserConductPoints {
            id: Uuid::new_v4(), guild_id: "g".into(), user_id: "maxed".into(),
            username: "M".into(), points: 12,
            last_regen_at: now, created_at: now, updated_at: now,
        },
        UserConductPoints {
            id: Uuid::new_v4(), guild_id: "g".into(), user_id: "low".into(),
            username: "L".into(), points: 5,
            last_regen_at: now, created_at: now, updated_at: now,
        },
    ];
    let svc = make_svc(repo.clone(), Arc::new(MockInfRepo::default()));
    let total = svc.run_regen().await.unwrap();
    // 2 users * 2 intervalles (weekly+monthly), mais le mock renvoie la meme
    // liste aux 2 calls -> ~4 processings. Verifie au moins 2 traitements.
    assert!(total >= 2);
    let deleted = repo.deleted.lock().unwrap();
    assert!(deleted.iter().any(|(_, u)| u == "maxed"));
}
