use super::*;

#[test]
fn proto_to_flags_round_trip_all_true() {
    let p = proto::DetectionFlags {
        spam: true,
        insult: true,
        link: true,
        phishing: true,
    };
    let f = proto_to_flags(p);
    assert!(f.spam && f.insult && f.link && f.phishing);
}

#[test]
fn proto_to_flags_round_trip_mixed() {
    let p = proto::DetectionFlags {
        spam: true,
        insult: false,
        link: true,
        phishing: false,
    };
    let f = proto_to_flags(p);
    assert!(f.spam);
    assert!(!f.insult);
    assert!(f.link);
    assert!(!f.phishing);
}

#[test]
fn action_to_proto_all_variants() {
    assert_eq!(action_to_proto(Action::None), proto::Action::None as i32);
    assert_eq!(action_to_proto(Action::Warn), proto::Action::Warn as i32);
    assert_eq!(
        action_to_proto(Action::Delete),
        proto::Action::Delete as i32
    );
    assert_eq!(action_to_proto(Action::Mute), proto::Action::Mute as i32);
    assert_eq!(action_to_proto(Action::Ban), proto::Action::Ban as i32);
}

#[test]
fn analysis_to_proto_full_mapping() {
    let a = MessageAnalysis {
        action: Action::Warn,
        reason: "spam".into(),
        score: 0.65,
        duration: Some(300),
        route: sentinel_core::domain::services::moderation::automod_routing::Routing::Card,
        severe: false,
        auto_delete_link: false,
    };
    let p = analysis_to_proto(a);
    assert_eq!(p.action, proto::Action::Warn as i32);
    assert_eq!(p.reason, "spam");
    assert!((p.score - 0.65).abs() < 1e-6);
    assert_eq!(p.duration, Some(300));
}

#[test]
fn analysis_to_proto_no_action() {
    let a = MessageAnalysis {
        action: Action::None,
        reason: String::new(),
        score: 0.0,
        duration: None,
        route: sentinel_core::domain::services::moderation::automod_routing::Routing::None,
        severe: false,
        auto_delete_link: false,
    };
    let p = analysis_to_proto(a);
    assert_eq!(p.action, proto::Action::None as i32);
    assert!(p.duration.is_none());
}

// ── RPC handler tests avec mock AnalyzeMessageUseCase ──

use crate::ports::inbound::ai::analyze_message::AnalyzeMessageCommand;
use crate::ports::inbound::ai::analyze_message::AnalyzeMessageUseCase;
use async_trait::async_trait;
use sentinel_core::domain::errors::DomainError;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Default)]
struct MockAnalyzeUc {
    calls: Mutex<Vec<AnalyzeMessageCommand>>,
}

#[async_trait]
impl AnalyzeMessageUseCase for MockAnalyzeUc {
    async fn analyze(&self, cmd: AnalyzeMessageCommand) -> Result<MessageAnalysis, DomainError> {
        self.calls.lock().unwrap().push(cmd);
        Ok(MessageAnalysis {
            action: Action::Warn,
            reason: "spam".into(),
            score: 0.75,
            duration: None,
            route: sentinel_core::domain::services::moderation::automod_routing::Routing::Card,
            severe: false,
            auto_delete_link: false,
        })
    }
}

fn make_req(guild_id: &str, user_id: &str, content: &str) -> Request<proto::AnalyzeMessageRequest> {
    Request::new(proto::AnalyzeMessageRequest {
        guild_id: guild_id.into(),
        channel_id: "c1".into(),
        user_id: user_id.into(),
        username: "alice".into(),
        content: content.into(),
        flags: None,
        message_id: "m1".into(),
        timestamp: "2026-01-01T00:00:00Z".into(),
        context_messages: vec![],
    })
}

#[tokio::test]
async fn analyze_message_rejects_empty_guild_id() {
    let g = AutomodGrpc {
        uc: Arc::new(MockAnalyzeUc::default()),
    };
    let err = g
        .analyze_message(make_req("", "u", "hello"))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(err.message().contains("guild_id"));
}

#[tokio::test]
async fn analyze_message_rejects_too_long_guild_id() {
    let g = AutomodGrpc {
        uc: Arc::new(MockAnalyzeUc::default()),
    };
    let long = "1".repeat(21);
    let err = g
        .analyze_message(make_req(&long, "u", "hello"))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn analyze_message_rejects_empty_user_id() {
    let g = AutomodGrpc {
        uc: Arc::new(MockAnalyzeUc::default()),
    };
    let err = g
        .analyze_message(make_req("g", "", "hello"))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(err.message().contains("user_id"));
}

#[tokio::test]
async fn analyze_message_rejects_empty_content() {
    let g = AutomodGrpc {
        uc: Arc::new(MockAnalyzeUc::default()),
    };
    let err = g.analyze_message(make_req("g", "u", "")).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(err.message().contains("content"));
}

#[tokio::test]
async fn analyze_message_delegates_to_uc_and_returns_analysis() {
    let uc = Arc::new(MockAnalyzeUc::default());
    let g = AutomodGrpc { uc: uc.clone() };
    let resp = g
        .analyze_message(make_req("g1", "u1", "message content"))
        .await
        .unwrap();
    let inner = resp.into_inner();
    assert_eq!(inner.action, proto::Action::Warn as i32);
    assert_eq!(inner.reason, "spam");

    let calls = uc.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].guild_id, "g1".into());
    assert_eq!(calls[0].content, "message content");
}

#[tokio::test]
async fn analyze_message_maps_flags_from_proto() {
    let uc = Arc::new(MockAnalyzeUc::default());
    let g = AutomodGrpc { uc: uc.clone() };
    let req = Request::new(proto::AnalyzeMessageRequest {
        guild_id: "g".into(),
        channel_id: "c".into(),
        user_id: "u".into(),
        username: "a".into(),
        content: "text".into(),
        flags: Some(proto::DetectionFlags {
            spam: true,
            insult: false,
            link: true,
            phishing: false,
        }),
        message_id: "m".into(),
        timestamp: "".into(),
        context_messages: vec![],
    });
    let _ = g.analyze_message(req).await.unwrap();
    let calls = uc.calls.lock().unwrap();
    assert!(calls[0].flags.spam);
    assert!(!calls[0].flags.insult);
    assert!(calls[0].flags.link);
    assert!(!calls[0].flags.phishing);
}

#[tokio::test]
async fn analyze_message_maps_context_messages() {
    let uc = Arc::new(MockAnalyzeUc::default());
    let g = AutomodGrpc { uc: uc.clone() };
    let req = Request::new(proto::AnalyzeMessageRequest {
        guild_id: "g".into(),
        channel_id: "c".into(),
        user_id: "u".into(),
        username: "a".into(),
        content: "text".into(),
        flags: None,
        message_id: "m".into(),
        timestamp: "".into(),
        context_messages: vec![
            proto::ContextMessage {
                username: "prev1".into(),
                content: "a".into(),
            },
            proto::ContextMessage {
                username: "prev2".into(),
                content: "b".into(),
            },
        ],
    });
    let _ = g.analyze_message(req).await.unwrap();
    let calls = uc.calls.lock().unwrap();
    assert_eq!(calls[0].context_messages.len(), 2);
    assert_eq!(calls[0].context_messages[0].username, "prev1");
}

#[tokio::test]
async fn analyze_message_flags_none_defaults_all_false() {
    let uc = Arc::new(MockAnalyzeUc::default());
    let g = AutomodGrpc { uc: uc.clone() };
    let _ = g.analyze_message(make_req("g", "u", "hi")).await.unwrap();
    let calls = uc.calls.lock().unwrap();
    assert!(!calls[0].flags.spam);
    assert!(!calls[0].flags.insult);
    assert!(!calls[0].flags.link);
    assert!(!calls[0].flags.phishing);
}
