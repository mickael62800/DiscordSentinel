use async_trait::async_trait;

use crate::domain::errors::DomainError;

/// Config welcome brute (1 row par guild). Les defaults sont appliques
/// par le repository si la row n'existe pas.
#[derive(Debug, Clone)]
pub struct WelcomeConfigData {
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
    // Embed enrichi — bienvenue
    pub welcome_title: String,
    pub welcome_image_url: String,
    pub welcome_footer_text: String,
    // Embed enrichi — retour (rejoin)
    pub rejoin_title: String,
    pub rejoin_image_url: String,
    pub rejoin_footer_text: String,
    // Embed enrichi — depart
    pub leave_title: String,
    pub leave_image_url: String,
    pub leave_footer_text: String,
    // Embed enrichi — anniversaire
    pub anniversary_title: String,
    pub anniversary_image_url: String,
    pub anniversary_footer_text: String,
}

#[async_trait]
pub trait WelcomeConfigRepository: Send + Sync {
    async fn get_config(&self, guild_id: &str) -> Result<WelcomeConfigData, DomainError>;
    async fn save_config(&self, guild_id: &str, data: &WelcomeConfigData) -> Result<WelcomeConfigData, DomainError>;
}
