use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::ports::inbound::WelcomeConfigPatch;
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
    pub rejoin_title: String,
    pub rejoin_image_url: String,
    pub rejoin_footer_text: String,
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
            rejoin_title: c.rejoin_title, rejoin_image_url: c.rejoin_image_url,
            rejoin_footer_text: c.rejoin_footer_text,
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
    let config = state.welcome_config_uc.get(&guild_id).await?;
    Ok(Json(config.into()))
}

/// PUT /api/welcome/{guild_id}
pub async fn save_config(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<SaveWelcomeConfigDto>,
) -> Result<Json<WelcomeConfigDto>, ApiError> {
    let saved = state
        .welcome_config_uc
        .save_patch(&guild_id, dto_to_patch(dto))
        .await?;
    Ok(Json(saved.into()))
}

fn dto_to_patch(dto: SaveWelcomeConfigDto) -> WelcomeConfigPatch {
    WelcomeConfigPatch {
        welcome_enabled: dto.welcome_enabled,
        welcome_channel_id: dto.welcome_channel_id,
        welcome_message: dto.welcome_message,
        welcome_embed_color: dto.welcome_embed_color,
        welcome_dm_enabled: dto.welcome_dm_enabled,
        welcome_dm_message: dto.welcome_dm_message,
        welcome_title: None,
        welcome_image_url: None,
        welcome_footer_text: None,
        leave_enabled: dto.leave_enabled,
        leave_channel_id: dto.leave_channel_id,
        leave_message: dto.leave_message,
        leave_title: None,
        leave_image_url: None,
        leave_footer_text: None,
        rules_enabled: dto.rules_enabled,
        rules_channel_id: dto.rules_channel_id,
        rules_message: dto.rules_message,
        rules_role_id: dto.rules_role_id,
        rules_button_label: dto.rules_button_label,
        counter_enabled: dto.counter_enabled,
        counter_channel_id: dto.counter_channel_id,
        counter_format: dto.counter_format,
        anniversary_enabled: dto.anniversary_enabled,
        anniversary_channel_id: dto.anniversary_channel_id,
        anniversary_message: dto.anniversary_message,
        anniversary_title: None,
        anniversary_image_url: None,
        anniversary_footer_text: None,
        rejoin_message: dto.rejoin_message,
        rejoin_title: None,
        rejoin_image_url: None,
        rejoin_footer_text: None,
    }
}

#[cfg(test)]
#[path = "tests/welcome.rs"]
mod tests;
