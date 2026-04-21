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

    /// Construit un IaConfig en normalisant/validant les entrees selon les
    /// invariants metier :
    /// - thresholds et dampening clampes dans [0.0, 1.0]
    /// - context_max_messages clampe dans [0, 10]
    /// - context_max_chars clampe dans [50, 500]
    /// - context_format : "natural" ou "tagged" ; toute autre valeur retombe
    ///   sur "natural"
    ///
    /// Regle metier pure. Les handlers HTTP/gRPC qui recoivent du DTO brut
    /// doivent passer par cette fonction au lieu d'appliquer les clamps
    /// localement.
    #[allow(clippy::too_many_arguments)]
    pub fn new_normalized(
        guild_id: String,
        text_enabled: bool,
        text_threshold: f64,
        vision_enabled: bool,
        vision_threshold: f64,
        context_dampening: f64,
        context_format: String,
        context_max_messages: i32,
        context_max_chars: i32,
    ) -> Self {
        let context_format = match context_format.as_str() {
            "natural" | "tagged" => context_format,
            _ => "natural".to_string(),
        };
        let now = Utc::now();
        Self {
            guild_id,
            text_enabled,
            text_threshold: text_threshold.clamp(0.0, 1.0),
            vision_enabled,
            vision_threshold: vision_threshold.clamp(0.0, 1.0),
            context_dampening: context_dampening.clamp(0.0, 1.0),
            context_format,
            context_max_messages: context_max_messages.clamp(0, 10),
            context_max_chars: context_max_chars.clamp(50, 500),
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
#[path = "tests/ia_config.rs"]
mod tests;
