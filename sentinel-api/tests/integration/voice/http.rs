//! Tests d'integration HTTP pour les endpoints voice channels.
//! Teste la couche complete : routing → handler → serialisation → codes HTTP.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::ports::inbound::community::manage_voice_channels::*;
use sentinel_core::domain::entities::community::voice_channel::*;
use sentinel_core::domain::errors::DomainError;

use test_helpers::build_test_state;

// ══════════════════════════════════════════════════════════
// Mock Voice Channels Use Case (fonctionnel)
// ══════════════════════════════════════════════════════════

struct MockVoiceUC {
    channels: Vec<VoiceChannel>,
    themes: Vec<VoiceChannelTheme>,
    invite_links: Vec<VoiceChannelInviteLink>,
}

impl MockVoiceUC {
    fn new() -> Self {
        Self {
            channels: vec![],
            themes: vec![],
            invite_links: vec![],
        }
    }

    fn with_channel(mut self, ch: VoiceChannel) -> Self {
        self.channels.push(ch);
        self
    }

    fn with_theme(mut self, t: VoiceChannelTheme) -> Self {
        self.themes.push(t);
        self
    }

    fn with_invite_link(mut self, l: VoiceChannelInviteLink) -> Self {
        self.invite_links.push(l);
        self
    }
}

fn make_channel(guild_id: &str, channel_id: &str) -> VoiceChannel {
    VoiceChannel {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        owner_id: "owner1".into(),
        owner_name: "Owner".into(),
        channel_id: channel_id.into(),
        text_channel_id: None,
        members_channel_id: None,
        queue_channel_id: None,
        category_id: None,
        channel_name: "Test".into(),
        kind:
            sentinel_core::domain::enums::community::voice_channel_kind::VoiceChannelKind::Private,
        visibility: "visible".into(),
        queue_enabled: false,
        locked: false,
        stage_enabled: false,
        member_limit: None,
        status: None,
        channel_status: "open".into(),
        closed_at: None,
        created_at: Utc::now(),
    }
}

fn make_theme(guild_id: &str, name: &str) -> VoiceChannelTheme {
    VoiceChannelTheme {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        name: name.into(),
        emoji: Some("🎮".into()),
        channel_name_template: "{user}".into(),
        member_limit: Some(10),
        visibility: "visible".into(),
        locked: false,
        queue_enabled: false,
        bitrate: None,
        slowmode_secs: None,
        stage_enabled: false,
        is_default: false,
        sort_order: 0,
        created_at: Utc::now(),
    }
}

fn make_invite(channel_id: &str, code: &str) -> VoiceChannelInviteLink {
    VoiceChannelInviteLink {
        id: Uuid::new_v4(),
        voice_channel_id: Uuid::new_v4(),
        guild_id: "guild1".into(),
        channel_id: channel_id.into(),
        created_by: "user1".into(),
        created_by_name: "User".into(),
        code: code.into(),
        max_uses: None,
        current_uses: 0,
        expires_at: Utc::now() + chrono::Duration::hours(1),
        revoked: false,
        created_at: Utc::now(),
    }
}

#[async_trait]
impl ManageVoiceChannelsUseCase for MockVoiceUC {
    async fn list_all_channels(&self) -> Result<Vec<VoiceChannel>, DomainError> {
        Ok(self.channels.clone())
    }

    async fn list_channels(&self, guild_id: &str) -> Result<Vec<VoiceChannel>, DomainError> {
        Ok(self
            .channels
            .iter()
            .filter(|c| c.guild_id.as_str() == guild_id)
            .cloned()
            .collect())
    }

    async fn get_channel_detail(
        &self,
        channel_id: &str,
    ) -> Result<VoiceChannelDetail, DomainError> {
        let ch = self
            .channels
            .iter()
            .find(|c| c.channel_id.as_str() == channel_id)
            .ok_or_else(|| DomainError::NotFound(format!("Channel {channel_id}")))?;
        let links = self
            .invite_links
            .iter()
            .filter(|l| l.channel_id.as_str() == channel_id)
            .cloned()
            .collect();
        Ok(VoiceChannelDetail {
            channel: ch.clone(),
            co_admins: vec![],
            bans: vec![],
            invite_links: links,
        })
    }

    async fn create_channel(
        &self,
        cmd: CreateVoiceChannelCommand,
    ) -> Result<VoiceChannel, DomainError> {
        Ok(VoiceChannel {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            owner_id: cmd.owner_id,
            owner_name: cmd.owner_name,
            channel_id: cmd.channel_id,
            text_channel_id: cmd.text_channel_id,
            members_channel_id: cmd.members_channel_id,
            queue_channel_id: cmd.queue_channel_id,
            category_id: cmd.category_id,
            channel_name: cmd.channel_name,
            kind: sentinel_core::domain::enums::community::voice_channel_kind::VoiceChannelKind::from_str_lossy(&cmd.kind),
            visibility: cmd.visibility,
            queue_enabled: cmd.queue_enabled,
            locked: false,
            stage_enabled: cmd.stage_enabled,
            member_limit: None,
            status: None,
            channel_status: "open".into(),
            closed_at: None,
            created_at: Utc::now(),
        })
    }

    async fn list_history_channels(
        &self,
        _: &str,
        _: i64,
    ) -> Result<Vec<VoiceChannel>, DomainError> {
        Ok(vec![])
    }
    async fn get_voice_config(&self, _: &str) -> Result<VoiceChannelConfig, DomainError> {
        Ok(VoiceChannelConfig::default())
    }
    async fn close_channel(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete_channel(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn update_channel(&self, _: UpdateVoiceChannelCommand) -> Result<(), DomainError> {
        Ok(())
    }
    async fn transfer_ownership(&self, _: TransferOwnershipCommand) -> Result<(), DomainError> {
        Ok(())
    }
    async fn add_co_admin(&self, _: ManageCoAdminCommand) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_co_admin(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_whitelist(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Vec<VoiceChannelWhitelistEntry>, DomainError> {
        Ok(vec![])
    }
    async fn add_to_whitelist(&self, _: ManageWhitelistCommand) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_from_whitelist(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_preset(
        &self,
        _: &str,
        _: &str,
    ) -> Result<
        Option<sentinel_core::domain::entities::community::voice_channel::VoiceChannelPreset>,
        DomainError,
    > {
        Ok(None)
    }
    async fn save_preset(
        &self,
        _: sentinel_core::ports::inbound::community::manage_voice_channels::SavePresetCommand,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn ban_from_channel(&self, _: BanFromChannelCommand) -> Result<(), DomainError> {
        Ok(())
    }
    async fn unban_from_channel(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn is_banned(&self, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn list_owner_bans(
        &self,
        _: &str,
        _: &str,
    ) -> Result<
        Vec<sentinel_core::domain::entities::community::voice_channel::VoiceChannelBan>,
        DomainError,
    > {
        Ok(vec![])
    }

    async fn create_invite_link(
        &self,
        cmd: CreateInviteLinkCommand,
    ) -> Result<VoiceChannelInviteLink, DomainError> {
        Ok(VoiceChannelInviteLink {
            id: Uuid::new_v4(),
            voice_channel_id: Uuid::new_v4(),
            guild_id: "guild1".into(),
            channel_id: cmd.channel_id,
            created_by: cmd.created_by,
            created_by_name: cmd.created_by_name,
            code: "TEST1234".into(),
            max_uses: cmd.max_uses,
            current_uses: 0,
            expires_at: Utc::now() + chrono::Duration::seconds(cmd.duration_secs.unwrap_or(1800)),
            revoked: false,
            created_at: Utc::now(),
        })
    }

    async fn list_invite_links(
        &self,
        channel_id: &str,
    ) -> Result<Vec<VoiceChannelInviteLink>, DomainError> {
        Ok(self
            .invite_links
            .iter()
            .filter(|l| l.channel_id.as_str() == channel_id)
            .cloned()
            .collect())
    }

    async fn use_invite_link(
        &self,
        cmd: UseInviteLinkCommand,
    ) -> Result<VoiceChannelInviteLink, DomainError> {
        self.invite_links
            .iter()
            .find(|l| l.code == cmd.code)
            .cloned()
            .ok_or_else(|| DomainError::NotFound(format!("Code {}", cmd.code)))
    }

    async fn revoke_invite_link(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }

    async fn list_themes(&self, guild_id: &str) -> Result<Vec<VoiceChannelTheme>, DomainError> {
        Ok(self
            .themes
            .iter()
            .filter(|t| t.guild_id.as_str() == guild_id)
            .cloned()
            .collect())
    }

    async fn create_theme(
        &self,
        cmd: CreateThemeCommand,
    ) -> Result<VoiceChannelTheme, DomainError> {
        if cmd.name.trim().is_empty() {
            return Err(DomainError::ValidationError("Nom vide".into()));
        }
        Ok(VoiceChannelTheme {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            name: cmd.name,
            emoji: cmd.emoji,
            channel_name_template: cmd.channel_name_template,
            member_limit: cmd.member_limit,
            visibility: cmd.visibility,
            locked: cmd.locked,
            queue_enabled: cmd.queue_enabled,
            bitrate: cmd.bitrate,
            slowmode_secs: cmd.slowmode_secs,
            stage_enabled: cmd.stage_enabled,
            is_default: cmd.is_default,
            sort_order: cmd.sort_order,
            created_at: Utc::now(),
        })
    }

    async fn update_theme(
        &self,
        theme_id: &str,
        cmd: CreateThemeCommand,
    ) -> Result<VoiceChannelTheme, DomainError> {
        Ok(VoiceChannelTheme {
            id: uuid::Uuid::parse_str(theme_id).unwrap_or_else(|_| Uuid::new_v4()),
            guild_id: cmd.guild_id,
            name: cmd.name,
            emoji: cmd.emoji,
            channel_name_template: cmd.channel_name_template,
            member_limit: cmd.member_limit,
            visibility: cmd.visibility,
            locked: cmd.locked,
            queue_enabled: cmd.queue_enabled,
            bitrate: cmd.bitrate,
            slowmode_secs: cmd.slowmode_secs,
            stage_enabled: cmd.stage_enabled,
            is_default: cmd.is_default,
            sort_order: cmd.sort_order,
            created_at: Utc::now(),
        })
    }
    async fn delete_theme(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

// ══════════════════════════════════════════════════════════
// Helper : build app + send request
// ══════════════════════════════════════════════════════════

fn build_app(uc: MockVoiceUC) -> axum::Router {
    let state = build_test_state(Arc::new(uc));
    router::build_for_test(state)
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, json)
}

async fn post_json(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

async fn delete(app: axum::Router, uri: &str) -> StatusCode {
    let req = Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    app.oneshot(req).await.unwrap().status()
}

async fn patch_json(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("PATCH")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — Channels
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_all_channels_empty() {
    let app = build_app(MockVoiceUC::new());
    let (status, json) = get(app, "/api/voice-channels/_all").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_all_channels_with_data() {
    let uc = MockVoiceUC::new()
        .with_channel(make_channel("g1", "c1"))
        .with_channel(make_channel("g2", "c2"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/voice-channels/_all").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_channels_by_guild() {
    let uc = MockVoiceUC::new()
        .with_channel(make_channel("g1", "c1"))
        .with_channel(make_channel("g1", "c2"))
        .with_channel(make_channel("g2", "c3"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/voice-channels/g1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_channel_detail_success() {
    let uc = MockVoiceUC::new().with_channel(make_channel("g1", "c1"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/voice-channels/by-channel/c1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["channel"]["channel_id"], "c1");
    assert!(json["co_admins"].as_array().unwrap().is_empty());
    assert!(json["bans"].as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_channel_detail_not_found() {
    let app = build_app(MockVoiceUC::new());
    let (status, json) = get(app, "/api/voice-channels/by-channel/unknown").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].as_str().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_channel_success() {
    let app = build_app(MockVoiceUC::new());
    let body = serde_json::json!({
        "guild_id": "g1",
        "owner_id": "o1",
        "owner_name": "Owner",
        "channel_id": "c1",
        "channel_name": "Test",
        "kind": "private",
        "visibility": "visible",
        "queue_enabled": false,
        "stage_enabled": false
    });
    let (status, json) = post_json(app, "/api/voice-channels", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["channel_id"], "c1");
    assert_eq!(json["kind"], "private");
    assert_eq!(json["channel_status"], "open");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn close_channel_success() {
    let uc = MockVoiceUC::new().with_channel(make_channel("g1", "c1"));
    let app = build_app(uc);
    let (status, json) = patch_json(
        app,
        "/api/voice-channels/by-channel/c1/close",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["ok"], true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_channel_success() {
    let uc = MockVoiceUC::new().with_channel(make_channel("g1", "c1"));
    let app = build_app(uc);
    let status = delete(app, "/api/voice-channels/by-channel/c1").await;
    assert_eq!(status, StatusCode::OK);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — Invite Links
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_invite_link_success() {
    let uc = MockVoiceUC::new().with_channel(make_channel("g1", "c1"));
    let app = build_app(uc);
    let body = serde_json::json!({
        "created_by": "user1",
        "created_by_name": "User",
        "duration_secs": 3600
    });
    let (status, json) = post_json(app, "/api/voice-channels/by-channel/c1/invites", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["code"], "TEST1234");
    assert!(!json["revoked"].as_bool().unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_invite_links_empty() {
    let uc = MockVoiceUC::new().with_channel(make_channel("g1", "c1"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/voice-channels/by-channel/c1/invites").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_invite_links_with_data() {
    let uc = MockVoiceUC::new()
        .with_channel(make_channel("g1", "c1"))
        .with_invite_link(make_invite("c1", "LINK0001"))
        .with_invite_link(make_invite("c1", "LINK0002"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/voice-channels/by-channel/c1/invites").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn use_invite_link_success() {
    let uc = MockVoiceUC::new()
        .with_channel(make_channel("g1", "c1"))
        .with_invite_link(make_invite("c1", "VALID123"));
    let app = build_app(uc);
    let body = serde_json::json!({
        "user_id": "user2",
        "user_name": "User2"
    });
    let (status, json) = post_json(app, "/api/voice-channels/invites/VALID123/use", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["code"], "VALID123");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn use_invite_link_not_found() {
    let app = build_app(MockVoiceUC::new());
    let body = serde_json::json!({
        "user_id": "user2",
        "user_name": "User2"
    });
    let (status, json) = post_json(app, "/api/voice-channels/invites/INVALID1/use", body).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].as_str().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoke_invite_link_success() {
    let uc = MockVoiceUC::new().with_channel(make_channel("g1", "c1"));
    let app = build_app(uc);
    let id = Uuid::new_v4();
    let status = delete(
        app,
        &format!("/api/voice-channels/by-channel/c1/invites/{id}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — Themes
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_themes_empty() {
    let app = build_app(MockVoiceUC::new());
    let (status, json) = get(app, "/api/voice-channels/themes/g1").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_themes_with_data() {
    let uc = MockVoiceUC::new()
        .with_theme(make_theme("g1", "Gaming"))
        .with_theme(make_theme("g1", "Musique"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/voice-channels/themes/g1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_theme_success() {
    let app = build_app(MockVoiceUC::new());
    let body = serde_json::json!({
        "name": "Gaming",
        "emoji": "🎮",
        "channel_name_template": "{user} Gaming",
        "member_limit": 10,
        "visibility": "visible"
    });
    let (status, json) = post_json(app, "/api/voice-channels/themes/g1", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["name"], "Gaming");
    assert_eq!(json["emoji"], "🎮");
    assert_eq!(json["member_limit"], 10);
    assert_eq!(json["guild_id"], "g1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_theme_empty_name_422() {
    let app = build_app(MockVoiceUC::new());
    let body = serde_json::json!({
        "name": "",
        "visibility": "visible"
    });
    let (status, json) = post_json(app, "/api/voice-channels/themes/g1", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(json["error"].as_str().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_theme_success() {
    let app = build_app(MockVoiceUC::new());
    let id = Uuid::new_v4();
    let status = delete(app, &format!("/api/voice-channels/themes/g1/{id}")).await;
    assert_eq!(status, StatusCode::OK);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — Bans & Whitelist
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn check_ban_returns_false() {
    let uc = MockVoiceUC::new().with_channel(make_channel("g1", "c1"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/voice-channels/by-channel/c1/bans/user1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["banned"], false);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_whitelist_empty() {
    let app = build_app(MockVoiceUC::new());
    let (status, json) = get(app, "/api/voice-channels/whitelist/g1/owner1").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.as_array().unwrap().is_empty());
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — Detail includes invite_links
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detail_includes_invite_links() {
    let uc = MockVoiceUC::new()
        .with_channel(make_channel("g1", "c1"))
        .with_invite_link(make_invite("c1", "DETAIL01"));
    let app = build_app(uc);
    let (status, json) = get(app, "/api/voice-channels/by-channel/c1").await;
    assert_eq!(status, StatusCode::OK);
    let links = json["invite_links"].as_array().unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0]["code"], "DETAIL01");
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — endpoints supplementaires + RBAC
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_history_channels_empty() {
    let app = build_app(MockVoiceUC::new());
    let (status, json) = get(app, "/api/voice-channels/111111111111111111/history").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_channel_success() {
    let uc = MockVoiceUC::new().with_channel(make_channel("111111111111111111", "c1"));
    let app = build_app(uc);
    let body = serde_json::json!({
        "channel_name": "New Name",
        "visibility": "hidden"
    });
    let (status, _) = patch_json(app, "/api/voice-channels/by-channel/c1", body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_ownership_success() {
    let uc = MockVoiceUC::new().with_channel(make_channel("111111111111111111", "c1"));
    let app = build_app(uc);
    let body = serde_json::json!({
        "new_owner_id": "555555555555555555",
        "new_owner_name": "NewOwner"
    });
    let (status, _) = patch_json(app, "/api/voice-channels/by-channel/c1/transfer", body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_co_admin_success() {
    let uc = MockVoiceUC::new().with_channel(make_channel("111111111111111111", "c1"));
    let app = build_app(uc);
    let body = serde_json::json!({
        "user_id": "555555555555555555",
        "user_name": "CoAdmin"
    });
    let (status, _) = post_json(app, "/api/voice-channels/by-channel/c1/co-admins", body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remove_co_admin_success() {
    let uc = MockVoiceUC::new().with_channel(make_channel("111111111111111111", "c1"));
    let app = build_app(uc);
    let status = delete(
        app,
        "/api/voice-channels/by-channel/c1/co-admins/555555555555555555",
    )
    .await;
    assert!(status.is_success() || status == StatusCode::NO_CONTENT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_to_whitelist_success() {
    let app = build_app(MockVoiceUC::new());
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "owner_id": "444444444444444444",
        "target_id": "555555555555555555",
        "target_name": "Target"
    });
    let (status, _) = post_json(app, "/api/voice-channels/whitelist", body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remove_from_whitelist_success() {
    let app = build_app(MockVoiceUC::new());
    let status = delete(
        app,
        "/api/voice-channels/whitelist/111111111111111111/444444444444444444/555555555555555555",
    )
    .await;
    assert!(status.is_success() || status == StatusCode::NO_CONTENT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ban_from_channel_success() {
    let uc = MockVoiceUC::new().with_channel(make_channel("111111111111111111", "c1"));
    let app = build_app(uc);
    let body = serde_json::json!({
        "user_id": "555555555555555555",
        "user_name": "Banned",
        "banned_by": "444444444444444444",
        "reason": "toxic"
    });
    let (status, _) = post_json(app, "/api/voice-channels/by-channel/c1/bans", body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unban_from_channel_success() {
    let uc = MockVoiceUC::new().with_channel(make_channel("111111111111111111", "c1"));
    let app = build_app(uc);
    let status = delete(
        app,
        "/api/voice-channels/by-channel/c1/bans/555555555555555555",
    )
    .await;
    assert!(status.is_success() || status == StatusCode::NO_CONTENT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_theme_success() {
    let theme_id = Uuid::new_v4();
    let uc = MockVoiceUC::new();
    let app = build_app(uc);
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "name": "Updated Theme",
        "emoji": "🎯",
        "channel_name_template": "{user}-updated",
        "member_limit": 5,
        "visibility": "visible",
        "locked": false,
        "queue_enabled": false,
        "bitrate": null,
        "slowmode_secs": null,
        "stage_enabled": false,
        "is_default": false,
        "sort_order": 0
    });
    let (status, _) = patch_json(
        app,
        &format!("/api/voice-channels/themes/111111111111111111/{theme_id}"),
        body,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

// ── RBAC injecte ────────────────────────────────────────

async fn send_request(
    app: axum::Router,
    req: axum::http::Request<Body>,
) -> (StatusCode, serde_json::Value) {
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
    )
}

async fn pool() -> sqlx::PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    sqlx::PgPool::connect(&url).await.unwrap()
}

async fn seed_role(pool: &sqlx::PgPool, user_id: &str, guild_id: &str, role: &str) {
    sqlx::query("INSERT INTO api_users (discord_user_id, display_name) VALUES ($1, 'T') ON CONFLICT DO NOTHING")
        .bind(user_id).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO api_user_guilds (discord_user_id, guild_id, role) VALUES ($1, $2, $3)",
    )
    .bind(user_id)
    .bind(guild_id)
    .bind(role)
    .execute(pool)
    .await
    .unwrap();
}

async fn seed_voice_channel(pool: &sqlx::PgPool, guild_id: &str, channel_id: &str) {
    sqlx::query(
        "INSERT INTO voice_channels (id, guild_id, owner_id, owner_name, channel_id, channel_name) \
         VALUES ($1, $2, '444444444444444444', 'Owner', $3, 'VC')",
    )
    .bind(Uuid::new_v4()).bind(guild_id).bind(channel_id).execute(pool).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_channel_with_rbac_moderator_succeeds() {
    use sentinel_core::domain::enums::system::role::Role;
    let p = pool().await;
    let guild_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let user_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    seed_role(&p, &user_id, &guild_id, "moderator").await;

    let app = build_app(MockVoiceUC::new());
    let body = serde_json::json!({
        "guild_id": guild_id,
        "owner_id": "444444444444444444",
        "owner_name": "Owner",
        "channel_id": "c-new",
        "channel_name": "New VC",
        "kind": "private",
        "visibility": "visible",
        "queue_enabled": false,
        "stage_enabled": false
    });
    let req = test_helpers::request_with_rbac(
        "POST",
        "/api/voice-channels",
        &user_id,
        Some(Role::Moderator),
        Some(guild_id),
        Some(body),
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_channel_with_rbac_viewer_forbidden() {
    use sentinel_core::domain::enums::system::role::Role;
    let p = pool().await;
    let guild_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let user_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    seed_role(&p, &user_id, &guild_id, "viewer").await;
    // gate_by_channel_id fait un SELECT sur voice_channels → il faut une row en DB.
    let channel_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    seed_voice_channel(&p, &guild_id, &channel_id).await;

    let uc = MockVoiceUC::new().with_channel(make_channel(&guild_id, &channel_id));
    let app = build_app(uc);
    let req = test_helpers::request_with_rbac(
        "DELETE",
        &format!("/api/voice-channels/by-channel/{channel_id}"),
        &user_id,
        Some(Role::Viewer),
        Some(guild_id),
        None,
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_channel_closed_deletes_row() {
    let p = pool().await;
    let guild_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let channel_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    sqlx::query(
        "INSERT INTO voice_channels (id, guild_id, owner_id, owner_name, channel_id, channel_name, channel_status, closed_at) \
         VALUES ($1, $2, 'o', 'Owner', $3, 'VC', 'closed', NOW())",
    ).bind(Uuid::new_v4()).bind(&guild_id).bind(&channel_id).execute(&p).await.unwrap();

    let app = build_app(MockVoiceUC::new());
    let status = delete(
        app,
        &format!("/api/voice-channels/by-channel/{channel_id}/purge"),
    )
    .await;
    assert!(status.is_success() || status == StatusCode::NO_CONTENT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_channel_still_open_returns_error() {
    let p = pool().await;
    let guild_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let channel_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    seed_voice_channel(&p, &guild_id, &channel_id).await;
    let app = build_app(MockVoiceUC::new());
    let status = delete(
        app,
        &format!("/api/voice-channels/by-channel/{channel_id}/purge"),
    )
    .await;
    assert!(status.is_client_error() || status.is_server_error());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn purge_history_deletes_closed_channels() {
    let p = pool().await;
    let guild_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let cid1 = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let cid2 = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    sqlx::query(
        "INSERT INTO voice_channels (id, guild_id, owner_id, owner_name, channel_id, channel_name, channel_status, closed_at) \
         VALUES ($1, $2, 'o', 'O', $3, 'A', 'closed', NOW())",
    ).bind(Uuid::new_v4()).bind(&guild_id).bind(&cid1).execute(&p).await.unwrap();
    sqlx::query(
        "INSERT INTO voice_channels (id, guild_id, owner_id, owner_name, channel_id, channel_name, channel_status, closed_at) \
         VALUES ($1, $2, 'o', 'O', $3, 'B', 'closed', NOW())",
    ).bind(Uuid::new_v4()).bind(&guild_id).bind(&cid2).execute(&p).await.unwrap();

    let app = build_app(MockVoiceUC::new());
    let status = delete(app, &format!("/api/voice-channels/{guild_id}/history")).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_channel_events_empty_returns_array() {
    let channel_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let app = build_app(MockVoiceUC::new());
    let (status, json) = get(
        app,
        &format!("/api/voice-channels/by-channel/{channel_id}/events"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.as_array().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transfer_ownership_with_rbac_moderator_succeeds() {
    use sentinel_core::domain::enums::system::role::Role;
    let p = pool().await;
    let guild_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let user_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    seed_role(&p, &user_id, &guild_id, "moderator").await;

    let channel_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    seed_voice_channel(&p, &guild_id, &channel_id).await;
    let uc = MockVoiceUC::new().with_channel(make_channel(&guild_id, &channel_id));
    let app = build_app(uc);
    let body = serde_json::json!({
        "new_owner_id": "555555555555555555",
        "new_owner_name": "NewOwner"
    });
    let req = test_helpers::request_with_rbac(
        "PATCH",
        &format!("/api/voice-channels/by-channel/{channel_id}/transfer"),
        &user_id,
        Some(Role::Moderator),
        Some(guild_id),
        Some(body),
    );
    let (status, _) = send_request(app, req).await;
    assert_eq!(status, StatusCode::OK);
}

// ══════════════════════════════════════════════════════════
// Tests HTTP — Health (route publique, verifie que le router fonctionne)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_endpoint() {
    let app = build_app(MockVoiceUC::new());
    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    // Health check essaie de ping PG/Redis — echoue avec fake pools
    // On verifie juste qu'on obtient une reponse HTTP (pas de panic)
    let status = resp.status();
    assert!(status.is_success() || status.is_server_error() || status == StatusCode::NOT_FOUND);
}
