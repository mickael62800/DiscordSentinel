use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use tracing::{error, info, warn};

use serenity::builder::CreateMessage;

use sentinel_shared::embeds::success_embed;

use sentinel_shared::heartbeat::register_guilds;

use crate::api_client::ApiClient;
use crate::commands;
use crate::tracker::StatsTracker;

/// Cle TypeMap pour le client API specifique au stats-bot.
pub struct StatsApiKey;

impl TypeMapKey for StatsApiKey {
    type Value = ApiClient;
}

/// Cle pour acceder au StatsTracker dans le TypeMap.
pub struct TrackerKey;

impl TypeMapKey for TrackerKey {
    type Value = StatsTracker;
}

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Stats bot connecte");

        // Enregistrer les guilds via le shared helper
        register_guilds(&ctx, &ready).await;

        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::all(),
        )
        .await
        {
            error!(error = %e, "Impossible d'enregistrer les slash commands");
        } else {
            info!("Slash commands enregistrees");
        }
    }

    /// Compteur de messages — envoie au backend via l'API.
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let guild_id = match msg.guild_id {
            Some(id) => id,
            None => return,
        };

        let data = ctx.data.read().await;

        // Track localement (fallback pour les commandes)
        if let Some(tracker) = data.get::<TrackerKey>() {
            tracker.record_message(guild_id.get(), msg.author.id.get()).await;
        }

        // Envoyer au backend
        if let Some(api) = data.get::<StatsApiKey>() {
            if let Err(e) = api
                .record_messages(
                    &guild_id.to_string(),
                    &msg.author.id.to_string(),
                    &msg.author.name,
                    1,
                )
                .await
            {
                warn!(error = %e, "Impossible d'envoyer les stats messages au backend");
            }

            // Ajouter de l'XP (15 XP par message par defaut, gere cote API)
            match api
                .add_xp(
                    &guild_id.to_string(),
                    &msg.author.id.to_string(),
                    &msg.author.name,
                    15,
                )
                .await
            {
                Ok(result) => {
                    if result.leveled_up {
                        let embed = success_embed("\u{1f389} LEVEL UP !")
                            .description(format!(
                                "<@{}> est maintenant **niveau {}** !",
                                msg.author.id, result.user.level
                            ))
                            .thumbnail(msg.author.face());
                        let _ = msg.channel_id.send_message(
                            &ctx.http,
                            CreateMessage::new().embed(embed),
                        ).await;

                        // Attribuer le role recompense si configure
                        if let Some(role_id_str) = &result.reward_role_id {
                            if let Ok(role_id) = role_id_str.parse::<u64>() {
                                if let Ok(member) = guild_id.member(&ctx.http, msg.author.id).await {
                                    let _ = member.add_role(&ctx.http, serenity::model::id::RoleId::new(role_id)).await;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!(error = %e, "Erreur ajout XP message");
                }
            }
        }
    }

    /// Suivi des sessions vocales (join/leave).
    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        let guild_id = match new.guild_id {
            Some(id) => id,
            None => return,
        };

        let user_id = new.user_id;
        let data = ctx.data.read().await;

        let was_in_voice = old.as_ref().and_then(|s| s.channel_id).is_some();
        let is_in_voice = new.channel_id.is_some();

        let tracker = data.get::<TrackerKey>();
        let api = data.get::<StatsApiKey>();

        match (was_in_voice, is_in_voice) {
            (false, true) => {
                // Rejoint un salon vocal
                if let Some(tracker) = tracker {
                    tracker.voice_join(guild_id.get(), user_id.get()).await;
                }
            }
            (true, false) => {
                // Quitte le salon vocal — calculer la duree et envoyer au backend
                if let Some(tracker) = tracker {
                    let seconds = tracker.voice_leave(guild_id.get(), user_id.get()).await;

                    if seconds > 0 {
                        // Recuperer le nom d'utilisateur
                        let username = user_id
                            .to_user(&ctx.http)
                            .await
                            .map(|u| u.name)
                            .unwrap_or_else(|_| user_id.to_string());

                        // Recuperer le channel_id et channel_name du salon quitte
                        let (channel_id_str, channel_name) = if let Some(old_state) = &old {
                            if let Some(ch_id) = old_state.channel_id {
                                let name = ch_id.to_channel(&ctx.http).await
                                    .ok()
                                    .and_then(|c| c.guild())
                                    .map(|c| c.name.clone())
                                    .unwrap_or_default();
                                (ch_id.to_string(), name)
                            } else {
                                (String::new(), String::new())
                            }
                        } else {
                            (String::new(), String::new())
                        };

                        if let Some(api) = api {
                            if let Err(e) = api
                                .record_voice(
                                    &guild_id.to_string(),
                                    &user_id.to_string(),
                                    &username,
                                    seconds,
                                    &channel_id_str,
                                    &channel_name,
                                )
                                .await
                            {
                                warn!(error = %e, "Impossible d'envoyer les stats vocal au backend");
                            }

                            // Ajouter XP vocal (5 XP par minute)
                            let xp_amount = (seconds / 60) as i64 * 5;
                            if xp_amount > 0 {
                                let _ = api
                                    .add_xp(
                                        &guild_id.to_string(),
                                        &user_id.to_string(),
                                        &username,
                                        xp_amount,
                                    )
                                    .await;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Gestion des slash commands.
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            match command.data.name.as_str() {
                "stats" => commands::stats::handle(&ctx, &command).await,
                "level" => commands::level::handle(&ctx, &command).await,
                _ => {}
            }
        }
    }
}
