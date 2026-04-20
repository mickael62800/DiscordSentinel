use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IaConfig {
    pub guild_id: String,
    pub text_enabled: bool,
    pub text_threshold: f64,
    pub vision_enabled: bool,
    pub vision_threshold: f64,
    /// Facteur d'attenuation du score IA quand du contexte conversationnel est disponible (0.0 = aucun effet, 1.0 = score complet).
    pub context_dampening: f64,
    /// Format du contexte envoye au modele : "natural" (conversation brute) ou "tagged" (balises [message]/[context]).
    pub context_format: String,
    /// Nombre maximum de messages de contexte a recuperer.
    pub context_max_messages: i32,
    /// Nombre maximum de caracteres par message de contexte.
    pub context_max_chars: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl IaConfig {
    pub fn default_for_guild(guild_id: &str) -> Self {
        let now = Utc::now();
        Self {
            guild_id: guild_id.to_string(),
            text_enabled: true,
            text_threshold: 0.5,
            vision_enabled: true,
            vision_threshold: 0.5,
            context_dampening: 0.65,
            context_format: "natural".to_string(),
            context_max_messages: 3,
            context_max_chars: 200,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
#[path = "tests/ia_config.rs"]
mod tests;
