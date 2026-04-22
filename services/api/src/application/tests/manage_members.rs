//! Tests pour ManageMembersService. On couvre les pass-through + get_member
//! (404 path + success). get_member_summary est couvert par les tests HTTP
//! integration (members_http) qui testent le flow complet avec stubs.

use super::*;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::application::ManageMembersService;
use crate::domain::entities::GuildMember;
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_members::{
    ManageMembersUseCase, RegisterMemberCommand, SyncMembersCommand, UpdateMemberCommand,
};
use crate::ports::outbound::MemberRepository;

fn sample_member(g: &str, u: &str, name: &str) -> GuildMember {
    GuildMember {
        guild_id: g.into(), user_id: u.into(), username: name.into(),
        display_name: None, avatar: None, roles: serde_json::json!([]),
        joined_at: None, account_created: None, is_bot: false, last_seen_at: None,
    }
}

#[derive(Default)]
struct MockMemberRepo {
    members: Mutex<Vec<GuildMember>>,
    upserts: Mutex<Vec<GuildMember>>,
    upsert_many_calls: Mutex<Vec<Vec<GuildMember>>>,
    deletes: Mutex<Vec<(String, String)>>,
}
#[async_trait]
impl MemberRepository for MockMemberRepo {
    async fn find_by_guild(&self, _: &str) -> Result<Vec<GuildMember>, DomainError> {
        Ok(self.members.lock().unwrap().clone())
    }
    async fn find_one(&self, g: &str, u: &str) -> Result<Option<GuildMember>, DomainError> {
        Ok(self.members.lock().unwrap().iter().find(|m| m.guild_id == g && m.user_id == u).cloned())
    }
    async fn upsert(&self, m: &GuildMember) -> Result<(), DomainError> {
        self.upserts.lock().unwrap().push(m.clone()); Ok(())
    }
    async fn upsert_many(&self, m: &[GuildMember]) -> Result<u64, DomainError> {
        self.upsert_many_calls.lock().unwrap().push(m.to_vec());
        Ok(m.len() as u64)
    }
    async fn delete(&self, g: &str, u: &str) -> Result<(), DomainError> {
        self.deletes.lock().unwrap().push((g.into(), u.into())); Ok(())
    }
    async fn update_last_seen(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
}

// ── Stubs minimaux pour les use cases satellites (non utilises ici) ──

use crate::ports::inbound::{
    AddPointsCommand, DeductPointsCommand, InfractionFilters, LogModerationCommand,
    ManageConductUseCase, ManageInfractionsUseCase, ManageModerationUseCase, ManageStatsUseCase,
    SaveConductConfigCommand,
};
use crate::ports::inbound::manage_stats::{RecordMessagesCommand, RecordVoiceCommand};
use crate::domain::entities::{
    ConductConfig, ConductPointsLog, DashboardStats, GuildStatsOverview, GuildVoiceStats,
    Infraction, ModerationAction, UserConductPoints, UserModerationHistory, UserStats,
};

struct StubInfUc;
#[async_trait]
impl ManageInfractionsUseCase for StubInfUc {
    async fn list_infractions(&self, _: &str, _: InfractionFilters) -> Result<Vec<Infraction>, DomainError> { Ok(vec![]) }
    async fn list_all_infractions(&self, _: i64, _: i64) -> Result<Vec<Infraction>, DomainError> { Ok(vec![]) }
    async fn count_today(&self) -> Result<u64, DomainError> { Ok(0) }
    async fn find_by_id(&self, _: &str) -> Result<Option<Infraction>, DomainError> { Ok(None) }
    async fn delete_infraction(&self, _: &str) -> Result<bool, DomainError> { Ok(false) }
    async fn delete_older_than_days(&self, _: &str, _: i32) -> Result<u64, DomainError> { Ok(0) }
}

struct StubModUc;
#[async_trait]
impl ManageModerationUseCase for StubModUc {
    async fn log_action(&self, _: LogModerationCommand) -> Result<ModerationAction, DomainError> { unimplemented!() }
    async fn get_history(&self, _: &str, t: &str) -> Result<UserModerationHistory, DomainError> {
        Ok(UserModerationHistory {
            target_id: t.into(), target_name: "t".into(),
            total_warns: 0, total_mutes: 0, total_bans: 0, actions: vec![],
        })
    }
    async fn list_bans(&self, _: Option<&str>, _: i64, _: i64) -> Result<Vec<ModerationAction>, DomainError> { Ok(vec![]) }
    async fn list_actions(&self, _: Option<&str>, _: i64) -> Result<Vec<ModerationAction>, DomainError> { Ok(vec![]) }
    async fn delete_bans_for_user(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn delete_action(&self, _: uuid::Uuid) -> Result<bool, DomainError> { Ok(false) }
}

struct StubConductUc;
#[async_trait]
impl ManageConductUseCase for StubConductUc {
    async fn get_config(&self, g: &str) -> Result<ConductConfig, DomainError> {
        Ok(ConductConfig::default_for_guild(g))
    }
    async fn save_config(&self, _: SaveConductConfigCommand) -> Result<ConductConfig, DomainError> { unimplemented!() }
    async fn get_points(&self, g: &str, u: &str) -> Result<UserConductPoints, DomainError> {
        let now = chrono::Utc::now();
        Ok(UserConductPoints {
            id: uuid::Uuid::new_v4(),
            guild_id: g.into(), user_id: u.into(), username: u.into(),
            points: 100,
            last_regen_at: now, created_at: now, updated_at: now,
        })
    }
    async fn deduct_points(&self, _: DeductPointsCommand) -> Result<UserConductPoints, DomainError> { unimplemented!() }
    async fn add_points(&self, _: AddPointsCommand) -> Result<UserConductPoints, DomainError> { unimplemented!() }
    async fn get_leaderboard(&self, _: &str, _: i64) -> Result<Vec<UserConductPoints>, DomainError> { Ok(vec![]) }
    async fn get_points_log(&self, _: &str, _: &str, _: i64) -> Result<Vec<ConductPointsLog>, DomainError> { Ok(vec![]) }
    async fn run_regen(&self) -> Result<u64, DomainError> { Ok(0) }
}

struct StubStatsUc;
#[async_trait]
impl ManageStatsUseCase for StubStatsUc {
    async fn record_messages(&self, _: RecordMessagesCommand) -> Result<(), DomainError> { Ok(()) }
    async fn record_voice(&self, _: RecordVoiceCommand) -> Result<(), DomainError> { Ok(()) }
    async fn get_user_stats(&self, _: &str, _: &str) -> Result<Option<UserStats>, DomainError> { Ok(None) }
    async fn get_guild_overview(&self, _: &str) -> Result<GuildStatsOverview, DomainError> { unimplemented!() }
    async fn get_leaderboard(&self, _: &str, _: u32) -> Result<Vec<UserStats>, DomainError> { Ok(vec![]) }
    async fn get_dashboard_stats(&self) -> Result<DashboardStats, DomainError> { unimplemented!() }
    async fn get_guild_voice_stats(&self, _: &str, _: u32, _: u32) -> Result<GuildVoiceStats, DomainError> { unimplemented!() }
}

fn make_service(repo: Arc<MockMemberRepo>) -> ManageMembersService {
    ManageMembersService::new(
        repo,
        Arc::new(StubInfUc),
        Arc::new(StubModUc),
        Arc::new(StubConductUc),
        Arc::new(StubStatsUc),
    )
}

#[tokio::test]
async fn list_members_returns_repo_data() {
    let r = Arc::new(MockMemberRepo::default());
    r.members.lock().unwrap().push(sample_member("g", "u", "Alice"));
    let svc = make_service(r);
    let members = svc.list_members("g").await.unwrap();
    assert_eq!(members.len(), 1);
}

#[tokio::test]
async fn get_member_not_found_returns_404() {
    let svc = make_service(Arc::new(MockMemberRepo::default()));
    let err = svc.get_member("g", "u").await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test]
async fn get_member_found_returns_member() {
    let r = Arc::new(MockMemberRepo::default());
    r.members.lock().unwrap().push(sample_member("g", "u", "Alice"));
    let svc = make_service(r);
    let m = svc.get_member("g", "u").await.unwrap();
    assert_eq!(m.username, "Alice");
}

#[tokio::test]
async fn sync_members_returns_count() {
    let r = Arc::new(MockMemberRepo::default());
    let svc = make_service(r.clone());
    let n = svc.sync_members(SyncMembersCommand {
        guild_id: "g".into(),
        members: vec![sample_member("g", "u1", "A"), sample_member("g", "u2", "B")],
    }).await.unwrap();
    assert_eq!(n, 2);
    assert_eq!(r.upsert_many_calls.lock().unwrap()[0].len(), 2);
}

#[tokio::test]
async fn register_member_forwards_upsert() {
    let r = Arc::new(MockMemberRepo::default());
    let svc = make_service(r.clone());
    svc.register_member(RegisterMemberCommand {
        member: sample_member("g", "u", "A")
    }).await.unwrap();
    assert_eq!(r.upserts.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn remove_member_forwards_delete() {
    let r = Arc::new(MockMemberRepo::default());
    let svc = make_service(r.clone());
    svc.remove_member("g", "u").await.unwrap();
    assert_eq!(r.deletes.lock().unwrap()[0], ("g".into(), "u".into()));
}

#[tokio::test]
async fn update_member_applies_partial_fields() {
    let r = Arc::new(MockMemberRepo::default());
    r.members.lock().unwrap().push(sample_member("g", "u", "OldName"));
    let svc = make_service(r.clone());
    svc.update_member(UpdateMemberCommand {
        guild_id: "g".into(), user_id: "u".into(),
        username: Some("NewName".into()),
        display_name: Some("Display".into()),
        avatar: None,
        roles: None,
    }).await.unwrap();
    let upserted = &r.upserts.lock().unwrap()[0];
    assert_eq!(upserted.username, "NewName");
    assert_eq!(upserted.display_name.as_deref(), Some("Display"));
}

#[tokio::test]
async fn update_member_not_found_returns_404() {
    let svc = make_service(Arc::new(MockMemberRepo::default()));
    let err = svc.update_member(UpdateMemberCommand {
        guild_id: "g".into(), user_id: "u".into(),
        username: Some("X".into()), display_name: None,
        avatar: None, roles: None,
    }).await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}
