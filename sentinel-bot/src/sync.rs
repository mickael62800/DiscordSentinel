//! Helpers de synchronisation Discord <-> Web (cf. SYNC_DISCORD_WEB_DESIGN.md).
//!
//! Quand un module poste un message Discord lie a une entite metier
//! (ban_proposal, ticket, roles_panel...), il appelle
//! `register_action_message` pour persister la correspondance
//! `action_id <-> (channel_id, message_id)` cote API.
//!
//! L appel est fire-and-forget : si l API est down, le post Discord
//! reste valide (juste pas synchronise).

use std::sync::Arc;
use uuid::Uuid;

use crate::shared::api_client::BaseApiClient;

/// Conventions de `kind` partagees avec le domain API
/// (sentinel-api/src/domain/entities/discord_action_message.rs::kinds).
pub mod kinds {
    pub const TICKET: &str = "ticket";
    pub const AUTOMOD_REVIEW: &str = "automod_review";
}

#[derive(serde::Serialize)]
struct RegisterBody<'a> {
    action_id: Uuid,
    kind: &'a str,
    guild_id: &'a str,
    channel_id: &'a str,
    message_id: &'a str,
}

/// Enregistre une correspondance `action_id <-> message Discord`.
/// Fire-and-forget : log d'erreur mais ne propage pas. A appeler juste
/// apres avoir poste le message Discord.
pub async fn register_action_message(
    api: &Arc<BaseApiClient>,
    action_id: Uuid,
    kind: &str,
    guild_id: &str,
    channel_id: &str,
    message_id: &str,
) {
    let body = RegisterBody {
        action_id,
        kind,
        guild_id,
        channel_id,
        message_id,
    };
    api.post_fire_and_forget("/api/discord-messages/register", &body)
        .await;
}
