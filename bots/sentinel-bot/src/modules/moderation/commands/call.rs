use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, ComponentInteraction,
    Context, CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, PermissionOverwrite, PermissionOverwriteType,
};
use serenity::builder::{CreateActionRow, CreateButton, CreateChannel, CreateMessage};
use serenity::model::channel::ChannelType;
use serenity::model::Permissions;
use tracing::{error, info, warn};

use sentinel_shared::discord_helpers::reply_ephemeral;
use sentinel_shared::embeds::info_embed;
use sentinel_shared::heartbeat::ApiClientKey;

use super::super::api_client::ModerationAction;
use super::super::ModerationApiKey;

pub const CALL_CLOSE_PREFIX: &str = "sentinel_mod_call_close:";

pub fn register() -> CreateCommand {
    CreateCommand::new("call")
        .description("Convoquer un membre dans un salon prive")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Membre a convoquer")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Raison de la convocation")
                .required(false),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, serenity::all::Permissions::MODERATE_MEMBERS) {
        reply_ephemeral(ctx, command, "❌ Permission MODERATE_MEMBERS requise pour /call.").await;
        warn!(user = %command.user.name, "Tentative /call sans permission");
        return;
    }

    let target_id = match command.data.options.iter().find(|o| o.name == "user")
        .and_then(|o| match &o.value { CommandDataOptionValue::User(id) => Some(*id), _ => None })
    {
        Some(id) => id,
        None => { reply_ephemeral(ctx, command, "Parametre 'user' manquant.").await; return; }
    };

    let reason = command.data.options.iter().find(|o| o.name == "reason")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("Convocation par un moderateur");

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => { reply_ephemeral(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    let target = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => { reply_ephemeral(ctx, command, "Utilisateur introuvable.").await; return; }
    };

    if let Some(role_id) = super::find_immune_role(ctx, guild_id, target.id).await {
        reply_ephemeral(ctx, command, &super::immunity_message(role_id, "Convocation")).await;
        return;
    }

    let bot_id = ctx.cache.current_user().id;

    let category_id = {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let gc = match base.get_guild_config(&guild_id.to_string()).await {
                Ok(config) => config,
                Err(e) => {
                    warn!(error = %e, "Failed to fetch guild config for call");
                    std::collections::HashMap::new()
                }
            };
            gc.get("call_category_id").and_then(|v| v.parse::<u64>().ok())
        } else {
            None
        }
    };

    let channel_name = format!("call-{}", target.name.to_lowercase().replace(' ', "-"));

    let mut builder = CreateChannel::new(&channel_name)
        .kind(ChannelType::Text)
        .topic(format!("[call:{}:{}] {}", command.user.id, target.id, reason))
        .permissions(vec![
            PermissionOverwrite {
                allow: Permissions::empty(),
                deny: Permissions::VIEW_CHANNEL,
                kind: PermissionOverwriteType::Role(serenity::model::id::RoleId::new(guild_id.get())),
            },
            PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(target.id),
            },
            PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY | Permissions::MANAGE_MESSAGES,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(command.user.id),
            },
            PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::MANAGE_CHANNELS,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(bot_id),
            },
        ]);

    if let Some(cat_id) = category_id {
        builder = builder.category(serenity::model::id::ChannelId::new(cat_id));
    }

    let channel = match guild_id.create_channel(&ctx.http, builder).await {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Impossible de creer le salon de convocation");
            reply_ephemeral(ctx, command, &format!("Erreur creation salon : {e}")).await;
            return;
        }
    };

    let close_button = CreateButton::new(format!("{}{}", CALL_CLOSE_PREFIX, command.user.id))
        .label("Terminer la convocation")
        .style(serenity::all::ButtonStyle::Danger);
    let row = CreateActionRow::Buttons(vec![close_button]);

    let embed = info_embed("Convocation")
        .description(format!(
            "<@{}>, vous avez ete convoque par <@{}>.\n\n**Raison :** {}",
            target.id, command.user.id, reason
        ))
        .field("Moderateur", format!("<@{}>", command.user.id), true)
        .field("Membre", format!("<@{}>", target.id), true);

    if let Err(e) = channel.send_message(
        &ctx.http,
        CreateMessage::new().embed(embed).components(vec![row]),
    ).await {
        warn!(error = %e, "Failed to send call welcome message");
    }

    {
        let data = ctx.data.read().await;
        let api = match data.get::<ModerationApiKey>() {
            Some(a) => a,
            None => { tracing::error!("ModerationApiKey manquant"); return; }
        };
        let action = ModerationAction {
            guild_id: guild_id.to_string(),
            channel_id: channel.id.to_string(),
            moderator_id: command.user.id.to_string(),
            moderator_name: command.user.name.clone(),
            target_id: target.id.to_string(),
            target_name: target.name.clone(),
            action_type: "call".to_string(),
            reason: reason.to_string(),
            gravity: None,
            duration: None,
        };
        if let Err(e) = api.log_action(&action).await {
            warn!(error = %e, "Failed to log call action");
        }
    }

    if let Err(e) = command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!("Convocation creee dans <#{}>", channel.id))
                .ephemeral(true),
        ),
    ).await {
        warn!(error = %e, "Failed to send call response");
    }

    info!(
        moderator = %command.user.name,
        target = %target.name,
        channel = %channel.name,
        "Convocation creee"
    );

    let call_log_embed = info_embed("📞 Convocation")
        .field("Cible", format!("<@{}>", target.id), true)
        .field("ID Cible", target.id.to_string(), true)
        .field("Moderateur", format!("<@{}>", command.user.id), true)
        .field("Salon cree", format!("<#{}>", channel.id), false)
        .field("Raison", reason, false)
        .thumbnail(target.face())
        .timestamp(serenity::model::Timestamp::now())
        .footer(serenity::builder::CreateEmbedFooter::new("Moderation | Sentinel"));
    super::log_to_channel(ctx, &guild_id.to_string(), call_log_embed).await;
}

pub async fn handle_close(ctx: &Context, component: &ComponentInteraction) {
    let channel_id = component.channel_id;

    let original_mod_id = component
        .data
        .custom_id
        .strip_prefix(CALL_CLOSE_PREFIX)
        .and_then(|s| s.parse::<u64>().ok());

    let is_original_mod = original_mod_id
        .map(|id| component.user.id.get() == id)
        .unwrap_or(false);

    let has_mod_permission = component
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| p.contains(serenity::all::Permissions::MODERATE_MEMBERS) || p.contains(serenity::all::Permissions::ADMINISTRATOR))
        .unwrap_or(false);

    if !is_original_mod && !has_mod_permission {
        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Seul un moderateur peut clore la convocation.")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("Convocation terminee. Suppression du salon dans 3 secondes...")
            .ephemeral(false),
    );
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Failed to send call close response");
    }

    let http = ctx.http.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        if let Err(e) = channel_id.delete(&http).await {
            error!(error = %e, "Impossible de supprimer le salon de convocation");
        } else {
            info!(channel_id = %channel_id, "Salon de convocation supprime");
        }
    });
}
