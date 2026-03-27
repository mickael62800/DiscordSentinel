use serde::{Deserialize, Serialize};

use crate::domain::entities::IaConfig;

#[derive(Debug, Serialize)]
pub struct IaConfigDto {
    pub guild_id: String,
    pub text_enabled: bool,
    pub text_threshold: f64,
    pub vision_enabled: bool,
    pub vision_threshold: f64,
}

#[derive(Debug, Deserialize)]
pub struct SaveIaConfigDto {
    pub text_enabled: bool,
    pub text_threshold: f64,
    pub vision_enabled: bool,
    pub vision_threshold: f64,
}

impl From<IaConfig> for IaConfigDto {
    fn from(c: IaConfig) -> Self {
        Self {
            guild_id: c.guild_id,
            text_enabled: c.text_enabled,
            text_threshold: c.text_threshold,
            vision_enabled: c.vision_enabled,
            vision_threshold: c.vision_threshold,
        }
    }
}
