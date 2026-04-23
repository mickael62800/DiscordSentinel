use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::ports::outbound::WelcomeConfigData;

#[derive(Debug, Serialize)]
pub struct WelcomeConfigDto {
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
    pub welcome_title: String,
    pub welcome_image_url: String,
    pub welcome_footer_text: String,
    pub leave_title: String,
    pub leave_image_url: String,
    pub leave_footer_text: String,
    pub anniversary_title: String,
    pub anniversary_image_url: String,
    pub anniversary_footer_text: String,
}

impl From<WelcomeConfigData> for WelcomeConfigDto {
    fn from(c: WelcomeConfigData) -> Self {
        Self {
            guild_id: c.guild_id, welcome_enabled: c.welcome_enabled,
            welcome_channel_id: c.welcome_channel_id, welcome_message: c.welcome_message,
            welcome_embed_color: c.welcome_embed_color, welcome_dm_enabled: c.welcome_dm_enabled,
            welcome_dm_message: c.welcome_dm_message, leave_enabled: c.leave_enabled,
            leave_channel_id: c.leave_channel_id, leave_message: c.leave_message,
            rules_enabled: c.rules_enabled, rules_channel_id: c.rules_channel_id,
            rules_message: c.rules_message, rules_role_id: c.rules_role_id,
            rules_button_label: c.rules_button_label, counter_enabled: c.counter_enabled,
            counter_channel_id: c.counter_channel_id, counter_format: c.counter_format,
            anniversary_enabled: c.anniversary_enabled, anniversary_channel_id: c.anniversary_channel_id,
            anniversary_message: c.anniversary_message, rejoin_message: c.rejoin_message,
            welcome_title: c.welcome_title, welcome_image_url: c.welcome_image_url,
            welcome_footer_text: c.welcome_footer_text,
            leave_title: c.leave_title, leave_image_url: c.leave_image_url,
            leave_footer_text: c.leave_footer_text,
            anniversary_title: c.anniversary_title, anniversary_image_url: c.anniversary_image_url,
            anniversary_footer_text: c.anniversary_footer_text,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SaveWelcomeConfigDto {
    pub welcome_enabled: Option<bool>,
    pub welcome_channel_id: Option<String>,
    pub welcome_message: Option<String>,
    pub welcome_embed_color: Option<String>,
    pub welcome_dm_enabled: Option<bool>,
    pub welcome_dm_message: Option<String>,
    pub leave_enabled: Option<bool>,
    pub leave_channel_id: Option<String>,
    pub leave_message: Option<String>,
    pub rules_enabled: Option<bool>,
    pub rules_channel_id: Option<String>,
    pub rules_message: Option<String>,
    pub rules_role_id: Option<String>,
    pub rules_button_label: Option<String>,
    pub counter_enabled: Option<bool>,
    pub counter_channel_id: Option<String>,
    pub counter_format: Option<String>,
    pub anniversary_enabled: Option<bool>,
    pub anniversary_channel_id: Option<String>,
    pub anniversary_message: Option<String>,
    pub rejoin_message: Option<String>,
}

/// GET /api/welcome/{guild_id}
pub async fn get_config(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<WelcomeConfigDto>, ApiError> {
    let config = state.welcome_config_repo.get_config(&guild_id).await?;
    Ok(Json(config.into()))
}

/// PUT /api/welcome/{guild_id}
pub async fn save_config(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<SaveWelcomeConfigDto>,
) -> Result<Json<WelcomeConfigDto>, ApiError> {
    // Merge : lire la config actuelle puis appliquer les champs presents.
    let mut current = state.welcome_config_repo.get_config(&guild_id).await?;
    if let Some(v) = dto.welcome_enabled { current.welcome_enabled = v; }
    if let Some(v) = dto.welcome_channel_id { current.welcome_channel_id = Some(v); }
    if let Some(v) = dto.welcome_message { current.welcome_message = v; }
    if let Some(v) = dto.welcome_embed_color { current.welcome_embed_color = v; }
    if let Some(v) = dto.welcome_dm_enabled { current.welcome_dm_enabled = v; }
    if let Some(v) = dto.welcome_dm_message { current.welcome_dm_message = v; }
    if let Some(v) = dto.leave_enabled { current.leave_enabled = v; }
    if let Some(v) = dto.leave_channel_id { current.leave_channel_id = Some(v); }
    if let Some(v) = dto.leave_message { current.leave_message = v; }
    if let Some(v) = dto.rules_enabled { current.rules_enabled = v; }
    if let Some(v) = dto.rules_channel_id { current.rules_channel_id = Some(v); }
    if let Some(v) = dto.rules_message { current.rules_message = v; }
    if let Some(v) = dto.rules_role_id { current.rules_role_id = Some(v); }
    if let Some(v) = dto.rules_button_label { current.rules_button_label = v; }
    if let Some(v) = dto.counter_enabled { current.counter_enabled = v; }
    if let Some(v) = dto.counter_channel_id { current.counter_channel_id = Some(v); }
    if let Some(v) = dto.counter_format { current.counter_format = v; }
    if let Some(v) = dto.anniversary_enabled { current.anniversary_enabled = v; }
    if let Some(v) = dto.anniversary_channel_id { current.anniversary_channel_id = Some(v); }
    if let Some(v) = dto.anniversary_message { current.anniversary_message = v; }
    if let Some(v) = dto.rejoin_message { current.rejoin_message = v; }

    let saved = state.welcome_config_repo.save_config(&guild_id, &current).await?;
    Ok(Json(saved.into()))
}

#[cfg(test)]
#[path = "tests/welcome.rs"]
mod tests;
