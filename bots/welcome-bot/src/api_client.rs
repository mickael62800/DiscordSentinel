use std::sync::Arc;

use serde::Deserialize;
use sentinel_shared::api_client::BaseApiClient;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct WelcomeConfig {
    pub guild_id: String,
    pub welcome_enabled: bool,
    pub welcome_channel_id: Option<String>,
    pub welcome_message: String,
    pub welcome_embed_color: String,
    pub welcome_dm_enabled: bool,
    pub welcome_dm_message: String,
    pub leave_enabled: bool,
    pub leave_channel_id: Option<String>,
    pub leave_message: String,
    pub rules_enabled: bool,
    pub rules_channel_id: Option<String>,
    pub rules_message: String,
    pub rules_role_id: Option<String>,
    pub rules_button_label: String,
    pub counter_enabled: bool,
    pub counter_channel_id: Option<String>,
    pub counter_format: String,
    pub anniversary_enabled: bool,
    pub anniversary_channel_id: Option<String>,
    pub anniversary_message: String,
    pub rejoin_message: String,
}

pub struct WelcomeApiClient {
    pub base: Arc<BaseApiClient>,
}

impl WelcomeApiClient {
    pub fn new(base: Arc<BaseApiClient>) -> Self {
        Self { base }
    }

    pub async fn get_config(&self, guild_id: &str) -> Result<WelcomeConfig, String> {
        self.base.get_json(&format!("/api/welcome/{guild_id}")).await
    }

    /// Verifie si un membre est deja connu (existe dans guild_members = ancien membre qui revient).
    pub async fn is_known_member(&self, guild_id: &str, user_id: &str) -> bool {
        #[derive(serde::Deserialize)]
        struct MemberCheck {
            #[allow(dead_code)]
            username: String,
        }
        self.base
            .get_json::<MemberCheck>(&format!("/api/members/{guild_id}/{user_id}"))
            .await
            .is_ok()
    }
}
