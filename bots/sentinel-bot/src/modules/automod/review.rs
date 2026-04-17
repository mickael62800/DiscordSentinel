//! Review mode : cartes de validation moderateur + handlers des boutons.

use serenity::model::channel::Message;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::embeds::{warn_embed, moderate_embed, danger_embed, critical_embed};
use sentinel_shared::heartbeat::ApiClientKey;

use super::api_client::Action;
use super::config::{build_embed_colors, EmbedColors};
use super::detectors;
use super::{AM_PREFIX, DEFAULT_MUTE_DURATION_SECS};

/// Envoie une carte de review dans le salon de logs au lieu d'appliquer
/// l'action directement. Les moderateurs cliquent sur un bouton pour
/// valider ou ajuster la severite.
pub(super) async fn send_review_card(
    ctx: &Context,
    msg: &Message,
    suggested_action: &Action,
    reason: &str,
    score: f64,
    flags: &detectors::DetectionFlags,
    review_channel_id: u64,
    colors: &EmbedColors,
) {
    let guild_id = msg.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let channel_id = msg.channel_id.to_string();
    let message_id = msg.id.to_string();
    let user_id = msg.author.id.to_string();
    let content_preview = sanitize_embed_content(&msg.content, 500);

    let action_label = match suggested_action {
        Action::Warn => "Avertissement",
        Action::Delete => "Suppression",
        Action::Mute => "Mute",
        Action::Ban => "Bannissement",
        Action::None => return,
    };

    let action_color = match suggested_action {
        Action::Warn => colors.warn,
        Action::Delete => colors.delete,
        Action::Mute => colors.mute,
        Action::Ban => colors.ban,
        Action::None => 0x95a5a6,
    };

    let mut flag_parts = Vec::new();
    if flags.spam { flag_parts.push("Spam"); }
    if flags.insult { flag_parts.push("Insulte"); }
    if flags.link { flag_parts.push("Lien"); }
    if flags.phishing { flag_parts.push("Phishing"); }
    let flags_str = if flag_parts.is_empty() { "Aucun".to_string() } else { flag_parts.join(", ") };

    let embed = serenity::builder::CreateEmbed::new()
        .title(format!("AutoMod -- Action suggeree : {}", action_label))
        .color(action_color)
        .field("Utilisateur", format!("<@{}> (`{}`)", user_id, msg.author.name), true)
        .field("Salon", format!("<#{}>", channel_id), true)
        .field("Score IA", format!("{:.0}%", score * 100.0), true)
        .field("Message original", format!("```{}```", content_preview), false)
        .field("Raison IA", reason, false)
        .field("Flags detectes", &flags_str, true)
        .thumbnail(msg.author.face())
        .footer(serenity::builder::CreateEmbedFooter::new(
            "AutoMod Review | Cliquez pour valider ou ajuster",
        ))
        .timestamp(serenity::model::Timestamp::now());

    // Suffixe commun pour les custom_id
    let id_suffix = format!("{}:{}:{}:{}", guild_id, channel_id, message_id, user_id);

    // Bouton principal (action suggeree) + ajustements + ignorer.
    let suggested_char = action_char(suggested_action);

    let btn_apply = serenity::builder::CreateButton::new(format!("am_{}:{}", suggested_char, id_suffix))
        .label(format!("Appliquer ({})", action_label))
        .style(serenity::all::ButtonStyle::Success);

    let btn_ignore = serenity::builder::CreateButton::new(format!("am_i:{}", id_suffix))
        .label("Ignorer")
        .style(serenity::all::ButtonStyle::Secondary);

    // Rangee 2 : ajustements de severite (sans doublon avec le bouton principal)
    let mut adjust_buttons = Vec::new();
    if suggested_char != 'w' {
        adjust_buttons.push(
            serenity::builder::CreateButton::new(format!("am_w:{}", id_suffix))
                .label("Warn")
                .style(serenity::all::ButtonStyle::Secondary),
        );
    }
    if suggested_char != 'd' {
        adjust_buttons.push(
            serenity::builder::CreateButton::new(format!("am_d:{}", id_suffix))
                .label("Delete")
                .style(serenity::all::ButtonStyle::Secondary),
        );
    }
    if suggested_char != 'm' {
        adjust_buttons.push(
            serenity::builder::CreateButton::new(format!("am_m:{}", id_suffix))
                .label("Mute")
                .style(serenity::all::ButtonStyle::Danger),
        );
    }

    let row1 = serenity::builder::CreateActionRow::Buttons(vec![btn_apply, btn_ignore]);
    let row2 = serenity::builder::CreateActionRow::Buttons(adjust_buttons);

    let builder = serenity::builder::CreateMessage::new()
        .embed(embed)
        .components(vec![row1, row2]);

    match serenity::model::id::ChannelId::new(review_channel_id)
        .send_message(&ctx.http, builder)
        .await
    {
        Ok(_) => info!(
            user = %msg.author.name,
            channel = %msg.channel_id,
            action = %action_label,
            review_channel = review_channel_id,
            "Carte de review envoyee"
        ),
        Err(e) => error!(
            error = %e,
            review_channel = review_channel_id,
            "Echec envoi carte de review automod -- verifier que le bot a acces au salon"
        ),
    }
}

fn action_char(action: &Action) -> char {
    match action {
        Action::Warn => 'w',
        Action::Delete => 'd',
        Action::Mute => 'm',
        Action::Ban => 'b',
        Action::None => 'i',
    }
}

fn char_to_action(c: char) -> Action {
    match c {
        'w' => Action::Warn,
        'd' => Action::Delete,
        'm' => Action::Mute,
        'b' => Action::Ban,
        _ => Action::None,
    }
}

/// Handler des boutons de review. Parse le custom_id, execute l'action
/// choisie par le moderateur, et met a jour la carte.
pub(super) async fn handle_review_button(ctx: &Context, component: &serenity::model::application::ComponentInteraction) {
    let has_perm = component
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| {
            p.contains(serenity::all::Permissions::MODERATE_MEMBERS)
                || p.contains(serenity::all::Permissions::MANAGE_MESSAGES)
                || p.contains(serenity::all::Permissions::ADMINISTRATOR)
        })
        .unwrap_or(false);

    if !has_perm {
        let _ = component
            .create_response(
                &ctx.http,
                serenity::builder::CreateInteractionResponse::Message(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .content("Seul un moderateur peut valider une action automod.")
                        .ephemeral(true),
                ),
            )
            .await;
        warn!(
            user = %component.user.name,
            user_id = %component.user.id,
            "Tentative d'action review sans permission"
        );
        return;
    }

    let custom_id = &component.data.custom_id;
    // Format : am_{action}:{guild_id}:{channel_id}:{message_id}:{user_id}
    let parts: Vec<&str> = custom_id.split(':').collect();
    if parts.len() != 5 {
        warn!(custom_id = %custom_id, "custom_id review malforme (nombre de parts incorrect)");
        return;
    }

    let action_str = match parts[0].strip_prefix(AM_PREFIX) {
        Some(s) => s,
        None => {
            warn!(custom_id = %custom_id, "custom_id review sans prefix am_");
            return;
        }
    };
    let action_char_val = match action_str.chars().next() {
        Some(c) if matches!(c, 'w' | 'd' | 'm' | 'b' | 'i') => c,
        _ => {
            warn!(custom_id = %custom_id, "custom_id review action char invalide");
            return;
        }
    };
    let action = char_to_action(action_char_val);
    let _guild_id_str = parts[1];
    let channel_id_str = parts[2];
    let message_id_str = parts[3];
    let user_id_str = parts[4];

    let moderator_name = &component.user.name;

    // Charger la config guild pour mute_duration
    let guild_id = component.guild_id.map(|g| g.to_string()).unwrap_or_default();
    let data = ctx.data.read().await;
    let config = if let Some(api) = data.get::<ApiClientKey>() {
        api.get_guild_config_for(&guild_id, crate::modules::automod::MODULE_BOT_NAME).await.unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };
    drop(data);

    let mute_duration_secs = BaseApiClient::config_u64(&config, "mute_duration_secs", DEFAULT_MUTE_DURATION_SECS);
    let colors = build_embed_colors(&config);

    if action == Action::None {
        // Ignorer -- mettre a jour la carte
        info!(target = %user_id_str, moderator = %moderator_name, "Detection ignoree via review");
        let ignored_embed = serenity::builder::CreateEmbed::new()
            .title("AutoMod -- Ignore par un moderateur")
            .description(format!("Moderateur : **{}**\nAucune action appliquee.", moderator_name))
            .color(0x95a5a6)
            .timestamp(serenity::model::Timestamp::now());

        if let Err(e) = component
            .create_response(
                &ctx.http,
                serenity::builder::CreateInteractionResponse::UpdateMessage(
                    serenity::builder::CreateInteractionResponseMessage::new()
                        .embed(ignored_embed)
                        .components(vec![]),
                ),
            )
            .await
        {
            error!(error = %e, "Echec update carte review (ignore)");
        }
        return;
    }

    // Executer l'action sur le message original
    let action_label = match &action {
        Action::Warn => "Avertissement",
        Action::Delete => "Suppression",
        Action::Mute => "Mute",
        Action::Ban => "Bannissement",
        Action::None => "Aucune",
    };

    let channel_id = match channel_id_str.parse::<u64>() {
        Ok(id) => serenity::model::id::ChannelId::new(id),
        Err(_) => return,
    };

    // Execute l'action
    match action {
        Action::Warn => {
            info!(target = %user_id_str, channel = %channel_id_str, moderator = %moderator_name, "Warn valide via review");
            let embed = warn_embed("Avertissement AutoMod")
                .color(colors.warn)
                .field("Raison", "Contenu inapproprie detecte par l'IA", false)
                .field("Valide par", moderator_name, true);
            if let Err(e) = channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new().embed(embed)).await {
                error!(error = %e, "Echec envoi embed warn dans le salon");
            }
        }
        Action::Delete => {
            if let Ok(msg_id) = message_id_str.parse::<u64>() {
                match channel_id
                    .delete_message(&ctx.http, serenity::model::id::MessageId::new(msg_id))
                    .await
                {
                    Ok(_) => info!(message_id = %msg_id, "Message supprime via review"),
                    Err(e) => warn!(error = %e, message_id = %msg_id, "Echec suppression message (peut-etre deja supprime)"),
                }
            }
            let embed = moderate_embed("Message supprime par un moderateur")
                .color(colors.delete)
                .field("Valide par", moderator_name, true);
            if let Err(e) = channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new().embed(embed)).await {
                error!(error = %e, "Echec envoi embed delete dans le salon");
            }
        }
        Action::Mute => {
            let mut mute_applied = false;
            if let (Some(guild_id_val), Ok(uid)) = (component.guild_id, user_id_str.parse::<u64>()) {
                match guild_id_val.member(&ctx.http, serenity::model::id::UserId::new(uid)).await {
                    Ok(mut member) => {
                        let secs = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64 + mute_duration_secs as i64)
                            .unwrap_or(0);
                        match time::OffsetDateTime::from_unix_timestamp(secs) {
                            Ok(dt) => {
                                let timeout = serenity::model::Timestamp::from(dt);
                                match member.disable_communication_until_datetime(&ctx.http, timeout).await {
                                    Ok(_) => {
                                        info!(user_id = %uid, duration = mute_duration_secs, "Mute applique via review");
                                        mute_applied = true;
                                    }
                                    Err(e) => error!(error = %e, user_id = %uid, "Echec Discord disable_communication -- le bot a-t-il la permission MODERATE_MEMBERS ?"),
                                }
                            }
                            Err(e) => error!(error = %e, "Timestamp invalide pour mute"),
                        }
                    }
                    Err(e) => warn!(error = %e, user_id = %uid, "Membre introuvable pour mute"),
                }
            } else {
                warn!(guild_id = ?component.guild_id, user_id = %user_id_str, "guild_id ou user_id invalide pour mute");
            }

            // Supprimer le message original APRES le mute (best-effort).
            if let Ok(msg_id) = message_id_str.parse::<u64>() {
                if let Err(e) = channel_id
                    .delete_message(&ctx.http, serenity::model::id::MessageId::new(msg_id))
                    .await
                {
                    warn!(error = %e, "Echec suppression message apres mute review");
                }
            }
            let mute_min = mute_duration_secs / 60;
            let status_text = if mute_applied { "applique" } else { "ECHOUE (voir logs)" };
            let embed = danger_embed(&format!("Mute {} par un moderateur", status_text))
                .color(colors.mute)
                .field("Duree", format!("{} minutes", mute_min), true)
                .field("Valide par", moderator_name, true);
            let _ = channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new().embed(embed)).await;
        }
        Action::Ban => {
            info!(target = %user_id_str, channel = %channel_id_str, moderator = %moderator_name, "Ban signale via review");
            let embed = critical_embed("Signalement pour bannissement (valide)")
                .color(colors.ban)
                .field("Valide par", moderator_name, true);
            if let Err(e) = channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new().embed(embed)).await {
                error!(error = %e, "Echec envoi embed ban dans le salon");
            }
        }
        Action::None => {}
    }

    // Mettre a jour la carte de review (retirer les boutons, afficher le resultat)
    let result_embed = serenity::builder::CreateEmbed::new()
        .title(format!("AutoMod -- {} applique", action_label))
        .description(format!(
            "Moderateur : **{}**\nCible : <@{}>\nSalon : <#{}>",
            moderator_name, user_id_str, channel_id_str
        ))
        .color(match action {
            Action::Warn => colors.warn,
            Action::Delete => colors.delete,
            Action::Mute => colors.mute,
            Action::Ban => colors.ban,
            Action::None => 0x95a5a6,
        })
        .footer(serenity::builder::CreateEmbedFooter::new("AutoMod Review | Action executee"))
        .timestamp(serenity::model::Timestamp::now());

    let _ = component
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::UpdateMessage(
                serenity::builder::CreateInteractionResponseMessage::new()
                    .embed(result_embed)
                    .components(vec![]),
            ),
        )
        .await;

    info!(
        moderator = %moderator_name,
        action = %action_label,
        target_user = %user_id_str,
        "Action automod validee par un moderateur"
    );
}

/// Sanitise le contenu utilisateur pour l'affichage dans les embeds Discord.
pub(super) fn sanitize_embed_content(content: &str, max_len: usize) -> String {
    let truncated: String = content.chars().take(max_len).collect();
    truncated
        .replace("```", "` ` `")
        .replace("||", "| |")
        .replace("@everyone", "@-everyone")
        .replace("@here", "@-here")
}
