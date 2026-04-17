//! Middleware simple : bloque les commandes gameplay si le joueur est
//! en prison (Phase 10 /braquage).
//!
//! Appele par `handler.rs` avant le dispatch, pour une whitelist de
//! commandes qui consomment ou modifient les coins / l'inventaire.
//! Les commandes passives (/profil, /cagnotte, /leaderboard, /hp,
//! /saison, /resume, /no-taunts, /annuler, /refuser) ne sont PAS
//! bloquees.
//!
//! Zero logique metier cote bot : on appelle GetPrisonStatus via
//! gRPC, l'API decide si le joueur est in_prison (comparant released_at
//! a NOW). On affiche juste le message ephemeral de refus.

use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

use crate::modules::coude::GameApiKey;

/// Prefixes de boutons bloques en prison (actions offensives/actives).
/// Refuser, annuler, classe = passif, pas bloque.
const BLOCKED_BUTTON_PREFIXES: &[&str] = &[
    "coude_prec_ok|",     // confirmer un defi
    "coude_accept:",      // accepter un defi
    "coude_defend:",      // utiliser item defensif (= participer au combat)
    "coude_defend_select:", // choisir item defensif
    "steal_defend:",      // se defendre contre un vol
];

/// Liste des noms de commandes bloquees quand le joueur est en prison.
/// Les commandes passives (lecture seule) ne sont pas bloquees.
const BLOCKED_IN_PRISON: &[&str] = &[
    "coude",
    "voler",
    "pari",
    "prime",
    "potion",
    "shop",
    "protection",
    "boost-voleur",
    "train",
    "classe",
    "donner",
    "repos",
    "reset-stats",
    "braquage",
];

/// Retourne `true` si la commande est bloquee parce que le joueur est
/// en prison. Le handler doit alors `return` immediatement.
///
/// **Important** : cette fonction peut envoyer une `create_response`
/// ephemere en cas de blocage. Elle doit donc etre appelee AVANT tout
/// autre `create_response` / `defer` dans le handler de la commande.
pub async fn check_and_reply_if_in_prison(
    ctx: &Context,
    command: &CommandInteraction,
) -> bool {
    let cmd_name = command.data.name.as_str();

    // Not in the blocked list = always allowed.
    if !BLOCKED_IN_PRISON.iter().any(|c| *c == cmd_name) {
        return false;
    }

    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => return false, // DMs : pas de prison
    };
    let user_id = command.user.id.to_string();

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => return false, // Pas d'API, on laisse passer (fail-open)
    };

    let status = match api.get_prison_status(&guild_id, &user_id).await {
        Ok(s) => s,
        Err(e) => {
            // Fail-open : si l'API est down, on ne bloque pas. Mieux
            // vaut laisser jouer qu'empecher tout le monde.
            tracing::warn!(error = %e, "Echec get_prison_status (fail-open)");
            return false;
        }
    };

    if !status.in_prison {
        return false;
    }

    // Joueur en prison : on refuse.
    let until_str = status
        .released_at
        .as_deref()
        .and_then(|ts| ts.split(&[' ', 'T'][..]).next())
        .map(|d| format!(" jusqu'au **{}**", d))
        .unwrap_or_default();

    let msg = format!(
        "\u{26d3}\u{fe0f} **Tu es en prison{}** !\n\n\
         Aucune action de jeu n'est possible (ni combat, ni achat, ni vol, \
         ni meme `/repos`). Attends ta liberation ou accepte ton sort.",
        until_str
    );

    if let Err(e) = command
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(msg)
                    .ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec reponse prison block");
    }

    true
}

/// Variante pour les interactions de boutons/selects (ComponentInteraction).
/// Meme logique : si le joueur est en prison et le bouton est offensif, on bloque.
pub async fn check_component_in_prison(
    ctx: &Context,
    component: &ComponentInteraction,
) -> bool {
    let custom_id = &component.data.custom_id;

    if !BLOCKED_BUTTON_PREFIXES.iter().any(|p| custom_id.starts_with(p)) {
        return false;
    }

    let guild_id = match component.guild_id {
        Some(id) => id.to_string(),
        None => return false,
    };
    let user_id = component.user.id.to_string();

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => return false,
    };

    let status = match api.get_prison_status(&guild_id, &user_id).await {
        Ok(s) => s,
        Err(_) => return false, // fail-open
    };

    if !status.in_prison {
        return false;
    }

    let msg = "\u{26d3}\u{fe0f} **Tu es en prison !** Impossible d'effectuer cette action.";
    if let Err(e) = component
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(msg)
                    .ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec reponse prison block (component)");
    }

    true
}

/// Helper testable : retourne true si le custom_id est bloque en prison.
#[allow(dead_code)]
pub fn is_button_blocked(custom_id: &str) -> bool {
    BLOCKED_BUTTON_PREFIXES.iter().any(|p| custom_id.starts_with(p))
}

/// Helper testable : retourne true si la commande est bloquee en prison.
#[allow(dead_code)]
pub fn is_command_blocked(cmd_name: &str) -> bool {
    BLOCKED_IN_PRISON.iter().any(|c| *c == cmd_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coude_is_blocked() { assert!(is_command_blocked("coude")); }
    #[test]
    fn voler_is_blocked() { assert!(is_command_blocked("voler")); }
    #[test]
    fn braquage_is_blocked() { assert!(is_command_blocked("braquage")); }
    #[test]
    fn profil_not_blocked() { assert!(!is_command_blocked("profil")); }
    #[test]
    fn leaderboard_not_blocked() { assert!(!is_command_blocked("leaderboard")); }
    #[test]
    fn hp_not_blocked() { assert!(!is_command_blocked("hp")); }

    #[test]
    fn accept_button_blocked() { assert!(is_button_blocked("coude_accept:123")); }
    #[test]
    fn preconfirm_ok_blocked() { assert!(is_button_blocked("coude_prec_ok|456")); }
    #[test]
    fn defend_blocked() { assert!(is_button_blocked("coude_defend:789")); }
    #[test]
    fn steal_defend_blocked() { assert!(is_button_blocked("steal_defend:abc")); }
    #[test]
    fn refuse_not_blocked() { assert!(!is_button_blocked("coude_refuse:123")); }
    #[test]
    fn cancel_not_blocked() { assert!(!is_button_blocked("coude_cancel:123")); }
    #[test]
    fn class_not_blocked() { assert!(!is_button_blocked("class_select:tank")); }
    #[test]
    fn random_not_blocked() { assert!(!is_button_blocked("some_other_button")); }
}
