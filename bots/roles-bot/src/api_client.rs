use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Deserialize)]
pub struct RolePanelDetail {
    pub panel: RolePanel,
    pub entries: Vec<RolePanelEntry>,
}

#[derive(Debug, Deserialize)]
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
pub struct AutoRole {
    pub role_id: String,
    pub role_name: String,
    pub delay_secs: i32,
    pub enabled: bool,
}

pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url: config.api_base_url.clone(),
            api_key: config.api_key.clone(),
        }
    }

    fn auth(&self, req: RequestBuilder) -> RequestBuilder {
        if self.api_key.is_empty() { req } else { req.bearer_auth(&self.api_key) }
    }

    pub async fn heartbeat(&self, bot_name: &str) {
        #[derive(Serialize)]
        struct P { bot_name: String }
        let req = self.client.post(format!("{}/api/bots/heartbeat", self.base_url))
            .json(&P { bot_name: bot_name.to_string() });
        let _ = self.auth(req).send().await;
    }

    pub async fn register_guild(&self, guild_id: &str, name: &str, member_count: i32) -> Result<(), String> {
        #[derive(Serialize)]
        struct P { guild_id: String, name: String, member_count: Option<i32> }
        let req = self.client.post(format!("{}/api/guilds/register", self.base_url))
            .json(&P { guild_id: guild_id.into(), name: name.into(), member_count: Some(member_count) });
        self.auth(req).send().await.map_err(|e| format!("{e}"))?;
        Ok(())
    }

    pub async fn get_panel_by_message(&self, message_id: &str) -> Result<Option<RolePanelDetail>, String> {
        let resp = self.auth(self.client.get(format!("{}/api/role-panels/by-message/{message_id}", self.base_url)))
            .send().await.map_err(|e| format!("{e}"))?;
        if resp.status().as_u16() == 404 { return Ok(None); }
        if !resp.status().is_success() { return Err(format!("API error {}", resp.status())); }
        resp.json::<Option<RolePanelDetail>>().await.map_err(|e| format!("{e}"))
    }

    pub async fn get_auto_roles(&self, guild_id: &str) -> Result<Vec<AutoRole>, String> {
        let resp = self.auth(self.client.get(format!("{}/api/auto-roles/{guild_id}", self.base_url)))
            .send().await.map_err(|e| format!("{e}"))?;
        if !resp.status().is_success() { return Ok(vec![]); }
        resp.json::<Vec<AutoRole>>().await.map_err(|e| format!("{e}"))
    }

    pub async fn set_message_id(&self, panel_id: &str, message_id: &str) -> Result<(), String> {
        #[derive(Serialize)]
        struct P { panel_id: String, message_id: String }
        let req = self.client.patch(format!("{}/api/role-panels/set-message", self.base_url))
            .json(&P { panel_id: panel_id.into(), message_id: message_id.into() });
        self.auth(req).send().await.map_err(|e| format!("{e}"))?;
        Ok(())
    }

    pub async fn list_panels(&self, guild_id: &str) -> Result<Vec<RolePanel>, String> {
        let resp = self.auth(self.client.get(format!("{}/api/role-panels/{guild_id}", self.base_url)))
            .send().await.map_err(|e| format!("{e}"))?;
        if !resp.status().is_success() { return Ok(vec![]); }
        resp.json::<Vec<RolePanel>>().await.map_err(|e| format!("{e}"))
    }

    pub async fn get_panel(&self, panel_id: &str) -> Result<Option<RolePanelDetail>, String> {
        let resp = self.auth(self.client.get(format!("{}/api/role-panels/detail/{panel_id}", self.base_url)))
            .send().await.map_err(|e| format!("{e}"))?;
        if resp.status().as_u16() == 404 { return Ok(None); }
        resp.json::<RolePanelDetail>().await.map(Some).map_err(|e| format!("{e}"))
    }
}
