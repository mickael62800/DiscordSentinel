//! Override de configuration par instance (key/value).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameServerConfig {
    pub server_id: Uuid,
    pub config_key: String,
    pub config_value: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<String>,
}

/// Validation : key en SCREAMING_SNAKE_CASE (exigence DB).
pub fn validate_config_key(key: &str) -> Result<(), String> {
    if key.is_empty() || key.len() > 64 {
        return Err("config_key invalide : 1-64 caracteres".into());
    }
    let mut chars = key.chars();
    let first = chars.next().ok_or("config_key vide")?;
    if !first.is_ascii_uppercase() {
        return Err("config_key doit commencer par une lettre majuscule".into());
    }
    if !chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') {
        return Err(
            "config_key invalide : majuscules, chiffres et underscores uniquement".into(),
        );
    }
    Ok(())
}
