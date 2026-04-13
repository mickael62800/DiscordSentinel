use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

/// Cooldown anti-spam sur la detection des mentions `#Jeu` : un user ne
/// peut declencher au maximum une notification pour un jeu donne toutes
/// les N secondes dans la meme guild.
const MENTION_COOLDOWN_SECS: u64 = 60;

static MENTION_COOLDOWN: std::sync::LazyLock<Mutex<HashMap<(u64, u64, String), Instant>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Retourne true si l'appelant n'a pas encore cooldown pour ce jeu.
fn can_notify(guild_id: u64, user_id: u64, game_id: &str) -> bool {
    let key = (guild_id, user_id, game_id.to_string());
    let now = Instant::now();
    let cooldown = Duration::from_secs(MENTION_COOLDOWN_SECS);
    let mut map = match MENTION_COOLDOWN.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(ts) = map.get(&key) {
        if now.duration_since(*ts) < cooldown {
            return false;
        }
    }
    map.insert(key, now);
    // Cleanup opportuniste pour borner la memoire.
    if map.len() > 500 {
        map.retain(|_, ts| now.duration_since(*ts) < cooldown);
    }
    true
}

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

        // Detecter les mentions de jeux (#NomDuJeu) et dedupliquer pour
        // eviter qu'un message contenant `#X #X #X` declenche plusieurs
        // lookups API et plusieurs notifications.
        let mut mentions = detector::extract_game_mentions(&msg.content);
        mentions.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        mentions.dedup_by(|a, b| a.to_lowercase() == b.to_lowercase());
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

            // Anti-spam ping : un meme user ne peut declencher qu'une
            // notification par jeu toutes les MENTION_COOLDOWN_SECS.
            if !can_notify(guild_id.get(), msg.author.id.get(), &game.id) {
                continue;
            }

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
