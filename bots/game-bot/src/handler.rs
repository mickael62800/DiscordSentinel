use std::sync::Arc;

use serenity::async_trait;
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::model::application::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::heartbeat::{ApiClientKey, register_guilds};

use crate::api_client::GameApiClient;
use crate::commands;
use crate::detector;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Game bot connecte");

        register_guilds(&ctx, &ready).await;

        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::all(),
        )
        .await
        {
            error!(error = %e, "Erreur enregistrement commandes");
        } else {
            info!("Slash commands enregistrees : game");
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let guild_id = match msg.guild_id {
            Some(g) => g,
            None => return,
        };

        // Verifier si le bot est active
        {
            let data = ctx.data.read().await;
            if let Some(api) = data.get::<ApiClientKey>() {
                if !sentinel_shared::discord_helpers::is_bot_enabled(api, &guild_id.to_string()).await {
                    return;
                }
            }
        }

        // Detecter les mentions de jeux (#NomDuJeu)
        let mentions = detector::extract_game_mentions(&msg.content);
        if mentions.is_empty() {
            return;
        }

        let data = ctx.data.read().await;
        let base = match data.get::<ApiClientKey>() {
            Some(b) => Arc::clone(b),
            None => return,
        };
        drop(data);

        let api = GameApiClient::new(base);
        let guild_id_str = guild_id.to_string();

        for mention in &mentions {
            // Trouver le jeu par nom
            let game = match api.get_game_by_name(&guild_id_str, mention).await {
                Ok(Some(g)) => g,
                Ok(None) => continue, // Pas un jeu enregistre, on ignore
                Err(e) => {
                    warn!(error = %e, game = %mention, "Erreur recherche jeu");
                    continue;
                }
            };

            // Recuperer les inscrits
            let subs = match api.get_subscribers(&guild_id_str, &game.id).await {
                Ok(s) => s,
                Err(e) => {
                    warn!(error = %e, game = %game.game_name, "Erreur recuperation inscrits");
                    continue;
                }
            };

            if subs.is_empty() {
                continue;
            }

            // Filtrer l'auteur du message (pas besoin de se ping soi-meme)
            let pings: Vec<String> = subs
                .iter()
                .filter(|s| s.user_id != msg.author.id.to_string())
                .map(|s| format!("<@{}>", s.user_id))
                .collect();

            if pings.is_empty() {
                continue;
            }

            let embed = CreateEmbed::new()
                .title(format!("\u{1f3ae} {}", game.game_name))
                .description(format!(
                    "<@{}> cherche des joueurs pour **{}** !\n\n{}",
                    msg.author.id,
                    game.game_name,
                    pings.join(" "),
                ))
                .color(0x3498db)
                .footer(serenity::builder::CreateEmbedFooter::new(
                    format!("{} joueur(s) notifie(s) | /game join {} pour rejoindre", pings.len(), game.game_name),
                ));

            if let Err(e) = msg
                .channel_id
                .send_message(&ctx.http, CreateMessage::new().content(pings.join(" ")).embed(embed))
                .await
            {
                warn!(error = %e, game = %game.game_name, "Erreur envoi notification jeu");
            } else {
                info!(
                    game = %game.game_name,
                    players = pings.len(),
                    caller = %msg.author.name,
                    "Joueurs notifies pour #{}", game.game_name
                );
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            if command.data.name.as_str() == "game" {
                commands::handle(&ctx, &command).await;
            }
        }
    }
}
