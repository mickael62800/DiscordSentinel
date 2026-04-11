//! Tests d'integration end-to-end pour les services gRPC restants.
//!
//! Couvre les services qui ne dependent que d'un trait UseCase (pas de
//! pool sqlx, pas de struct concrete) — donc mockables en in-process :
//!
//! - AutomodService          (AnalyzeMessageUseCase)
//! - ImagesService           (AnalyzeImageUseCase)
//! - SecurityService         (ManageSecurityUseCase)
//! - MembersService          (ManageMembersUseCase)
//! - ModerationService       (ManageModerationUseCase)
//! - StatsService            (ManageStatsUseCase + EventBroadcaster)
//! - ProgressionService      (ManageLevelsUseCase + EventBroadcaster)
//!
//! Les services skippes et la raison :
//! - BlackjackService : depend de `BlackjackApp` (struct concrete avec
//!   etat interne, pas un trait → mockage non trivial).
//! - TicketsService   : depend de sqlx::PgPool pour UpdateSla (besoin
//!   d'une vraie DB ou d'un mock pool).
//! - WelcomeService   : sqlx::PgPool only (pas de trait, besoin DB).
//! - CommunityService : sqlx::PgPool only (idem).
//! - VoiceChannelsService : trait avec >25 methodes (mock too verbose,
//!   ROI faible vu le converter unique).
//! - RolePanelsService : trait nombreux + DiscordRoleRepo. ROI moyen.
//! - CoudePlayerService et 5 services F.1 : deja couverts dans
//!   `grpc_coude_e2e.rs`.
//!
//! Ces tests valident la chaine wiring proto -> handler -> use case ->
//! conversion -> reponse pour 7 services additionnels.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use tokio::sync::oneshot;
use tonic::transport::{Endpoint, Server};
use uuid::Uuid;

use sentinel_api::adapters::inbound::grpc::automod::AutomodGrpc;
use sentinel_api::adapters::inbound::grpc::images::ImagesGrpc;
use sentinel_api::adapters::inbound::grpc::members::MembersGrpc;
use sentinel_api::adapters::inbound::grpc::moderation::ModerationGrpc;
use sentinel_api::adapters::inbound::grpc::progression::ProgressionGrpc;
use sentinel_api::adapters::inbound::grpc::security::SecurityGrpc;
use sentinel_api::adapters::inbound::grpc::stats::StatsGrpc;
use sentinel_api::adapters::inbound::ws::broadcaster::EventBroadcaster;
use sentinel_api::domain::entities::{
    DashboardStats, GuildMember, GuildStatsOverview, GuildVoiceStats, ImageAnalysis,
    LevelConfig, LevelReward, MemberSummary, MessageAnalysis, ModerationAction, SecurityEvent,
    UserLevel, UserModerationHistory, UserStats, XpSource,
};
use sentinel_api::domain::errors::DomainError;
use sentinel_api::domain::value_objects::Action;
use sentinel_api::ports::inbound::{
    AnalyzeImageCommand, AnalyzeImageUseCase, AnalyzeMessageCommand, AnalyzeMessageUseCase,
    LogModerationCommand, ManageModerationUseCase, ManageSecurityUseCase, ManageStatsUseCase,
    ReportSecurityEventCommand,
};
use sentinel_api::ports::inbound::manage_stats::{RecordMessagesCommand, RecordVoiceCommand};
use sentinel_api::ports::inbound::manage_levels::{AddXpCommand, AddXpResult, ManageLevelsUseCase, SaveLevelConfigCommand};
use sentinel_api::ports::inbound::manage_members::{
    ManageMembersUseCase, RegisterMemberCommand, SyncMembersCommand, UpdateMemberCommand,
};
use sentinel_proto::automod::v1 as automod_proto;
use sentinel_proto::automod::v1::automod_service_client::AutomodServiceClient;
use sentinel_proto::automod::v1::automod_service_server::AutomodServiceServer;
use sentinel_proto::images::v1 as images_proto;
use sentinel_proto::images::v1::images_service_client::ImagesServiceClient;
use sentinel_proto::images::v1::images_service_server::ImagesServiceServer;
use sentinel_proto::members::v1 as members_proto;
use sentinel_proto::members::v1::members_service_client::MembersServiceClient;
use sentinel_proto::members::v1::members_service_server::MembersServiceServer;
use sentinel_proto::moderation::v1 as mod_proto;
use sentinel_proto::moderation::v1::moderation_service_client::ModerationServiceClient;
use sentinel_proto::moderation::v1::moderation_service_server::ModerationServiceServer;
use sentinel_proto::progression::v1 as prog_proto;
use sentinel_proto::progression::v1::progression_service_client::ProgressionServiceClient;
use sentinel_proto::progression::v1::progression_service_server::ProgressionServiceServer;
use sentinel_proto::security::v1 as sec_proto;
use sentinel_proto::security::v1::security_service_client::SecurityServiceClient;
use sentinel_proto::security::v1::security_service_server::SecurityServiceServer;
use sentinel_proto::stats::v1 as stats_proto;
use sentinel_proto::stats::v1::stats_service_client::StatsServiceClient;
use sentinel_proto::stats::v1::stats_service_server::StatsServiceServer;

fn ts() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 4, 11, 12, 0, 0).unwrap()
}

macro_rules! spawn_one_service {
    ($svc:expr) => {{
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{addr}");
        let (tx, rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            Server::builder()
                .add_service($svc)
                .serve_with_incoming_shutdown(
                    tokio_stream::wrappers::TcpListenerStream::new(listener),
                    async {
                        let _ = rx.await;
                    },
                )
                .await
                .unwrap();
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        (url, tx)
    }};
}

// ══════════════════════════════════════════════════════════════════════
// AutomodService
// ══════════════════════════════════════════════════════════════════════

struct MockAnalyzeMessage;

#[async_trait]
impl AnalyzeMessageUseCase for MockAnalyzeMessage {
    async fn analyze(&self, cmd: AnalyzeMessageCommand) -> Result<MessageAnalysis, DomainError> {
        // Logique de mock : flag spam -> Warn, sinon None.
        let action = if cmd.flags.spam { Action::Warn } else { Action::None };
        Ok(MessageAnalysis {
            action,
            reason: if cmd.flags.spam { "spam detecte" } else { "ras" }.into(),
            score: if cmd.flags.spam { 0.85 } else { 0.0 },
            duration: None,
        })
    }
}

#[tokio::test]
async fn automod_analyze_returns_action_for_flagged_message() {
    let svc = AutomodServiceServer::new(AutomodGrpc { uc: Arc::new(MockAnalyzeMessage) });
    let (url, shutdown) = spawn_one_service!(svc);
    let mut client = AutomodServiceClient::connect(Endpoint::from_shared(url).unwrap())
        .await.unwrap();

    // Message flagge spam
    let resp = client.analyze_message(automod_proto::AnalyzeMessageRequest {
        guild_id: "g".into(),
        channel_id: "c".into(),
        user_id: "u".into(),
        username: "joe".into(),
        content: "buy now!!!".into(),
        flags: Some(automod_proto::DetectionFlags {
            spam: true, insult: false, link: false, phishing: false,
        }),
        message_id: "m".into(),
        timestamp: ts().to_rfc3339(),
        context_messages: vec![],
    }).await.unwrap().into_inner();
    assert_eq!(resp.action, automod_proto::Action::Warn as i32);
    assert!(resp.reason.contains("spam"));

    // Message clean
    let clean = client.analyze_message(automod_proto::AnalyzeMessageRequest {
        guild_id: "g".into(), channel_id: "c".into(),
        user_id: "u".into(), username: "joe".into(),
        content: "Hi everyone".into(),
        flags: Some(automod_proto::DetectionFlags::default()),
        message_id: "m".into(), timestamp: ts().to_rfc3339(),
        context_messages: vec![],
    }).await.unwrap().into_inner();
    assert_eq!(clean.action, automod_proto::Action::None as i32);

    let _ = shutdown.send(());
}

// ══════════════════════════════════════════════════════════════════════
// ImagesService
// ══════════════════════════════════════════════════════════════════════

struct MockAnalyzeImage;

#[async_trait]
impl AnalyzeImageUseCase for MockAnalyzeImage {
    async fn analyze_image(&self, cmd: AnalyzeImageCommand) -> Result<ImageAnalysis, DomainError> {
        // Mock : si filename contient "weapon", banni; sinon ok.
        if cmd.filename.contains("weapon") {
            Ok(ImageAnalysis {
                action: Action::Ban,
                reason: "arme detectee".into(),
                score: 0.95,
                duration: Some(120),
                classifications: vec![],
            })
        } else {
            Ok(ImageAnalysis {
                action: Action::None,
                reason: "image safe".into(),
                score: 0.05,
                duration: Some(50),
                classifications: vec![],
            })
        }
    }
}

#[tokio::test]
async fn images_analyze_classifies_by_filename() {
    let svc = ImagesServiceServer::new(ImagesGrpc { uc: Arc::new(MockAnalyzeImage) });
    let (url, shutdown) = spawn_one_service!(svc);
    let mut client = ImagesServiceClient::connect(Endpoint::from_shared(url).unwrap())
        .await.unwrap();

    let dangerous = client.analyze_image(images_proto::AnalyzeImageRequest {
        guild_id: "g".into(), channel_id: "c".into(),
        user_id: "u".into(), username: "joe".into(),
        message_id: "m".into(),
        image_data: b"fake png bytes".to_vec(),
        content_type: "image/png".into(),
        filename: "weapon.png".into(),
    }).await.unwrap().into_inner();
    assert_eq!(dangerous.action, images_proto::Action::Ban as i32);
    assert!(dangerous.reason.contains("arme"));

    let safe = client.analyze_image(images_proto::AnalyzeImageRequest {
        guild_id: "g".into(), channel_id: "c".into(),
        user_id: "u".into(), username: "joe".into(),
        message_id: "m".into(),
        image_data: b"png".to_vec(),
        content_type: "image/png".into(),
        filename: "cat.png".into(),
    }).await.unwrap().into_inner();
    assert_eq!(safe.action, images_proto::Action::None as i32);

    let _ = shutdown.send(());
}

// ══════════════════════════════════════════════════════════════════════
// SecurityService
// ══════════════════════════════════════════════════════════════════════

struct MockSecurityUc;

#[async_trait]
impl ManageSecurityUseCase for MockSecurityUc {
    async fn report_event(&self, cmd: ReportSecurityEventCommand) -> Result<SecurityEvent, DomainError> {
        Ok(SecurityEvent {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            event_type: cmd.event_type,
            severity: cmd.severity,
            description: cmd.description,
            user_ids: cmd.user_ids,
            created_at: ts(),
        })
    }
    async fn list_events(&self, _: Option<&str>) -> Result<Vec<SecurityEvent>, DomainError> {
        Ok(vec![SecurityEvent {
            id: Uuid::nil(),
            guild_id: "g".into(),
            event_type: "raid".into(),
            severity: "high".into(),
            description: "test".into(),
            user_ids: vec!["u1".into()],
            created_at: ts(),
        }])
    }
}

#[tokio::test]
async fn security_report_and_list_round_trip() {
    let svc = SecurityServiceServer::new(SecurityGrpc { uc: Arc::new(MockSecurityUc) });
    let (url, shutdown) = spawn_one_service!(svc);
    let mut client = SecurityServiceClient::connect(Endpoint::from_shared(url).unwrap())
        .await.unwrap();

    let reported = client.report_event(sec_proto::ReportEventRequest {
        guild_id: "g".into(),
        event_type: "scan".into(),
        severity: "info".into(),
        description: "Daily scan complete".into(),
        user_ids: vec!["u1".into(), "u2".into()],
    }).await.unwrap().into_inner();
    assert_eq!(reported.event_type, "scan");
    assert_eq!(reported.severity, "info");
    assert_eq!(reported.user_ids.len(), 2);

    let list = client.list_events(sec_proto::ListEventsRequest {
        guild_id: Some("g".into()),
    }).await.unwrap().into_inner();
    assert_eq!(list.events.len(), 1);
    assert_eq!(list.events[0].event_type, "raid");

    let _ = shutdown.send(());
}

// ══════════════════════════════════════════════════════════════════════
// MembersService
// ══════════════════════════════════════════════════════════════════════

struct MockMembersUc;

#[async_trait]
impl ManageMembersUseCase for MockMembersUc {
    async fn list_members(&self, guild_id: &str) -> Result<Vec<GuildMember>, DomainError> {
        Ok(vec![sample_member(guild_id, "u1"), sample_member(guild_id, "u2")])
    }
    async fn get_member(&self, guild_id: &str, user_id: &str) -> Result<GuildMember, DomainError> {
        if user_id == "missing" {
            return Err(DomainError::NotFound("introuvable".into()));
        }
        Ok(sample_member(guild_id, user_id))
    }
    async fn get_member_summary(&self, _: &str, _: &str) -> Result<MemberSummary, DomainError> { unimplemented!() }
    async fn sync_members(&self, _: SyncMembersCommand) -> Result<u64, DomainError> { Ok(2) }
    async fn register_member(&self, _: RegisterMemberCommand) -> Result<(), DomainError> { unimplemented!() }
    async fn remove_member(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn update_member(&self, _: UpdateMemberCommand) -> Result<(), DomainError> { unimplemented!() }
}

fn sample_member(guild_id: &str, user_id: &str) -> GuildMember {
    GuildMember {
        guild_id: guild_id.into(),
        user_id: user_id.into(),
        username: format!("user_{user_id}"),
        display_name: None,
        avatar: None,
        roles: serde_json::json!([]),
        joined_at: Some(ts()),
        account_created: Some(ts()),
        is_bot: false,
        last_seen_at: Some(ts()),
    }
}

#[tokio::test]
async fn members_list_and_get_round_trip() {
    let svc = MembersServiceServer::new(MembersGrpc { uc: Arc::new(MockMembersUc) });
    let (url, shutdown) = spawn_one_service!(svc);
    let mut client = MembersServiceClient::connect(Endpoint::from_shared(url).unwrap())
        .await.unwrap();

    // MembersService n'expose pas list_members en gRPC — seulement get_member.
    let one = client.get_member(members_proto::GetMemberRequest {
        guild_id: "g1".into(), user_id: "u1".into(),
    }).await.unwrap().into_inner();
    assert!(one.member.is_some());
    let m = one.member.unwrap();
    assert_eq!(m.user_id, "u1");
    assert_eq!(m.guild_id, "g1");

    // Le handler convertit volontairement NotFound -> Option::None (compat
    // avec le contrat HTTP 404 que welcome-bot consomme).
    let missing = client.get_member(members_proto::GetMemberRequest {
        guild_id: "g".into(), user_id: "missing".into(),
    }).await.unwrap().into_inner();
    assert!(missing.member.is_none(), "missing doit etre Ok(None), pas une erreur");

    let _ = shutdown.send(());
}

// ══════════════════════════════════════════════════════════════════════
// ModerationService
// ══════════════════════════════════════════════════════════════════════

struct MockModerationUc;

#[async_trait]
impl ManageModerationUseCase for MockModerationUc {
    async fn log_action(&self, cmd: LogModerationCommand) -> Result<ModerationAction, DomainError> {
        Ok(ModerationAction {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            channel_id: cmd.channel_id,
            moderator_id: cmd.moderator_id,
            moderator_name: cmd.moderator_name,
            target_id: cmd.target_id,
            target_name: cmd.target_name,
            action_type: cmd.action_type,
            reason: cmd.reason,
            gravity: None,
            duration: cmd.duration,
            created_at: ts(),
        })
    }
    async fn get_history(&self, _: &str, target_id: &str) -> Result<UserModerationHistory, DomainError> {
        Ok(UserModerationHistory {
            target_id: target_id.into(),
            target_name: "Joe".into(),
            total_warns: 2,
            total_mutes: 1,
            total_bans: 0,
            actions: vec![],
        })
    }
    async fn list_bans(&self, _: Option<&str>, _: i64, _: i64) -> Result<Vec<ModerationAction>, DomainError> { Ok(vec![]) }
    async fn delete_bans_for_user(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
}

#[tokio::test]
async fn moderation_log_action_and_get_history() {
    let svc = ModerationServiceServer::new(ModerationGrpc { moderation_uc: Arc::new(MockModerationUc) });
    let (url, shutdown) = spawn_one_service!(svc);
    let mut client = ModerationServiceClient::connect(Endpoint::from_shared(url).unwrap())
        .await.unwrap();

    let logged = client.log_action(mod_proto::LogActionRequest {
        guild_id: "g".into(),
        channel_id: "c".into(),
        moderator_id: "mod1".into(),
        moderator_name: "Mod".into(),
        target_id: "u".into(),
        target_name: "Joe".into(),
        action_type: "warn".into(),
        reason: "spam".into(),
        gravity: None,
        duration: None,
    }).await.unwrap().into_inner();
    assert_eq!(logged.action_type, "warn");
    assert_eq!(logged.target_name, "Joe");

    let history = client.get_history(mod_proto::GetHistoryRequest {
        guild_id: "g".into(),
        user_id: "u".into(),
    }).await.unwrap().into_inner();
    assert_eq!(history.total_warns, 2);
    assert_eq!(history.total_mutes, 1);
    assert_eq!(history.total_bans, 0);

    let _ = shutdown.send(());
}

// ══════════════════════════════════════════════════════════════════════
// StatsService
// ══════════════════════════════════════════════════════════════════════

struct MockStatsUc;

#[async_trait]
impl ManageStatsUseCase for MockStatsUc {
    async fn record_messages(&self, _: RecordMessagesCommand) -> Result<(), DomainError> { Ok(()) }
    async fn record_voice(&self, _: RecordVoiceCommand) -> Result<(), DomainError> { Ok(()) }
    async fn get_user_stats(&self, guild_id: &str, user_id: &str) -> Result<Option<UserStats>, DomainError> {
        if user_id == "missing" { return Ok(None); }
        Ok(Some(UserStats {
            id: Uuid::nil(),
            guild_id: guild_id.into(),
            user_id: user_id.into(),
            username: "alice".into(),
            message_count: 42,
            voice_seconds: 600,
            updated_at: ts(),
        }))
    }
    async fn get_guild_overview(&self, _: &str) -> Result<GuildStatsOverview, DomainError> { unimplemented!() }
    async fn get_leaderboard(&self, _: &str, _: u32) -> Result<Vec<UserStats>, DomainError> { Ok(vec![]) }
    async fn get_dashboard_stats(&self) -> Result<DashboardStats, DomainError> { unimplemented!() }
    async fn get_guild_voice_stats(&self, _: &str, _: u32, _: u32) -> Result<GuildVoiceStats, DomainError> { unimplemented!() }
}

#[tokio::test]
async fn stats_record_messages_and_get_user_stats() {
    let svc = StatsServiceServer::new(StatsGrpc {
        stats_uc: Arc::new(MockStatsUc),
        broadcaster: Arc::new(EventBroadcaster::new()),
    });
    let (url, shutdown) = spawn_one_service!(svc);
    let mut client = StatsServiceClient::connect(Endpoint::from_shared(url).unwrap())
        .await.unwrap();

    // Record messages — Ok
    client.record_messages(stats_proto::RecordMessagesRequest {
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "alice".into(),
        count: 5,
    }).await.unwrap();

    // Get user stats
    let resp = client.get_user_stats(stats_proto::GetUserStatsRequest {
        guild_id: "g".into(),
        user_id: "u".into(),
    }).await.unwrap().into_inner();
    assert!(resp.stats.is_some());
    assert_eq!(resp.stats.unwrap().message_count, 42);

    // Missing user -> reponse vide
    let missing = client.get_user_stats(stats_proto::GetUserStatsRequest {
        guild_id: "g".into(),
        user_id: "missing".into(),
    }).await.unwrap().into_inner();
    assert!(missing.stats.is_none());

    let _ = shutdown.send(());
}

// ══════════════════════════════════════════════════════════════════════
// ProgressionService
// ══════════════════════════════════════════════════════════════════════

struct MockLevelsUc;

#[async_trait]
impl ManageLevelsUseCase for MockLevelsUc {
    async fn add_xp(&self, cmd: AddXpCommand) -> Result<AddXpResult, DomainError> {
        let user_level = sample_user_level(&cmd.guild_id, &cmd.user_id, cmd.amount);
        Ok(AddXpResult {
            user_level,
            leveled_up: cmd.amount >= 100,
            old_level: 1,
            reward_role_id: None,
            source: cmd.source,
        })
    }
    async fn get_user_level(&self, guild_id: &str, user_id: &str) -> Result<UserLevel, DomainError> {
        if user_id == "missing" {
            return Err(DomainError::NotFound("user level absent".into()));
        }
        Ok(sample_user_level(guild_id, user_id, 250))
    }
    async fn get_leaderboard(&self, guild_id: &str, limit: i64) -> Result<Vec<UserLevel>, DomainError> {
        Ok((0..limit.min(3)).map(|i| sample_user_level(guild_id, &format!("u{i}"), 1000 - i * 100)).collect())
    }
    async fn get_rewards(&self, _: &str) -> Result<Vec<LevelReward>, DomainError> {
        Ok(vec![LevelReward {
            id: Uuid::nil(),
            guild_id: "g".into(),
            level: 5,
            role_id: "role5".into(),
            source: XpSource::Text,
        }])
    }
    async fn get_config(&self, _: &str) -> Result<LevelConfig, DomainError> { unimplemented!() }
    async fn save_config(&self, _: SaveLevelConfigCommand) -> Result<LevelConfig, DomainError> { unimplemented!() }
    async fn get_leaderboard_by_source(&self, _: &str, _: XpSource, _: i64) -> Result<Vec<UserLevel>, DomainError> { Ok(vec![]) }
    async fn get_rewards_by_source(&self, _: &str, _: XpSource) -> Result<Vec<LevelReward>, DomainError> { Ok(vec![]) }
    async fn set_reward(&self, _: &str, _: i32, _: &str, _: XpSource) -> Result<LevelReward, DomainError> { unimplemented!() }
    async fn delete_reward(&self, _: &str, _: i32, _: XpSource) -> Result<(), DomainError> { unimplemented!() }
}

fn sample_user_level(guild_id: &str, user_id: &str, xp: i64) -> UserLevel {
    UserLevel {
        id: Uuid::nil(),
        guild_id: guild_id.into(),
        user_id: user_id.into(),
        username: format!("u_{user_id}"),
        xp,
        level: ((xp as f64).sqrt() as i32 / 5).max(1),
        xp_text: xp / 2,
        level_text: 1,
        xp_voice: xp / 2,
        level_voice: 1,
        last_xp_at: ts(),
        created_at: ts(),
        updated_at: ts(),
    }
}

#[tokio::test]
async fn progression_add_xp_and_get_level_round_trip() {
    let svc = ProgressionServiceServer::new(ProgressionGrpc {
        levels_uc: Arc::new(MockLevelsUc),
        broadcaster: Arc::new(EventBroadcaster::new()),
    });
    let (url, shutdown) = spawn_one_service!(svc);
    let mut client = ProgressionServiceClient::connect(Endpoint::from_shared(url).unwrap())
        .await.unwrap();

    // Add xp avec levelup
    let added = client.add_xp(prog_proto::AddXpRequest {
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "alice".into(),
        amount: 150,
        source: 0,
    }).await.unwrap().into_inner();
    assert!(added.leveled_up);
    assert_eq!(added.old_level, 1);

    // Get level existant
    let level = client.get_user_level(prog_proto::GetUserLevelRequest {
        guild_id: "g".into(),
        user_id: "u".into(),
    }).await.unwrap().into_inner();
    assert_eq!(level.xp, 250);

    // Get level absent
    let err = client.get_user_level(prog_proto::GetUserLevelRequest {
        guild_id: "g".into(),
        user_id: "missing".into(),
    }).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::NotFound);

    // Leaderboard
    let lb = client.get_leaderboard(prog_proto::GetLeaderboardRequest {
        guild_id: "g".into(), limit: 10, source: 0,
    }).await.unwrap().into_inner();
    assert_eq!(lb.users.len(), 3);

    // Rewards
    let rewards = client.get_rewards(prog_proto::GetRewardsRequest {
        guild_id: "g".into(),
    }).await.unwrap().into_inner();
    assert_eq!(rewards.rewards.len(), 1);
    assert_eq!(rewards.rewards[0].level, 5);

    let _ = shutdown.send(());
}
