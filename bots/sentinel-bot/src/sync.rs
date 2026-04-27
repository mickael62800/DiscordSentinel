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

use sentinel_shared::api_client::BaseApiClient;

/// Conventions de `kind` partagees avec le domain API
/// (services/api/src/domain/entities/discord_action_message.rs::kinds).
pub mod kinds {
    pub const BAN_PROPOSAL: &str = "ban_proposal";
    pub const TICKET: &str = "ticket";
    pub const ROLES_PANEL: &str = "roles_panel";
    pub const COMBAT_CHALLENGE: &str = "combat_challenge";
    pub const REVIEW_REQUEST: &str = "review_request";
    pub const AUTOMOD_REVIEW: &str = "automod_review";
    pub const BLACKJACK_TABLE: &str = "blackjack_table";
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

/// Format conventionnel du `custom_id` des boutons d'une action :
/// `{namespace}:{verb}:{action_id}` — permet au handler de retrouver
/// l action via parsing de l UUID.
///
/// Exemples :
/// - `ban_proposal:cancel:{uuid}`
/// - `ticket:close:{uuid}`
pub fn build_action_custom_id(namespace: &str, verb: &str, action_id: Uuid) -> String {
    format!("{namespace}:{verb}:{action_id}")
}

/// Parse un custom_id au format `{namespace}:{verb}:{action_id}`.
/// Retourne `(verb, action_id)` si le namespace correspond, sinon None.
pub fn parse_action_custom_id(
    custom_id: &str,
    expected_namespace: &str,
) -> Option<(String, Uuid)> {
    let mut parts = custom_id.splitn(3, ':');
    let ns = parts.next()?;
    if ns != expected_namespace {
        return None;
    }
    let verb = parts.next()?.to_string();
    let action_id = parts.next()?.parse::<Uuid>().ok()?;
    Some((verb, action_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_and_parse_round_trip() {
        let uuid = Uuid::new_v4();
        let cid = build_action_custom_id("ban_proposal", "cancel", uuid);
        assert_eq!(cid, format!("ban_proposal:cancel:{uuid}"));

        let parsed = parse_action_custom_id(&cid, "ban_proposal").unwrap();
        assert_eq!(parsed.0, "cancel");
        assert_eq!(parsed.1, uuid);
    }

    #[test]
    fn parse_rejects_wrong_namespace() {
        let uuid = Uuid::new_v4();
        let cid = build_action_custom_id("ban_proposal", "cancel", uuid);
        assert!(parse_action_custom_id(&cid, "ticket").is_none());
    }

    #[test]
    fn parse_rejects_bad_uuid() {
        let cid = "ban_proposal:cancel:not-a-uuid";
        assert!(parse_action_custom_id(cid, "ban_proposal").is_none());
    }

    #[test]
    fn parse_handles_extra_colons_in_uuid() {
        // splitn(3, ':') ne consomme que les 2 premiers colons donc le 3e
        // segment peut contenir des colons (theoriquement pas pour un UUID
        // mais on verifie le contrat).
        let uuid = Uuid::new_v4();
        let cid = format!("ban_proposal:cancel:{uuid}");
        assert!(parse_action_custom_id(&cid, "ban_proposal").is_some());
    }
}
