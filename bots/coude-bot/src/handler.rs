use serenity::all::GuildId;
use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::heartbeat::{register_guilds, ApiClientKey};

use crate::commands;
use crate::guild_config::CoudeConfig;

/// Cle TypeMap pour stocker les guild IDs connus.
pub struct GuildIdsKey;
impl TypeMapKey for GuildIdsKey {
    type Value = Vec<GuildId>;
}

/// Charge la config guild depuis l'API (avec cache Redis cote API, TTL 15min).
pub async fn load_guild_config(ctx: &Context, guild_id: &str) -> CoudeConfig {
    let data = ctx.data.read().await;
    let api = data.get::<ApiClientKey>().unwrap();
    CoudeConfig::load(api, guild_id).await
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Coude bot connecte");

        register_guilds(&ctx, &ready).await;

        // Stocker les guild IDs pour le daily chaos
        let guild_ids: Vec<GuildId> = ready.guilds.iter().map(|g| g.id).collect();
        {
            let mut data = ctx.data.write().await;
            data.insert::<GuildIdsKey>(guild_ids);
        }

        // Clean up any lingering global commands. Historiquement, coude-bot
        // a pu etre deploye avec set_global_commands a un moment ; ces
        // commandes globales persistent cote Discord jusqu'a ce qu'on les
        // efface explicitement, ce qui creait des doublons (global +
        // per-guild affiches en parallele). On force un vec![] global au
        // boot pour garantir que seules les commandes per-guild vivent.
        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            vec![],
        )
        .await
        {
            warn!(error = %e, "Echec nettoyage des commandes globales (non-bloquant)");
        } else {
            info!("Commandes globales nettoyees (dedoublonnage).");
        }

        // Enregistrement per-guild : propagation INSTANTANEE (contrairement
        // a set_global_commands qui peut prendre jusqu'a 1h). A chaque
        // redemarrage, la liste est re-ecrasee sur chaque guild connectee.
        let cmds_count = commands::all().len();
        for guild in ready.guilds.iter() {
            if let Err(e) = guild.id.set_commands(&ctx.http, commands::all()).await {
                error!(
                    guild_id = %guild.id,
                    error = %e,
                    "Impossible d'enregistrer les slash commands pour cette guild"
                );
            }
        }
        info!(
            count = cmds_count,
            guilds = ready.guilds.len(),
            "Slash commands enregistrees per-guild : coude, profil, shop, prime, leaderboard, pari, potion, voler, assurance, train, classe, donner, hp, repos, saison, reset-stats, resume"
        );

        // Daily chaos migre dans coude-worker (timer aleatoire + API decide).
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                let cmd_name = command.data.name.clone();

                // Phase 10 : prison check avant dispatch. Si le joueur est
                // en prison (echec /braquage), les commandes gameplay sont
                // bloquees avec un message ephemeral et on return direct.
                // Les commandes passives (profil, leaderboard, cagnotte…)
                // ne sont pas dans la whitelist et passent.
                if crate::prison_check::check_and_reply_if_in_prison(&ctx, &command).await {
                    return;
                }

                match cmd_name.as_str() {
                    "coude" => commands::coude::handle(&ctx, &command).await,
                    "profil" => commands::profil::handle(&ctx, &command).await,
                    "shop" => commands::shop_cmd::handle(&ctx, &command).await,
                    "prime" => commands::prime::handle(&ctx, &command).await,
                    "leaderboard" => commands::leaderboard::handle(&ctx, &command).await,
                    "pari" => commands::pari::handle(&ctx, &command).await,
                    "voler" => commands::voler::handle(&ctx, &command).await,
                    "assurance" => commands::assurance::handle(&ctx, &command).await,
                    "train" => commands::train::handle(&ctx, &command).await,
                    "classe" => commands::classe::handle(&ctx, &command).await,
                    "donner" => commands::donner::handle(&ctx, &command).await,
                    "hp" => commands::hp::handle(&ctx, &command).await,
                    "repos" => commands::repos::handle(&ctx, &command).await,
                    "potion" => commands::potion::handle(&ctx, &command).await,
                    "saison" => commands::saison::handle(&ctx, &command).await,
                    "reset-stats" => commands::reset_stats::handle(&ctx, &command).await,
                    "resume" => commands::resume::handle(&ctx, &command).await,
                    "cagnotte" => commands::cagnotte::handle(&ctx, &command).await,
                    "protection" => commands::protection::handle(&ctx, &command).await,
                    "boost-voleur" => commands::boost_voleur::handle(&ctx, &command).await,
                    "no-taunts" => commands::no_taunts::handle(&ctx, &command).await,
                    "taunts-channel" => commands::taunts_channel::handle(&ctx, &command).await,
                    "braquage" => commands::braquage::handle(&ctx, &command).await,
                    _ => {}
                }
            }
            Interaction::Component(component) => {
                // Prison check sur les boutons offensifs.
                if crate::prison_check::check_component_in_prison(&ctx, &component).await {
                    return;
                }

                let custom_id = &component.data.custom_id;

                if custom_id.starts_with(commands::coude::PRECONFIRM_OK_PREFIX) {
                    commands::coude::handle_preconfirm_ok(&ctx, &component).await;
                } else if custom_id.starts_with(commands::coude::PRECONFIRM_NO_PREFIX) {
                    commands::coude::handle_preconfirm_no(&ctx, &component).await;
                } else if custom_id.starts_with(commands::accepter::ACCEPT_PREFIX) {
                    commands::accepter::handle(&ctx, &component).await;
                } else if custom_id.starts_with(commands::defend_item::DEFEND_SELECT_PREFIX) {
                    commands::defend_item::handle_defend_select(&ctx, &component).await;
                } else if custom_id.starts_with(commands::defend_item::DEFEND_PREFIX) {
                    commands::defend_item::handle_defend_button(&ctx, &component).await;
                } else if custom_id.starts_with(commands::refuser::REFUSE_PREFIX) {
                    commands::refuser::handle(&ctx, &component).await;
                } else if custom_id.starts_with(commands::annuler::CANCEL_PREFIX) {
                    commands::annuler::handle(&ctx, &component).await;
                } else if custom_id.starts_with(commands::classe::CLASS_SELECT_PREFIX) {
                    commands::classe::handle_select(&ctx, &component).await;
                } else if custom_id.starts_with(commands::voler::STEAL_DEFEND_PREFIX) {
                    commands::voler::handle_defend(&ctx, &component).await;
                }
            }
            _ => {}
        }
    }
}

// run_daily_chaos supprime — migre dans coude-worker + API.
