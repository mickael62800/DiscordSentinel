use serde::{Deserialize, Serialize};

use crate::domain::entities::IaConfig;

#[derive(Debug, Serialize)]
pub struct IaConfigDto {
    pub guild_id: String,
    pub text_enabled: bool,
    pub text_threshold: f64,
    pub vision_enabled: bool,
    pub vision_threshold: f64,
    pub context_dampening: f64,
    pub context_format: String,
    pub context_max_messages: i32,
    pub context_max_chars: i32,
}

#[derive(Debug, Deserialize)]
pub struct SaveIaConfigDto {
    pub text_enabled: bool,
    pub text_threshold: f64,
    pub vision_enabled: bool,
    pub vision_threshold: f64,
    pub context_dampening: f64,
    pub context_format: String,
    pub context_max_messages: i32,
    pub context_max_chars: i32,
}

impl From<IaConfig> for IaConfigDto {
    fn from(c: IaConfig) -> Self {
        Self {
            guild_id: c.guild_id,
            text_enabled: c.text_enabled,
            text_threshold: c.text_threshold,
            vision_enabled: c.vision_enabled,
            vision_threshold: c.vision_threshold,
            context_dampening: c.context_dampening,
            context_format: c.context_format,
            context_max_messages: c.context_max_messages,
            context_max_chars: c.context_max_chars,
        }
    }
}
