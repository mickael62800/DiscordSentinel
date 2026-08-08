//! Tests d'integration HTTP pour les endpoints role_panels et auto_roles.

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
use sentinel_core::domain::entities::community::role_panel::AutoRole;
use sentinel_core::domain::entities::community::role_panel::RolePanel;
use sentinel_core::domain::entities::community::role_panel::RolePanelDetail;
use sentinel_core::domain::entities::community::role_panel::RolePanelEntry;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::inbound::community::manage_role_panels::CreateAutoRoleCommand;
use sentinel_core::ports::inbound::community::manage_role_panels::CreateRolePanelCommand;
use sentinel_core::ports::inbound::community::manage_role_panels::ManageRolePanelsUseCase;
use sentinel_core::ports::inbound::community::manage_role_panels::SetMessageIdCommand;
use test_helpers::build_test_state_role_panels;

#[derive(Default)]
struct MockRolePanelsUC {
    panels: Mutex<Vec<RolePanel>>,
    entries: Mutex<Vec<RolePanelEntry>>,
    auto_roles: Mutex<Vec<AutoRole>>,
}

impl MockRolePanelsUC {
    fn new() -> Self {
        Self::default()
    }
    fn with_panel(self, p: RolePanel) -> Self {
        self.panels.lock().unwrap().push(p);
        self
    }
}

fn sample_panel(guild_id: &str) -> RolePanel {
    let now = Utc::now();
    RolePanel {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        channel_id: "c1".into(),
        message_id: None,
        title: "Roles".into(),
        description: "Pick".into(),
        mode: "button".into(),
        max_roles: None,
        enabled: true,
        created_at: now,
        updated_at: now,
    }
}

#[async_trait]
impl ManageRolePanelsUseCase for MockRolePanelsUC {
    async fn create_panel(
        &self,
        cmd: CreateRolePanelCommand,
    ) -> Result<RolePanelDetail, DomainError> {
        let now = Utc::now();
        let panel = RolePanel {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            channel_id: cmd.channel_id,
            message_id: None,
            title: cmd.title,
            description: cmd.description,
            mode: cmd.mode,
            max_roles: cmd.max_roles,
            enabled: true,
            created_at: now,
            updated_at: now,
        };
        let entries: Vec<RolePanelEntry> = cmd
            .entries
            .into_iter()
            .map(|e| RolePanelEntry {
                id: Uuid::new_v4(),
                panel_id: panel.id,
                role_id: e.role_id,
                role_name: e.role_name,
                emoji: e.emoji,
                label: e.label,
                style: e.style,
                position: e.position,
            })
            .collect();
        self.panels.lock().unwrap().push(panel.clone());
        self.entries.lock().unwrap().extend(entries.clone());
        Ok(RolePanelDetail { panel, entries })
    }
    async fn get_panel(&self, panel_id: &str) -> Result<RolePanelDetail, DomainError> {
        let uuid = Uuid::parse_str(panel_id)
            .map_err(|_| DomainError::ValidationError("bad uuid".into()))?;
        let panels = self.panels.lock().unwrap();
        let panel = panels
            .iter()
            .find(|p| p.id == uuid)
            .cloned()
            .ok_or_else(|| DomainError::NotFound("panel".into()))?;
        let entries = self
            .entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.panel_id == uuid)
            .cloned()
            .collect();
        Ok(RolePanelDetail { panel, entries })
    }
    async fn get_panel_by_message(
        &self,
        message_id: &str,
    ) -> Result<Option<RolePanelDetail>, DomainError> {
        let panels = self.panels.lock().unwrap();
        let Some(panel) = panels
            .iter()
            .find(|p| p.message_id.as_deref() == Some(message_id))
            .cloned()
        else {
            return Ok(None);
        };
        Ok(Some(RolePanelDetail {
            panel,
            entries: vec![],
        }))
    }
    async fn list_panels(&self, guild_id: &str) -> Result<Vec<RolePanel>, DomainError> {
        Ok(self
            .panels
            .lock()
            .unwrap()
            .iter()
            .filter(|p| p.guild_id.as_str() == guild_id)
            .cloned()
            .collect())
    }
    async fn set_message_id(&self, cmd: SetMessageIdCommand) -> Result<(), DomainError> {
        let uuid = Uuid::parse_str(&cmd.panel_id)
            .map_err(|_| DomainError::ValidationError("bad uuid".into()))?;
        let mut panels = self.panels.lock().unwrap();
        for p in panels.iter_mut() {
            if p.id == uuid {
                p.message_id = Some(cmd.message_id.to_string());
            }
        }
        Ok(())
    }
    async fn delete_panel(&self, panel_id: &str) -> Result<(), DomainError> {
        let uuid = Uuid::parse_str(panel_id)
            .map_err(|_| DomainError::ValidationError("bad uuid".into()))?;
        self.panels.lock().unwrap().retain(|p| p.id != uuid);
        Ok(())
    }
    async fn list_auto_roles(&self, guild_id: &str) -> Result<Vec<AutoRole>, DomainError> {
        Ok(self
            .auto_roles
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.guild_id.as_str() == guild_id)
            .cloned()
            .collect())
    }
    async fn add_auto_role(&self, cmd: CreateAutoRoleCommand) -> Result<AutoRole, DomainError> {
        let ar = AutoRole {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            role_id: cmd.role_id,
            role_name: cmd.role_name,
            delay_secs: cmd.delay_secs,
            enabled: true,
        };
        self.auto_roles.lock().unwrap().push(ar.clone());
        Ok(ar)
    }
    async fn delete_auto_role(&self, guild_id: &str, role_id: &str) -> Result<(), DomainError> {
        self.auto_roles
            .lock()
            .unwrap()
            .retain(|a| !(a.guild_id.as_str() == guild_id && a.role_id.as_str() == role_id));
        Ok(())
    }
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
    )
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
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
    )
}

async fn delete(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let s = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (
        s,
        serde_json::from_slice(&b).unwrap_or(serde_json::Value::Null),
    )
}

fn build_app(uc: Arc<MockRolePanelsUC>) -> axum::Router {
    router::build_for_test(build_test_state_role_panels(uc))
}

// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_panel_returns_detail() {
    let app = build_app(Arc::new(MockRolePanelsUC::new()));
    let body = serde_json::json!({
        "guild_id": "111111111111111111", "channel_id": "c1", "title": "Roles",
        "entries": [
            {"role_id": "r1", "role_name": "Gamer", "label": "Gamer", "position": 0}
        ]
    });
    let (status, json) = post_json(app, "/api/role-panels", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["panel"]["title"], "Roles");
    assert_eq!(json["entries"].as_array().unwrap().len(), 1);
    assert_eq!(json["entries"][0]["role_name"], "Gamer");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_panels_scoped_to_guild() {
    let uc = MockRolePanelsUC::new()
        .with_panel(sample_panel("111111111111111111"))
        .with_panel(sample_panel("222222222222222222"));
    let app = build_app(Arc::new(uc));
    let (status, json) = get(app, "/api/role-panels/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_panel_by_uuid_returns_detail() {
    let panel = sample_panel("111111111111111111");
    let pid = panel.id;
    let app = build_app(Arc::new(MockRolePanelsUC::new().with_panel(panel)));
    let (status, json) = get(app, &format!("/api/role-panels/detail/{pid}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["panel"]["guild_id"], "111111111111111111");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_panel_not_found_returns_404() {
    let app = build_app(Arc::new(MockRolePanelsUC::new()));
    let pid = Uuid::new_v4();
    let (status, _) = get(app, &format!("/api/role-panels/detail/{pid}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_panel_success() {
    let panel = sample_panel("111111111111111111");
    let pid = panel.id;
    let uc = Arc::new(MockRolePanelsUC::new().with_panel(panel));
    let app = build_app(uc.clone());
    let (status, _) = delete(app, &format!("/api/role-panels/detail/{pid}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(uc.panels.lock().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_panel_by_message_none() {
    let app = build_app(Arc::new(MockRolePanelsUC::new()));
    let (status, json) = get(app, "/api/role-panels/by-message/666666666666666666").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json, serde_json::Value::Null);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_auto_roles_empty() {
    let app = build_app(Arc::new(MockRolePanelsUC::new()));
    let (status, json) = get(app, "/api/auto-roles/111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_auto_role_success() {
    let uc = Arc::new(MockRolePanelsUC::new());
    let app = build_app(uc.clone());
    let body = serde_json::json!({
        "guild_id": "111111111111111111", "role_id": "r1",
        "role_name": "Member", "delay_secs": 30
    });
    let (status, json) = post_json(app, "/api/auto-roles", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["role_id"], "r1");
    assert_eq!(json["delay_secs"], 30);
    assert_eq!(uc.auto_roles.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_auto_role_success() {
    let uc = Arc::new(MockRolePanelsUC::new());
    uc.auto_roles.lock().unwrap().push(AutoRole {
        id: Uuid::new_v4(),
        guild_id: "111111111111111111".into(),
        role_id: "r1".into(),
        role_name: "X".into(),
        delay_secs: 0,
        enabled: true,
    });
    let app = build_app(uc.clone());
    let (status, _) = delete(app, "/api/auto-roles/111111111111111111/r1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(uc.auto_roles.lock().unwrap().len(), 0);
}
