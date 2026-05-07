//! Configuration du module tickets.
//! Contrairement au ticket-bot d'origine, on ne demande pas de token Discord
//! ici : le sentinel-bot unifie utilise un seul token partage. On conserve
//! juste les env vars de categorie et de salon de panel.

use serenity::prelude::TypeMapKey;
use crate::shared::config::load_env_optional;

#[derive(Clone, Default)]
#[allow(dead_code)]
pub struct TicketsConfig {
    pub ticket_category_id: Option<u64>,
    pub ticket_channel_id: Option<u64>,
}

impl TicketsConfig {
    pub fn from_env() -> Self {
        Self {
            ticket_category_id: load_env_optional("TICKET_CATEGORY_ID"),
            ticket_channel_id: load_env_optional("TICKET_CHANNEL_ID"),
        }
    }
}

pub struct ConfigKey;

impl TypeMapKey for ConfigKey {
    type Value = TicketsConfig;
}
