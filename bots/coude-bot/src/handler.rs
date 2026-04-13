use std::time::Duration;

use rand::Rng;
use serenity::all::{CreateEmbed, CreateEmbedFooter, CreateMessage, GuildId};
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

        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::all(),
        )
        .await
        {
            error!(error = %e, "Impossible d'enregistrer les slash commands");
        } else {
            info!("Slash commands enregistrees : coude, profil, shop, prime, leaderboard, pari, voler, assurance, train");
        }

        // Daily chaos background task
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            loop {
                // Random delay entre 18-24 heures
                let delay = {
                    let mut rng = rand::thread_rng();
                    rng.gen_range(18 * 3600..24 * 3600)
                };
                tokio::time::sleep(Duration::from_secs(delay)).await;

                run_daily_chaos(&ctx_clone).await;
            }
        });
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                let cmd_name = command.data.name.clone();

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
                    "saison" => commands::saison::handle(&ctx, &command).await,
                    "reset-stats" => commands::reset_stats::handle(&ctx, &command).await,
                    "resume" => commands::resume::handle(&ctx, &command).await,
                    _ => {}
                }
            }
            Interaction::Component(component) => {
                let custom_id = &component.data.custom_id;

                if custom_id.starts_with(commands::accepter::ACCEPT_PREFIX) {
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

async fn run_daily_chaos(ctx: &Context) {
    let guild_ids = {
        let data = ctx.data.read().await;
        match data.get::<GuildIdsKey>() {
            Some(ids) => ids.clone(),
            None => return,
        }
    };

    for guild_id in guild_ids {
        let gid = guild_id.to_string();

        let config = load_guild_config(ctx, &gid).await;
        if !config.enabled() || !config.daily_chaos_enabled() {
            continue;
        }

        let data = ctx.data.read().await;
        let api = match data.get::<crate::GameApiKey>() {
            Some(api) => api,
            None => return,
        };

        let players = match api.get_random_players(&gid, 2).await {
            Ok(p) if p.len() >= 2 => p,
            _ => continue,
        };

        let victim = &players[0];
        let winner = &players[1];

        // Victime perd X% de ses coins (depuis la config)
        let amount = (victim.coins as f64 * config.daily_chaos_percent()) as i64;
        if amount < 1 {
            continue;
        }

        // Transferer + comptabiliser (total_lost pour la victime, total_earned pour le gagnant)
        if let Err(e) = api
            .record_steal(&gid, &winner.user_id, &victim.user_id, amount)
            .await
        {
            error!(error = %e, guild = %gid, "Erreur daily chaos transfer");
            continue;
        }

        if let Err(e) = api
            .log_daily_chaos(
                &gid,
                &victim.user_id,
                &victim.username,
                &winner.user_id,
                &winner.username,
                amount,
            )
            .await
        {
            error!(error = %e, guild = %gid, "Erreur daily chaos log");
        }

        // Poster dans le salon annonces configure
        let announce_channel = match config.channel_announcements() {
            Some(id) => match id.parse::<u64>() {
                Ok(n) => serenity::model::id::ChannelId::new(n),
                Err(_) => continue,
            },
            None => continue, // Pas de salon configure → pas de chaos
        };

        let embed = CreateEmbed::new()
            .title("\u{1f32a}\u{fe0f} LA ROUE DU DESTIN A TOURNE !")
            .description(format!(
                "\u{1f480} <@{}> perd **{} coins** (-20%)\n\u{1f381} <@{}> gagne **{} coins** gratuitement !\n\nLa vie est injuste. Coup de Coude aussi.",
                victim.user_id, amount, winner.user_id, amount
            ))
            .color(0x9B59B6)
            .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
            .timestamp(serenity::model::Timestamp::now());

        if let Err(e) = announce_channel
            .send_message(&ctx.http, CreateMessage::new().embed(embed))
            .await
        {
            warn!(error = %e, "Failed to send announcement message");
        }
    }
}
