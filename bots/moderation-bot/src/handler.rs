use std::time::Instant;

use dashmap::DashMap;
use serenity::async_trait;
use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage};
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::heartbeat::{ApiClientKey, register_guilds};

use crate::api_client::{ApiClient, ModerationAction};
use crate::commands;
use crate::reason_templates;

/// Cle TypeMap pour le client API specifique a la moderation.
pub struct ModerationApiKey;
impl TypeMapKey for ModerationApiKey {
    type Value = std::sync::Arc<ApiClient>;
}

/// Actions en attente d'approbation (mode apprenti).
#[allow(dead_code)]
pub struct PendingAction {
    pub action: ModerationAction,
    pub moderator_id: String,
    pub proposed_at: Instant,
}

pub struct PendingActionsKey;
impl TypeMapKey for PendingActionsKey {
    type Value = DashMap<String, PendingAction>;
}

pub const APPROVE_PREFIX: &str = "sentinel_mod_approve_";
pub const REJECT_PREFIX: &str = "sentinel_mod_reject_";

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Moderation bot connecte");

        register_guilds(&ctx, &ready).await;

        // Vider les anciennes commandes globales (evite les doublons).
        let _ = serenity::model::application::Command::set_global_commands(&ctx.http, vec![]).await;

        // Enregistrement par guild (instantane) au lieu de global (jusqu'a 1h).
        let cmds = commands::all();
        for guild in &ready.guilds {
            match guild.id.set_commands(&ctx.http, cmds.clone()).await {
                Ok(_) => info!(guild_id = %guild.id, "Slash commands enregistrees pour la guild"),
                Err(e) => error!(error = %e, guild_id = %guild.id, "Echec enregistrement slash commands"),
            }
        }
        info!("Slash commands : warn, unwarn, mute, unmute, ban, unban, history, note, call, context, appeal, export, massmute, massban");

        // Phase 5B : consumer durable Redis Streams avec XREADGROUP + XACK.
        // Replay au redemarrage des events emis pendant que le bot etait down.
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            let consumer = sentinel_shared::event_bus::default_consumer_name();
            sentinel_shared::event_bus::listen_stream_group(
                "moderation-bot".to_string(),
                consumer,
                move |payload| {
                    let ctx = ctx_clone.clone();
                    async move {
                        handle_redis_moderation_event(&ctx, &payload).await;
                    }
                },
            )
            .await;
        });
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                let cmd_name = command.data.name.clone();
                let moderator = command.user.name.clone();
                let guild_id = command.guild_id.map(|g| g.to_string()).unwrap_or_default();

                if !guild_id.is_empty() {
                    let data = ctx.data.read().await;
                    if let Some(api) = data.get::<ApiClientKey>() {
                        let config = match api.get_guild_config(&guild_id).await {
                            Ok(config) => config,
                            Err(e) => {
                                warn!(error = %e, "Failed to fetch guild config");
                                std::collections::HashMap::new()
                            }
                        };
                        if !BaseApiClient::config_bool(&config, "enabled", true) {
                            return;
                        }
                    }
                }

                match cmd_name.as_str() {
                    "warn" => commands::warn::handle(&ctx, &command).await,
                    "unwarn" => commands::unwarn::handle(&ctx, &command).await,
                    "mute" => commands::mute::handle(&ctx, &command).await,
                    "unmute" => commands::mute::handle_unmute(&ctx, &command).await,
                    "ban" => commands::ban::handle(&ctx, &command).await,
                    "unban" => commands::ban::handle_unban(&ctx, &command).await,
                    "history" => commands::history::handle(&ctx, &command).await,
                    "note" => commands::notes::handle(&ctx, &command).await,
                    "call" => commands::call::handle(&ctx, &command).await,
                    "context" => commands::context::handle(&ctx, &command).await,
                    "appeal" => commands::appeal::handle(&ctx, &command).await,
                    "expirations" => commands::expirations::handle(&ctx, &command).await,
                    "compare" => commands::compare::handle(&ctx, &command).await,
                    "modstats" => commands::modstats::handle(&ctx, &command).await,
                    "evidence" => commands::evidence::handle(&ctx, &command).await,
                    "review" => commands::review::handle(&ctx, &command).await,
                    "template" => commands::template::handle(&ctx, &command).await,
                    "transcript" => commands::transcript::handle(&ctx, &command).await,
                    "export" => commands::export::handle(&ctx, &command).await,
                    "massmute" => commands::mass::handle_massmute(&ctx, &command).await,
                    "massban" => commands::mass::handle_massban(&ctx, &command).await,
                    _ => {}
                }

                let data = ctx.data.read().await;
                if let Some(api) = data.get::<ApiClientKey>() {
                    api.send_log("info", &guild_id, &format!("Commande /{} executee par {}", cmd_name, moderator));
                }
            }
            Interaction::Component(component) => {
                let custom_id = &component.data.custom_id;

                if custom_id.starts_with(commands::unwarn::UNWARN_PREFIX) {
                    commands::unwarn::handle_button(&ctx, &component).await;
                } else if custom_id.starts_with(commands::call::CALL_CLOSE_PREFIX) {
                    commands::call::handle_close(&ctx, &component).await;
                } else if custom_id.starts_with(commands::appeal::APPEAL_PREFIX) {
                    commands::appeal::handle_appeal_button(&ctx, &component).await;
                } else if custom_id.starts_with(APPROVE_PREFIX) {
                    handle_approve(&ctx, &component).await;
                } else if custom_id.starts_with(REJECT_PREFIX) {
                    handle_reject(&ctx, &component).await;
                } else if custom_id.starts_with(crate::risk_check::CONFIRM_PREFIX) {
                    handle_risky_confirm(&ctx, &component).await;
                } else if custom_id.starts_with(crate::risk_check::CANCEL_PREFIX) {
                    handle_risky_cancel(&ctx, &component).await;
                }
            }
            Interaction::Autocomplete(autocomplete) => {
                // Autocomplete pour le champ "reason" des commandes warn, mute, ban
                let cmd_name = &autocomplete.data.name;
                if matches!(cmd_name.as_str(), "warn" | "mute" | "ban") {
                    handle_reason_autocomplete(&ctx, &autocomplete).await;
                }
            }
            _ => {}
        }
    }
}

/// Gere l'autocomplete des raisons avec les templates configurees.
async fn handle_reason_autocomplete(
    ctx: &Context,
    autocomplete: &serenity::model::application::CommandInteraction,
) {
    let guild_id = autocomplete.guild_id.map(|g| g.to_string()).unwrap_or_default();

    // Trouver la valeur actuelle du champ "reason" (le champ focused dans l'autocomplete)
    let current_input = autocomplete.data.options.iter()
        .find(|o| o.name == "reason")
        .and_then(|o| match &o.value {
            serenity::all::CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("");

    // Lire les templates depuis la config
    let templates_raw = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let gc = match base.get_guild_config(&guild_id).await {
                Ok(config) => config,
                Err(e) => {
                    warn!(error = %e, "Failed to fetch guild config for reason templates");
                    std::collections::HashMap::new()
                }
            };
            BaseApiClient::config_or(&gc, "reason_templates", "")
        } else {
            String::new()
        }
    };

    let templates = reason_templates::parse_templates(&templates_raw);
    let filtered = reason_templates::filter_templates(&templates, current_input);

    let choices: Vec<serenity::builder::AutocompleteChoice> = filtered
        .iter()
        .map(|t| serenity::builder::AutocompleteChoice::new(&t.label, serde_json::Value::String(t.reason.clone())))
        .collect();

    let response = serenity::builder::CreateAutocompleteResponse::new().set_choices(choices);

    if let Err(e) = autocomplete.create_response(
        &ctx.http,
        serenity::all::CreateInteractionResponse::Autocomplete(response),
    ).await {
        warn!(error = %e, "Failed to send autocomplete response");
    }
}

/// Gere le clic "Approuver" sur une action en attente (mode apprenti).
async fn handle_approve(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
    let pending_id = match component.data.custom_id.strip_prefix(APPROVE_PREFIX) {
        Some(id) => id.to_string(),
        None => return,
    };

    let data = ctx.data.read().await;
    let pending_actions = match data.get::<PendingActionsKey>() {
        Some(p) => p,
        None => return,
    };

    // Verifier que l'approbateur a la permission MODERATE_MEMBERS
    if let Some(guild_id) = component.guild_id {
        if let Ok(member) = guild_id.member(&ctx.http, component.user.id).await {
            #[allow(deprecated)]
            if let Ok(perms) = member.permissions(&ctx.cache) {
                if !perms.moderate_members() {
                    let response = serenity::all::CreateInteractionResponse::Message(
                        serenity::all::CreateInteractionResponseMessage::new()
                            .content("Tu n'as pas la permission d'approuver des actions.")
                            .ephemeral(true),
                    );
                    let _ = component.create_response(&ctx.http, response).await;
                    return;
                }
            }
        }
    }

    let pending = match pending_actions.remove(&pending_id) {
        Some((_, p)) => p,
        None => {
            let response = serenity::all::CreateInteractionResponse::Message(
                serenity::all::CreateInteractionResponseMessage::new()
                    .content("Cette action n'est plus en attente.")
                    .ephemeral(true),
            );
            if let Err(e) = component.create_response(&ctx.http, response).await {
                warn!(error = %e, "Failed to send pending-action-not-found response");
            }
            return;
        }
    };

    // Cleanup : supprimer les actions en attente > 24h
    if pending_actions.len() > 50 {
        let now = Instant::now();
        pending_actions.retain(|_, p| now.duration_since(p.proposed_at).as_secs() < 86400);
    }

    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => return,
    };

    // Executer l'action approuvee
    match api.log_action(&pending.action).await {
        Ok(_) => {
            // Persister l'approbation via l'API
            api.resolve_pending_action(&pending_id, "approved", &component.user.id.to_string()).await;

            // Event temps reel pour le desktop
            if let Some(base) = data.get::<ApiClientKey>() {
                base.publish_event("pending_action_resolved", serde_json::json!({
                    "action_id": pending_id,
                    "status": "approved",
                    "reviewed_by": component.user.id.to_string(),
                    "action_type": pending.action.action_type,
                    "target_id": pending.action.target_id,
                }));
            }

            let response = serenity::all::CreateInteractionResponse::Message(
                serenity::all::CreateInteractionResponseMessage::new()
                    .content(format!(
                        "Action approuvee par <@{}>. {} executee sur <@{}>.",
                        component.user.id, pending.action.action_type, pending.action.target_id
                    )),
            );
            if let Err(e) = component.create_response(&ctx.http, response).await {
                warn!(error = %e, "Failed to send approve response");
            }
            info!(
                approver = %component.user.name,
                action = %pending.action.action_type,
                target = %pending.action.target_name,
                "Action apprenti approuvee"
            );
        }
        Err(e) => {
            error!(error = %e, "Erreur execution action approuvee");
        }
    }
}

/// Gere le clic "Rejeter" sur une action en attente (mode apprenti).
async fn handle_reject(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
    let pending_id = match component.data.custom_id.strip_prefix(REJECT_PREFIX) {
        Some(id) => id.to_string(),
        None => return,
    };

    let data = ctx.data.read().await;
    let pending_actions = match data.get::<PendingActionsKey>() {
        Some(p) => p,
        None => return,
    };

    if let Some((_, pending)) = pending_actions.remove(&pending_id) {
        // Persister le rejet via l'API
        if let Some(api) = data.get::<ModerationApiKey>() {
            api.resolve_pending_action(&pending_id, "rejected", &component.user.id.to_string()).await;
        }

        let response = serenity::all::CreateInteractionResponse::Message(
            serenity::all::CreateInteractionResponseMessage::new()
                .content(format!(
                    "Action rejetee par <@{}>. {} sur <@{}> annulee.",
                    component.user.id, pending.action.action_type, pending.action.target_id
                )),
        );
        if let Err(e) = component.create_response(&ctx.http, response).await {
            warn!(error = %e, "Failed to send reject response");
        }
        info!(
            rejector = %component.user.name,
            action = %pending.action.action_type,
            target = %pending.action.target_name,
            "Action apprenti rejetee"
        );
    }
}

/// Dispatche un event Redis vers le bon handler selon son type.
///
/// Events supportes :
/// - `moderation_action` : log embed dans le salon de logs (flux desktop → bot)
/// - `sanction_expiry_reminder` : DM au moderateur qui a pose la sanction temporaire
///   (emis par `moderation-worker::send_reminders` Phase 4 B.2)
/// - `appeal_sla_escalated` : ping des moderateurs seniors dans le salon de logs
///   (emis par `appeal-sla-worker` Phase 6A)
async fn handle_redis_moderation_event(ctx: &Context, payload: &str) {
    let event: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return,
    };

    let event_type = event.get("event").and_then(|e| e.as_str()).unwrap_or("");
    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };

    match event_type {
        "moderation_action" => handle_moderation_action_log(ctx, data).await,
        "sanction_expiry_reminder" => handle_sanction_expiry_reminder(ctx, data).await,
        "appeal_sla_escalated" => handle_appeal_sla_escalated(ctx, data).await,
        _ => {
            // Autres event types non geres par moderation-bot (ex: ticket_sla_updated)
        }
    }
}

/// Poste un embed dans le salon de logs pour une action de moderation.
async fn handle_moderation_action_log(ctx: &Context, data: &serde_json::Value) {
    let action_type = data.get("action_type").and_then(|v| v.as_str()).unwrap_or("");
    let target_id = data.get("target_id").and_then(|v| v.as_str()).unwrap_or("");
    let target_name = data.get("target_name").and_then(|v| v.as_str()).unwrap_or(target_id);
    let moderator = data.get("moderator_name").and_then(|v| v.as_str()).unwrap_or("Inconnu");
    let guild_id = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let reason_raw = data.get("reason").and_then(|v| v.as_str()).unwrap_or("Aucune raison specifiee");
    let reason: String = reason_raw.chars().take(500).collect::<String>()
        .replace("```", "` ` `")
        .replace("||", "| |")
        .replace('@', "@\u{200b}");

    if guild_id.is_empty() {
        return;
    }

    let log_channel_id = {
        let ctx_data = ctx.data.read().await;
        if let Some(base) = ctx_data.get::<ApiClientKey>() {
            let config = match base.get_guild_config(guild_id).await {
                Ok(config) => config,
                Err(e) => {
                    warn!(error = %e, "Failed to fetch guild config for log channel");
                    std::collections::HashMap::new()
                }
            };
            config.get("log_channel_id")
                .and_then(|v| v.parse::<u64>().ok())
        } else {
            None
        }
    };

    let log_channel = match log_channel_id {
        Some(id) if id > 0 => ChannelId::new(id),
        _ => return,
    };

    let (icon, action_label, color) = match action_type {
        "ban_permanent" => ("\u{1f6ab}", "Bannissement permanent", 0xdc2626u32),
        "ban_temp" => ("\u{1f6ab}", "Bannissement temporaire", 0xdc2626),
        "ban" => ("\u{1f6ab}", "Bannissement", 0xdc2626),
        "unban" => ("\u{1f513}", "Debannissement", 0x2ecc71),
        "mute" | "mute_temp" => ("\u{1f507}", "Mute", 0xef4444),
        "unmute" => ("\u{1f50a}", "Unmute", 0x2ecc71),
        "warn" => ("\u{26a0}\u{fe0f}", "Avertissement", 0xf59e0b),
        "kick" => ("\u{1f462}", "Expulsion", 0xf97316),
        "delete" => ("\u{1f5d1}\u{fe0f}", "Message supprime", 0xf97316),
        "call" => ("\u{1f4de}", "Convocation", 0x3498db),
        _ => ("\u{1f4cb}", "Action de moderation", 0x95a5a6),
    };

    let source = if moderator == "Desktop App" { "Desktop App" } else { moderator };

    let (avatar_url, real_name) = if let Ok(uid) = target_id.parse::<u64>() {
        let user_id = serenity::model::id::UserId::new(uid);
        match user_id.to_user(&ctx.http).await {
            Ok(user) => (Some(user.face()), user.name.clone()),
            Err(_) => (None, target_name.to_string()),
        }
    } else {
        (None, target_name.to_string())
    };

    let mut embed = CreateEmbed::new()
        .author(serenity::builder::CreateEmbedAuthor::new(
            format!("{} {}", icon, action_label),
        ))
        .title("Moderation - Action manuelle")
        .color(color)
        .field(
            "\u{1f464} Utilisateur",
            format!("<@{}> (`{}`)", target_id, real_name),
            true,
        )
        .field(
            "\u{1f6e1}\u{fe0f} Moderateur",
            source,
            true,
        )
        .field(
            "\u{2699}\u{fe0f} Action",
            action_label,
            true,
        )
        .field(
            "\u{1f4dd} Raison",
            format!("```{}```", reason),
            false,
        )
        .footer(CreateEmbedFooter::new(
            format!("Cible: {} | Moderateur: {} \u{2022} {}", target_id, source, action_type),
        ))
        .timestamp(serenity::model::Timestamp::now());

    if let Some(url) = avatar_url {
        embed = embed.thumbnail(url);
    }

    let msg = CreateMessage::new().embed(embed);
    if let Err(e) = log_channel.send_message(&ctx.http, msg).await {
        error!(error = %e, "Erreur envoi log moderation dans Discord");
    }
}

/// MOD #1 — Envoie un DM au moderateur 1h avant l'expiration d'une sanction temporaire.
///
/// Event `sanction_expiry_reminder` emis par `moderation-worker::send_reminders`
/// (Phase 4 B.2) quand `remind_at <= NOW()`. Le payload contient l'ID du moderateur,
/// de la cible, la raison, et l'heure d'expiration.
async fn handle_sanction_expiry_reminder(ctx: &Context, data: &serde_json::Value) {
    let moderator_id = data.get("moderator_id").and_then(|v| v.as_str()).unwrap_or("");
    let target_name = data.get("target_name").and_then(|v| v.as_str()).unwrap_or("?");
    let action_type = data.get("action_type").and_then(|v| v.as_str()).unwrap_or("?");
    let reason = data.get("reason").and_then(|v| v.as_str()).unwrap_or("Aucune raison");
    let minutes_left = data.get("minutes_left").and_then(|v| v.as_i64()).unwrap_or(0);
    let target_id = data.get("target_id").and_then(|v| v.as_str()).unwrap_or("");

    let mod_uid = match moderator_id.parse::<u64>() {
        Ok(u) => u,
        Err(_) => {
            warn!(moderator_id, "sanction_expiry_reminder: moderator_id invalide");
            return;
        }
    };

    let user_id = serenity::model::id::UserId::new(mod_uid);
    let user = match user_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(e) => {
            warn!(error = %e, moderator_id, "sanction_expiry_reminder: user fetch failed");
            return;
        }
    };

    let action_label = match action_type {
        "mute_temp" => "Mute temporaire",
        "ban_temp" => "Bannissement temporaire",
        _ => "Sanction temporaire",
    };

    let reason_trunc: String = reason.chars().take(300).collect::<String>()
        .replace("```", "` ` `");

    let embed = CreateEmbed::new()
        .title("\u{23f0} Rappel — Sanction proche de l'expiration")
        .color(0xf59e0b)
        .field("\u{2699}\u{fe0f} Action", action_label, true)
        .field(
            "\u{1f464} Cible",
            format!("<@{}> (`{}`)", target_id, target_name),
            true,
        )
        .field(
            "\u{23f3} Temps restant",
            format!("{} minutes", minutes_left),
            true,
        )
        .field("\u{1f4dd} Raison", format!("```{}```", reason_trunc), false)
        .footer(CreateEmbedFooter::new(
            "Rappel automatique envoye par le moderation-bot",
        ))
        .timestamp(serenity::model::Timestamp::now());

    let dm = CreateMessage::new().embed(embed);
    let channel = match user.create_dm_channel(&ctx.http).await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, moderator_id, "sanction_expiry_reminder: DM channel failed");
            return;
        }
    };
    if let Err(e) = channel.send_message(&ctx.http, dm).await {
        warn!(error = %e, moderator_id, "sanction_expiry_reminder: DM send failed");
    } else {
        info!(
            moderator_id,
            target_name, action_type, minutes_left, "DM rappel expiration envoye"
        );
    }
}

/// MOD #1 — Signale une escalade SLA d'appel de sanction dans le salon de logs.
///
/// Event `appeal_sla_escalated` emis par `appeal-sla-worker` (Phase 6A) quand un
/// ticket d'appel depasse le SLA d'escalade sans premiere reponse. Le bot poste
/// un embed rouge dans le salon de logs pour alerter les moderateurs seniors.
async fn handle_appeal_sla_escalated(ctx: &Context, data: &serde_json::Value) {
    let guild_id = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let ticket_id = data.get("ticket_id").and_then(|v| v.as_str()).unwrap_or("?");
    let author_id = data.get("author_id").and_then(|v| v.as_str()).unwrap_or("");
    let author_name = data.get("author_name").and_then(|v| v.as_str()).unwrap_or("?");
    let title = data.get("title").and_then(|v| v.as_str()).unwrap_or("Appel de sanction");
    let age_minutes = data.get("age_minutes").and_then(|v| v.as_i64()).unwrap_or(0);
    let sla_escalation_minutes = data
        .get("sla_escalation_minutes")
        .and_then(|v| v.as_i64())
        .unwrap_or(60);

    if guild_id.is_empty() {
        return;
    }

    let log_channel_id = {
        let ctx_data = ctx.data.read().await;
        if let Some(base) = ctx_data.get::<ApiClientKey>() {
            let config = match base.get_guild_config(guild_id).await {
                Ok(c) => c,
                Err(e) => {
                    warn!(error = %e, "Failed to fetch guild config for appeal SLA");
                    return;
                }
            };
            config.get("log_channel_id").and_then(|v| v.parse::<u64>().ok())
        } else {
            return;
        }
    };

    let log_channel = match log_channel_id {
        Some(id) if id > 0 => ChannelId::new(id),
        _ => return,
    };

    let short_id = ticket_id.chars().take(8).collect::<String>();
    let title_trunc: String = title.chars().take(200).collect::<String>()
        .replace("```", "` ` `");

    let embed = CreateEmbed::new()
        .title("\u{1f6a8} Escalade SLA — Appel de sanction en attente")
        .description(format!(
            "L'appel de sanction `{}` n'a pas recu de premiere reponse depuis **{} minutes** (SLA: {} min). \
             Un moderateur senior doit l'examiner.",
            short_id, age_minutes, sla_escalation_minutes
        ))
        .color(0xdc2626)
        .field(
            "\u{1f464} Auteur",
            format!("<@{}> (`{}`)", author_id, author_name),
            true,
        )
        .field("\u{1f3ab} Ticket", format!("`{}`", short_id), true)
        .field("\u{1f4ac} Titre", format!("```{}```", title_trunc), false)
        .footer(CreateEmbedFooter::new(
            "Escalade automatique emise par appeal-sla-worker",
        ))
        .timestamp(serenity::model::Timestamp::now());

    let msg = CreateMessage::new().content("@here").embed(embed);
    if let Err(e) = log_channel.send_message(&ctx.http, msg).await {
        warn!(error = %e, ticket_id, "appeal_sla_escalated: log send failed");
    } else {
        info!(ticket_id, guild_id, age_minutes, "Escalade appel SLA postee dans logs");
    }
}

/// MOD #4 — Clic "Confirmer" sur une action deferee (cible a risque).
///
/// Recupere l'action en attente dans `RiskyPendingKey`, l'execute via le
/// dispatcher approprie (ban/mute), puis purge l'entry.
async fn handle_risky_confirm(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
) {
    let pending_id = match component
        .data
        .custom_id
        .strip_prefix(crate::risk_check::CONFIRM_PREFIX)
    {
        Some(id) => id.to_string(),
        None => return,
    };

    let pending = {
        let data = ctx.data.read().await;
        let store = match data.get::<crate::risk_check::RiskyPendingKey>() {
            Some(s) => s,
            None => return,
        };
        crate::risk_check::purge_expired(store);
        store.remove(&pending_id).map(|(_, p)| p)
    };

    let pending = match pending {
        Some(p) => p,
        None => {
            let response = serenity::all::CreateInteractionResponse::Message(
                serenity::all::CreateInteractionResponseMessage::new()
                    .content("Cette confirmation a expire ou n'est plus disponible.")
                    .ephemeral(true),
            );
            if let Err(e) = component.create_response(&ctx.http, response).await {
                warn!(error = %e, "Failed to send risky expired response");
            }
            return;
        }
    };

    // ACK immediat au clic pour eviter le timeout Discord (3s)
    let ack = serenity::all::CreateInteractionResponse::UpdateMessage(
        serenity::all::CreateInteractionResponseMessage::new()
            .content(format!(
                "\u{2705} Execution confirmee pour `{}`.",
                pending.target_name
            ))
            .embeds(vec![])
            .components(vec![]),
    );
    if let Err(e) = component.create_response(&ctx.http, ack).await {
        warn!(error = %e, "Failed to ACK risky confirm");
    }

    // Parser le guild_id + target_id
    let guild_id = match pending.guild_id.parse::<u64>() {
        Ok(id) => serenity::model::id::GuildId::new(id),
        Err(_) => return,
    };
    let target_uid = match pending.target_id.parse::<u64>() {
        Ok(id) => id,
        Err(_) => return,
    };
    let target_user = match serenity::model::id::UserId::new(target_uid)
        .to_user(&ctx.http)
        .await
    {
        Ok(u) => u,
        Err(e) => {
            warn!(error = %e, "risky confirm: user fetch failed");
            return;
        }
    };

    match pending.kind {
        crate::risk_check::PendingKind::Ban {
            delete_message_days,
            is_permanent,
        } => {
            commands::ban::execute_ban(
                ctx,
                pending.channel_id.clone(),
                pending.moderator_id.clone(),
                pending.moderator_name.clone(),
                guild_id,
                &target_user,
                &pending.reason,
                pending.duration_secs,
                &pending.duration_label,
                is_permanent,
                delete_message_days,
                None,
            )
            .await;
        }
        crate::risk_check::PendingKind::Mute { timeout_secs } => {
            let is_permanent = pending.duration_secs.is_none();
            commands::mute::execute_mute(
                ctx,
                pending.channel_id.clone(),
                pending.moderator_id.clone(),
                pending.moderator_name.clone(),
                guild_id,
                &target_user,
                &pending.reason,
                pending.duration_secs,
                &pending.duration_label,
                is_permanent,
                timeout_secs,
                None,
            )
            .await;
        }
    }
}

/// MOD #4 — Clic "Annuler" sur une action deferee.
async fn handle_risky_cancel(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
) {
    let pending_id = match component
        .data
        .custom_id
        .strip_prefix(crate::risk_check::CANCEL_PREFIX)
    {
        Some(id) => id.to_string(),
        None => return,
    };

    let removed = {
        let data = ctx.data.read().await;
        if let Some(store) = data.get::<crate::risk_check::RiskyPendingKey>() {
            store.remove(&pending_id).is_some()
        } else {
            false
        }
    };

    let content = if removed {
        "\u{274c} Action annulee."
    } else {
        "Cette confirmation a deja expire."
    };

    let response = serenity::all::CreateInteractionResponse::UpdateMessage(
        serenity::all::CreateInteractionResponseMessage::new()
            .content(content)
            .embeds(vec![])
            .components(vec![]),
    );
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Failed to send risky cancel response");
    }
}
