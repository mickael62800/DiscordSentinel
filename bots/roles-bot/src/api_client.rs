use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sentinel_shared::api_client::BaseApiClient;

#[derive(Debug, Deserialize)]
pub struct RolePanelDetail {
    pub panel: RolePanel,
    pub entries: Vec<RolePanelEntry>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RolePanel {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: Option<String>,
    pub title: String,
    pub description: String,
    pub mode: String,
    pub max_roles: Option<i32>,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RolePanelEntry {
    pub id: String,
    pub role_id: String,
    pub role_name: String,
    pub emoji: Option<String>,
    pub label: String,
    pub style: String,
    pub position: i32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AutoRole {
    pub role_id: String,
    pub role_name: String,
    pub delay_secs: i32,
    pub enabled: bool,
}

pub struct ApiClient {
    pub base: Arc<BaseApiClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>) -> Self {
        Self { base }
    }

    #[allow(dead_code)]
    pub async fn get_panel_by_message(&self, message_id: &str) -> Result<Option<RolePanelDetail>, String> {
        let resp = self.base.auth(
            self.base.client().get(format!("{}/api/role-panels/by-message/{message_id}", self.base.base_url()))
        )
            .send().await.map_err(|e| format!("{e}"))?;
        if resp.status().as_u16() == 404 { return Ok(None); }
        if !resp.status().is_success() { return Err(format!("API error {}", resp.status())); }
        resp.json::<Option<RolePanelDetail>>().await.map_err(|e| format!("{e}"))
    }

    pub async fn get_auto_roles(&self, guild_id: &str) -> Result<Vec<AutoRole>, String> {
        let resp = self.base.auth(
            self.base.client().get(format!("{}/api/auto-roles/{guild_id}", self.base.base_url()))
        )
            .send().await.map_err(|e| format!("{e}"))?;
        if !resp.status().is_success() { return Ok(vec![]); }
        resp.json::<Vec<AutoRole>>().await.map_err(|e| format!("{e}"))
    }

    pub async fn set_message_id(&self, panel_id: &str, message_id: &str) -> Result<(), String> {
        #[derive(Serialize)]
        struct P { panel_id: String, message_id: String }
        let req = self.base.client().patch(format!("{}/api/role-panels/set-message", self.base.base_url()))
            .json(&P { panel_id: panel_id.into(), message_id: message_id.into() });
        self.base.auth(req).send().await.map_err(|e| format!("{e}"))?;
        Ok(())
    }

    pub async fn list_panels(&self, guild_id: &str) -> Result<Vec<RolePanel>, String> {
        let resp = self.base.auth(
            self.base.client().get(format!("{}/api/role-panels/{guild_id}", self.base.base_url()))
        )
            .send().await.map_err(|e| format!("{e}"))?;
        if !resp.status().is_success() { return Ok(vec![]); }
        resp.json::<Vec<RolePanel>>().await.map_err(|e| format!("{e}"))
    }

    pub async fn get_panel(&self, panel_id: &str) -> Result<Option<RolePanelDetail>, String> {
        let resp = self.base.auth(
            self.base.client().get(format!("{}/api/role-panels/detail/{panel_id}", self.base.base_url()))
        )
            .send().await.map_err(|e| format!("{e}"))?;
        if resp.status().as_u16() == 404 { return Ok(None); }
        resp.json::<RolePanelDetail>().await.map(Some).map_err(|e| format!("{e}"))
    }
}
