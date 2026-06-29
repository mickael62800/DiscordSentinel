//! Module slot (machine a sous / tirette).
//!
//! Pattern aligne sur blackjack : panel persistant global avec un bouton
//! "Ouvrir ma machine" -> creation d un salon Discord prive par utilisateur,
//! ou se deroulent les spins avec animation suspense (3 frames de 2s),
//! resultat dans un message classique (non-ephemere) puis re-post des boutons
//! d action en bas.

pub const MODULE_BOT_NAME: &str = "slot-bot";

pub mod animation;
pub mod api_client;
mod buttons;
pub mod channel_manager;
mod embeds;
pub mod setup;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serenity::all::{CommandInteraction, ComponentInteraction, Context, CreateCommand};
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::discord_helpers::{
    is_module_enabled_or_reply_command, is_module_enabled_or_reply_component,
};
use crate::shared::heartbeat::ApiClientKey;

pub use channel_manager::SlotChannelManager;

/// Timeout d'inactivite par defaut (2 min) avant fermeture auto d'un salon
/// slot. Override par guild via `bot_guild_config` (slot-bot / afk_timeout_secs).
const DEFAULT_AFK_TIMEOUT_SECS: u64 = 120;

/// Intervalle de scan du cleanup AFK.
const AFK_SCAN_INTERVAL_SECS: u64 = 60;

/// TypeMapKey pour stocker le SlotChannelManager dans le ctx.data partage.
pub struct SlotChannelManagerKey;
impl TypeMapKey for SlotChannelManagerKey {
    type Value = Arc<SlotChannelManager>;
}

/// Initialise les TypeMapKeys du module dans le TypeMap partage.
/// Appele au boot depuis `main.rs` comme les autres modules.
pub fn init_typemap(data: &mut TypeMap) {
    data.insert::<SlotChannelManagerKey>(Arc::new(SlotChannelManager::new()));
}

pub fn register_commands() -> Vec<CreateCommand> {
    vec![setup::register()]
}

/// Tache de fond : ferme automatiquement les salons slot inactifs.
///
/// Le suivi des salons est en memoire (DashMap dans le bot), donc — contraire-
/// ment au blackjack qui persiste ses tables en DB et delegue au worker — le
/// cleanup doit vivre ici. Toutes les `AFK_SCAN_INTERVAL_SECS`, on snapshot les
/// salons actifs, on lit le timeout PAR GUILD (defaut `DEFAULT_AFK_TIMEOUT_SECS`)
/// et on ferme ceux inactifs depuis trop longtemps (delete Discord + retrait du
/// manager). Best-effort : un delete qui echoue (salon deja supprime) est logge
/// puis ignore, mais l'entree est tout de meme retiree du manager.
pub fn spawn_background(ctx: Context) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(AFK_SCAN_INTERVAL_SECS)).await;

            let channels = {
                let data = ctx.data.read().await;
                match data.get::<SlotChannelManagerKey>() {
                    Some(mgr) => mgr.snapshot(),
                    None => continue,
                }
            };
            if channels.is_empty() {
                continue;
            }

            let api = {
                let data = ctx.data.read().await;
                data.get::<ApiClientKey>().map(Arc::clone)
            };

            // Cache des timeouts par guild pour ne lire la config qu'une fois
            // par guild et par tick.
            let mut timeout_by_guild: HashMap<String, u64> = HashMap::new();

            for (user_id, channel) in channels {
                let guild_key = channel.guild_id.to_string();
                let timeout = match timeout_by_guild.get(&guild_key) {
                    Some(t) => *t,
                    None => {
                        let t = match &api {
                            Some(api) => api
                                .get_guild_config_for(&guild_key, MODULE_BOT_NAME)
                                .await
                                .ok()
                                .and_then(|cfg| {
                                    cfg.get("afk_timeout_secs")
                                        .and_then(|v| v.parse::<u64>().ok())
                                })
                                .filter(|v| *v > 0)
                                .unwrap_or(DEFAULT_AFK_TIMEOUT_SECS),
                            None => DEFAULT_AFK_TIMEOUT_SECS,
                        };
                        timeout_by_guild.insert(guild_key.clone(), t);
                        t
                    }
                };

                if SlotChannelManager::idle_secs(&channel) < timeout {
                    continue;
                }

                // Ferme le salon Discord (best-effort) puis retire du manager.
                if let Err(e) = channel.channel_id.delete(&ctx.http).await {
                    warn!(error = %e, channel = %channel.channel_id, "Echec suppression salon slot AFK");
                }
                {
                    let data = ctx.data.read().await;
                    if let Some(mgr) = data.get::<SlotChannelManagerKey>() {
                        mgr.remove(user_id);
                    }
                }
                info!(
                    channel = %channel.channel_id,
                    user = %user_id,
                    timeout,
                    "Salon slot ferme (inactivite)"
                );
            }
        }
    });
}

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if !is_module_enabled_or_reply_command(ctx, command, MODULE_BOT_NAME).await {
        return;
    }
    if command.data.name == "slot-setup" {
        setup::handle(ctx, command).await;
    }
}

pub fn handles_component(cid: &str) -> bool {
    cid == setup::PANEL_OPEN_ID
        || cid == setup::CHANNEL_SPIN_ID
        || cid == setup::CHANNEL_DAILY_ID
        || cid == setup::CHANNEL_CLOSE_ID
}

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    if !is_module_enabled_or_reply_component(ctx, component, MODULE_BOT_NAME).await {
        return;
    }
    let cid = component.data.custom_id.as_str();
    if cid == setup::PANEL_OPEN_ID {
        buttons::handle_open_machine(ctx, component).await;
    } else if cid == setup::CHANNEL_SPIN_ID {
        buttons::handle_spin_in_channel(ctx, component).await;
    } else if cid == setup::CHANNEL_DAILY_ID {
        buttons::handle_daily_in_channel(ctx, component).await;
    } else if cid == setup::CHANNEL_CLOSE_ID {
        buttons::handle_close_channel(ctx, component).await;
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
