//! Configuration du module tickets.
//! Contrairement au ticket-bot d'origine, on ne demande pas de token Discord
//! ici : le sentinel-bot unifie utilise un seul token partage. On conserve
//! juste les env vars de categorie et de salon de panel.

use crate::shared::config::load_env_optional;
use serenity::prelude::TypeMapKey;

#[derive(Clone, Default)]
pub struct TicketsConfig {
    pub ticket_channel_id: Option<u64>,
}

impl TicketsConfig {
    pub fn from_env() -> Self {
        Self {
            ticket_channel_id: load_env_optional("TICKET_CHANNEL_ID"),
        }
    }
}

pub struct ConfigKey;

impl TypeMapKey for ConfigKey {
    type Value = TicketsConfig;
}
