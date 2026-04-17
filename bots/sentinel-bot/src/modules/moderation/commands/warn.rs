use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};
use serenity::builder::CreateEmbedFooter;
use tracing::{error, info, warn};

use sentinel_shared::embeds::{sentinel_embed, gravity_color, gravity_emoji, danger_embed, moderate_embed};

use super::api_client::ModerationAction;
use super::ModerationApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("warn")
        .description("Avertir un utilisateur")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Utilisateur a avertir")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "gravity", "Gravite de l'avertissement")
                .required(true)
                .add_string_choice("Faible", "low")
                .add_string_choice("Moyenne", "medium")
                .add_string_choice("Haute", "high"),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Raison de l'avertissement")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, serenity::all::Permissions::MODERATE_MEMBERS) {
        let _ = command.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("❌ Permission MODERATE_MEMBERS requise pour /warn.")
                    .ephemeral(true),
            ),
        ).await;
        warn!(user = %command.user.name, "Tentative /warn sans permission");
        return;
    }

    if let Err(e) = command.create_response(
        &ctx.http,
        CreateInteractionResponse::Defer(
            CreateInteractionResponseMessage::new().ephemeral(true),
        ),
    ).await {
        warn!(error = %e, cmd = "warn", "Echec defer interaction Discord");
        return;
    }

    let options = &command.data.options;

    let target_id = match options.iter().find(|o| o.name == "user")
        .and_then(|o| match &o.value { CommandDataOptionValue::User(id) => Some(*id), _ => None })
    {
        Some(id) => id,
        None => { reply_text(ctx, command, "Parametre 'user' manquant.").await; return; }
    };

    let gravity = options.iter().find(|o| o.name == "gravity")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("medium");

    let reason_raw = options.iter().find(|o| o.name == "reason")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("Aucune raison");
    let reason: &str = &reason_raw.chars().take(500).collect::<String>();

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => { reply_text(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    let target = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => { reply_text(ctx, command, "Utilisateur introuvable.").await; return; }
    };

    if let Some(role_id) = super::find_immune_role(ctx, guild_id, target.id).await {
        reply_text(ctx, command, &super::immunity_message(role_id, "Warn")).await;
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => { tracing::error!("ModerationApiKey manquant"); return; }
    };

    let action = ModerationAction {
        guild_id: guild_id.to_string(),
        channel_id: command.channel_id.to_string(),
        moderator_id: command.user.id.to_string(),
        moderator_name: command.user.name.clone(),
        target_id: target.id.to_string(),
        target_name: target.name.clone(),
        action_type: "warn".to_string(),
        reason: reason.to_string(),
        gravity: Some(gravity.to_string()),
        duration: None,
    };

    match api.log_action(&action).await {
        Ok(resp) => {
            info!(
                action_id = %resp.id,
                target = %target.name,
                gravity = gravity,
                strikes = ?resp.strikes_count,
                escalation = ?resp.escalation_action,
                "Warn enregistre"
            );

            let guild_name = guild_id.to_partial_guild(&ctx.http).await
                .map(|g| g.name).unwrap_or_else(|_| "le serveur".into());

            if let Ok(dm) = target.create_dm_channel(&ctx.http).await {
                let dm_embed = sentinel_embed(
                    format!("{} Avertissement sur **{guild_name}**", gravity_emoji(gravity)),
                    gravity_color(gravity),
                )
                .field("Gravite", gravity, true)
                .field("Raison", reason, false);

                if let Err(e) = dm.send_message(
                    &ctx.http,
                    CreateMessage::new().embed(dm_embed),
                ).await {
                    warn!(error = %e, "Failed to send warn DM to user");
                }
            }

            let strikes_label = resp.strikes_count.map(|c| format!(" — Strike {c}")).unwrap_or_default();
            let channel_embed = sentinel_embed(
                format!("{} Warn ({gravity}){strikes_label}", gravity_emoji(gravity)),
                gravity_color(gravity),
            )
            .thumbnail(target.face())
            .field("Cible", format!("<@{}>", target.id), true)
            .field("Moderateur", format!("<@{}>", command.user.id), true)
            .field("Gravite", gravity, true)
            .field("ID Cible", target.id.to_string(), true)
            .field("Salon", format!("<#{}>", command.channel_id), true)
            .field("Strikes", resp.strikes_count.map(|c| c.to_string()).unwrap_or_else(|| "—".to_string()), true)
            .field("Raison", reason, false)
            .timestamp(serenity::model::Timestamp::now())
            .footer(CreateEmbedFooter::new("Moderation | Sentinel"));

            if let Err(e) = command.edit_response(
                &ctx.http,
                serenity::builder::EditInteractionResponse::new()
                    .content(format!("✅ Avertissement envoye a <@{}>.", target.id)),
            ).await {
                warn!(error = %e, "Failed to edit warn response");
            }

            super::log_to_channel(ctx, &guild_id.to_string(), channel_embed).await;

            if let Some(ref esc_action) = resp.escalation_action {
                let mut member = match guild_id.member(&ctx.http, target.id).await {
                    Ok(m) => m,
                    Err(_) => return,
                };
                match esc_action.as_str() {
                    "mute" => {
                        let secs = resp.escalation_duration.unwrap_or(600);
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64 + secs as i64;
                        let datetime = time::OffsetDateTime::from_unix_timestamp(ts).unwrap_or_else(|_| time::OffsetDateTime::now_utc());
                        let timeout = serenity::model::Timestamp::from(datetime);
                        if let Err(e) = member.disable_communication_until_datetime(&ctx.http, timeout).await {
                            warn!(error = %e, "Escalation mute echouee");
                        } else {
                            let esc_embed = moderate_embed(format!("🔇 Mute auto (escalation — {} strikes)", resp.strikes_count.unwrap_or(0)))
                                .field("Cible", format!("<@{}>", target.id), true)
                                .field("ID Cible", target.id.to_string(), true)
                                .field("Duree", format!("{}min", secs / 60), true)
                                .field("Declencheur", format!("/warn par <@{}>", command.user.id), false)
                                .thumbnail(target.face())
                                .timestamp(serenity::model::Timestamp::now())
                                .footer(CreateEmbedFooter::new("Moderation | Sentinel"));
                            super::log_to_channel(ctx, &guild_id.to_string(), esc_embed).await;
                        }
                    }
                    "ban" => {
                        let esc_embed = danger_embed(format!("🔨 Ban auto (escalation — {} strikes)", resp.strikes_count.unwrap_or(0)))
                            .field("Cible", format!("<@{}>", target.id), true)
                            .field("ID Cible", target.id.to_string(), true)
                            .field("Declencheur", format!("/warn par <@{}>", command.user.id), false)
                            .thumbnail(target.face())
                            .timestamp(serenity::model::Timestamp::now())
                            .footer(CreateEmbedFooter::new("Moderation | Sentinel"));
                        super::log_to_channel(ctx, &guild_id.to_string(), esc_embed).await;
                        if let Err(e) = guild_id.ban_with_reason(&ctx.http, target.id, 1, reason).await {
                            warn!(error = %e, "Escalation ban echouee");
                        }
                    }
                    _ => {}
                }
            }
        }
        Err(e) => {
            error!(error = %e, "Erreur log warn");
            reply_text(ctx, command, &format!("Erreur : {e}")).await;
        }
    }
}

async fn reply_text(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command.edit_response(
        &ctx.http,
        serenity::builder::EditInteractionResponse::new().content(content),
    ).await {
        warn!(error = %e, "Failed to send reply text");
    }
}
