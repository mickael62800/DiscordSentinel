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
        let path = format!("/api/role-panels/by-message/{message_id}");
        // 404 means no panel found — return None
        let resp = self.base.auth(
            self.base.client().get(format!("{}{}", self.base.base_url(), path))
        ).send().await.map_err(|e| format!("{e}"))?;
        if resp.status().as_u16() == 404 { return Ok(None); }
        if !resp.status().is_success() { return Err(format!("API error {}", resp.status())); }
        resp.json::<Option<RolePanelDetail>>().await.map_err(|e| format!("{e}"))
    }

    pub async fn get_auto_roles(&self, guild_id: &str) -> Result<Vec<AutoRole>, String> {
        self.base.get_json(&format!("/api/auto-roles/{guild_id}")).await
    }

    pub async fn set_message_id(&self, panel_id: &str, message_id: &str) -> Result<(), String> {
        #[derive(Serialize)]
        struct P { panel_id: String, message_id: String }
        self.base.patch_fire_and_forget(
            "/api/role-panels/set-message",
            &P { panel_id: panel_id.into(), message_id: message_id.into() },
        ).await;
        Ok(())
    }

    pub async fn list_panels(&self, guild_id: &str) -> Result<Vec<RolePanel>, String> {
        self.base.get_json(&format!("/api/role-panels/{guild_id}")).await
    }

    pub async fn get_panel(&self, panel_id: &str) -> Result<Option<RolePanelDetail>, String> {
        let path = format!("/api/role-panels/detail/{panel_id}");
        let resp = self.base.auth(
            self.base.client().get(format!("{}{}", self.base.base_url(), path))
        ).send().await.map_err(|e| format!("{e}"))?;
        if resp.status().as_u16() == 404 { return Ok(None); }
        resp.json::<RolePanelDetail>().await.map(Some).map_err(|e| format!("{e}"))
    }

    // ── Sponsorships (fire-and-forget) ──

    /// Persiste un parrainage.
    pub async fn create_sponsorship(&self, guild_id: &str, sponsor_id: &str, sponsored_id: &str) {
        self.base.post_fire_and_forget("/api/sponsorships", &serde_json::json!({
            "guild_id": guild_id,
            "sponsor_id": sponsor_id,
            "sponsored_id": sponsored_id,
        })).await;
    }

    // ── Temp Roles (fire-and-forget) ──

    /// Persiste un role temporaire.
    pub async fn create_temp_role(&self, guild_id: &str, user_id: &str, role_id: &str, expires_at: &str) {
        self.base.post_fire_and_forget("/api/temp-roles", &serde_json::json!({
            "guild_id": guild_id,
            "user_id": user_id,
            "role_id": role_id,
            "expires_at": expires_at,
        })).await;
    }

    /// Supprime un role temporaire expire.
    pub async fn delete_temp_role(&self, guild_id: &str, user_id: &str, role_id: &str) {
        // DELETE fire-and-forget — no body, no response needed
        let req = self.base.client().delete(format!(
            "{}/api/temp-roles/{}/{}/{}",
            self.base.base_url(), guild_id, user_id, role_id
        ));
        if let Err(e) = self.base.auth(req).send().await {
            tracing::warn!(error = %e, "Failed to delete temp role");
        }
    }
}
