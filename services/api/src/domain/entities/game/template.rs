//! Template de jeu (catalogue) : reference Docker image + schema config UX.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type d'un champ configurable cote UX (game-portal page).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFieldType {
    Text,
    Number,
    Enum,
    Boolean,
}

/// Definition d'un champ configurable par l'admin du jeu.
/// Sert au front pour generer dynamiquement le formulaire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: ConfigFieldType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
}

/// Protocole reseau du port jeu (TCP : Minecraft, Terraria... ; UDP :
/// Valheim, Factorio, Palworld...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortProtocol {
    Tcp,
    Udp,
}

impl PortProtocol {
    pub fn from_str(s: &str) -> Self {
        match s {
            "udp" => Self::Udp,
            _ => Self::Tcp,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
        }
    }
}

/// Template de jeu — entree du catalogue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameTemplate {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub image: String,
    pub category: Option<String>,
    pub icon: Option<String>,
    pub accent_color: Option<String>,
    pub container_port: u16,
    pub port_protocol: PortProtocol,
    pub default_memory_mb: i32,
    pub min_memory_mb: i32,
    pub max_memory_mb: i32,
    pub default_env: serde_json::Value,
    pub config_schema: Vec<ConfigField>,
    pub supports_rcon: bool,
    pub supports_mods: bool,
    pub idle_shutdown_days: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GameTemplate {
    /// Verifie qu'un override de memoire est dans les bornes du template.
    pub fn validate_memory(&self, requested_mb: i32) -> Result<(), String> {
        if requested_mb < self.min_memory_mb {
            return Err(format!(
                "memoire trop basse: {} Mo < min {} Mo",
                requested_mb, self.min_memory_mb
            ));
        }
        if requested_mb > self.max_memory_mb {
            return Err(format!(
                "memoire trop haute: {} Mo > max {} Mo",
                requested_mb, self.max_memory_mb
            ));
        }
        Ok(())
    }

    /// Cherche la definition d'un champ config par sa key.
    pub fn find_field(&self, key: &str) -> Option<&ConfigField> {
        self.config_schema.iter().find(|f| f.key == key)
    }
}
