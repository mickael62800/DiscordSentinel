use std::time::Duration;

use rand::Rng;
use serenity::all::{ChannelType, CreateEmbed, CreateEmbedFooter, CreateMessage, GuildId};
use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{error, info};

use sentinel_shared::heartbeat::register_guilds;

use crate::commands;
use crate::db::GameDb;

/// Cle TypeMap pour le client de base de donnees du jeu.
pub struct GameDbKey;
impl TypeMapKey for GameDbKey {
    type Value = GameDb;
}

/// Cle TypeMap pour stocker les guild IDs connus.
pub struct GuildIdsKey;
impl TypeMapKey for GuildIdsKey {
    type Value = Vec<GuildId>;
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
            info!("Slash commands enregistrees : coude, profil, shop, casino, prime, leaderboard, pari, voler, assurance, train");
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
                    "casino" => commands::casino::handle(&ctx, &command).await,
                    "prime" => commands::prime::handle(&ctx, &command).await,
                    "leaderboard" => commands::leaderboard::handle(&ctx, &command).await,
                    "pari" => commands::pari::handle(&ctx, &command).await,
                    "voler" => commands::voler::handle(&ctx, &command).await,
                    "assurance" => commands::assurance::handle(&ctx, &command).await,
                    "train" => commands::train::handle(&ctx, &command).await,
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
                }
            }
            _ => {}
        }
    }
}

async fn run_daily_chaos(ctx: &Context) {
    let data = ctx.data.read().await;
    let db = match data.get::<GameDbKey>() {
        Some(db) => db,
        None => return,
    };
    let guild_ids = match data.get::<GuildIdsKey>() {
        Some(ids) => ids.clone(),
        None => return,
    };

    for guild_id in guild_ids {
        let gid = guild_id.to_string();

        let players = match db.get_random_players(&gid, 2).await {
            Ok(p) if p.len() >= 2 => p,
            _ => continue,
        };

        let victim = &players[0];
        let winner = &players[1];

        // Victime perd 20% de ses coins
        let amount = (victim.coins as f64 * 0.20) as i64;
        if amount < 1 {
            continue;
        }

        // Transferer
        if let Err(e) = db
            .transfer_coins(&gid, &victim.user_id, &winner.user_id, amount)
            .await
        {
            error!(error = %e, guild = %gid, "Erreur daily chaos transfer");
            continue;
        }

        if let Err(e) = db
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

        // Trouver le premier channel texte pour poster l'annonce
        let channels = match guild_id.channels(&ctx.http).await {
            Ok(c) => c,
            Err(_) => continue,
        };

        let text_channel = channels.values().find(|c| c.kind == ChannelType::Text);

        if let Some(channel) = text_channel {
            let embed = CreateEmbed::new()
                .title("\u{1f32a}\u{fe0f} LA ROUE DU DESTIN A TOURNE !")
                .description(format!(
                    "\u{1f480} <@{}> perd **{} coins** (-20%)\n\u{1f381} <@{}> gagne **{} coins** gratuitement !\n\nLa vie est injuste. Coup de Coude aussi.",
                    victim.user_id, amount, winner.user_id, amount
                ))
                .color(0x9B59B6)
                .footer(CreateEmbedFooter::new("Coup de Coude | Sentinel"))
                .timestamp(serenity::model::Timestamp::now());

            let _ = channel
                .id
                .send_message(&ctx.http, CreateMessage::new().embed(embed))
                .await;
        }
    }
}
