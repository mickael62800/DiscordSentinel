//! Module bot du jeu « Influence » (cf. docs/Nouveau jeux/ARCHITECTURE.md).
//!
//! Phase 1 (MVP) : commande `/influence-profil`. Les organisations et votes
//! s'ajoutent aux lots suivants.

use serenity::all::{CommandInteraction, Context, CreateCommand};

use crate::shared::discord_helpers::is_module_enabled_or_reply_command;

pub mod api_client;
pub mod commands;

pub const MODULE_BOT_NAME: &str = "influence-bot";

/// Commandes slash exposees par le module.
pub fn register_commands() -> Vec<CreateCommand> {
    vec![commands::profil::register(), commands::org::register()]
}

/// `true` si la commande appartient a ce module.
pub fn handles_command(name: &str) -> bool {
    matches!(name, "influence-profil" | "org")
}

/// Dispatch d'une commande du module.
pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }
    match command.data.name.as_str() {
        "influence-profil" => commands::profil::handle(ctx, command).await,
        "org" => commands::org::handle(ctx, command).await,
        _ => {}
    }
}

/// `true` si le composant (bouton/menu) appartient a ce module.
pub fn handles_component(_cid: &str) -> bool {
    false
}
