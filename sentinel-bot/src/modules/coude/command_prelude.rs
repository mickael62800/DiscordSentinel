//! Preamble commun aux commandes slash du module Coude.
//!
//! Presque toutes les commandes `coude/commands/*.rs` ouvraient leur `handle`
//! avec la meme sequence : resolution du `guild_id`, chargement de la config
//! guild, verification du salon, (parfois) gate `enabled()`, puis recuperation
//! du client API. `coude_prelude` factorise cette sequence a l'identique.

use serenity::all::{CommandInteraction, Context};

use crate::shared::discord_helpers::{reply_ephemeral, require_guild_id};

use crate::modules::coude::api_client::ApiClient;
use crate::modules::coude::channel_check;
use crate::modules::coude::guild_config::Config;
use crate::modules::coude::{load_guild_config, GameApiKey};

/// Execute le preamble standard d'une commande Coude :
///   1. resout le `guild_id` (sinon `None`, garde a deja repondu) ;
///   2. charge la config guild ;
///   3. verifie le salon via `channel` (selecteur, ex. `Config::channel_profil`) ;
///   4. si `check_enabled`, applique le gate `enabled()` avec le message
///      ephemeral standard ;
///   5. clone et renvoie le client API.
///
/// Retourne `None` des qu'un garde a deja repondu/return — l'appelant doit
/// alors faire `else { return; }`.
pub async fn coude_prelude(
    ctx: &Context,
    command: &CommandInteraction,
    channel: impl Fn(&Config) -> Option<String>,
    check_enabled: bool,
) -> Option<(String, Config, ApiClient)> {
    let guild_id = require_guild_id(ctx, command).await?;

    let config = load_guild_config(ctx, &guild_id).await;
    if !channel_check::check_channel(ctx, command, channel(&config)).await {
        return None;
    }
    if check_enabled && !config.enabled() {
        reply_ephemeral(
            ctx,
            command,
            "Le jeu Coup de Coude est desactive sur ce serveur.",
        )
        .await;
        return None;
    }

    let api = {
        let data = ctx.data.read().await;
        data.get::<GameApiKey>().unwrap().clone()
    };

    Some((guild_id, config, api))
}
