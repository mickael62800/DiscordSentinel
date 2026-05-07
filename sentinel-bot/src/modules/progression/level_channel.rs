use std::collections::HashMap;

use crate::shared::api_client::BaseApiClient;

/// Resout le canal ou envoyer les annonces de level-up.
/// Retourne le channel_id configure si present et valide, sinon None (= utiliser le fallback).
pub fn resolve_level_up_channel(guild_config: &HashMap<String, String>) -> Option<u64> {
    let raw = BaseApiClient::config_or(guild_config, "level_up_channel_id", "");
    if raw.is_empty() {
        return None;
    }
    raw.parse::<u64>().ok().filter(|&id| id > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_none_when_not_configured() {
        let config = HashMap::new();
        assert_eq!(resolve_level_up_channel(&config), None);
    }

    #[test]
    fn returns_none_when_empty_string() {
        let mut config = HashMap::new();
        config.insert("level_up_channel_id".into(), "".into());
        assert_eq!(resolve_level_up_channel(&config), None);
    }

    #[test]
    fn returns_none_when_invalid() {
        let mut config = HashMap::new();
        config.insert("level_up_channel_id".into(), "not_a_number".into());
        assert_eq!(resolve_level_up_channel(&config), None);
    }

    #[test]
    fn returns_none_when_zero() {
        let mut config = HashMap::new();
        config.insert("level_up_channel_id".into(), "0".into());
        assert_eq!(resolve_level_up_channel(&config), None);
    }

    #[test]
    fn returns_channel_id_when_valid() {
        let mut config = HashMap::new();
        config.insert("level_up_channel_id".into(), "123456789012345678".into());
        assert_eq!(resolve_level_up_channel(&config), Some(123456789012345678));
    }

    #[test]
    fn returns_channel_id_small_number() {
        let mut config = HashMap::new();
        config.insert("level_up_channel_id".into(), "42".into());
        assert_eq!(resolve_level_up_channel(&config), Some(42));
    }
}
