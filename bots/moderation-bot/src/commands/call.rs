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

use crate::api_client::ModerationAction;
use crate::handler::ModerationApiKey;

pub const CALL_CLOSE_ID: &str = "sentinel_mod_call_close";

pub fn register() -> CreateCommand {
    CreateCommand::new("call")
        .description("Convoquer un membre dans un salon prive")
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
    let target_id = command.data.options.iter().find(|o| o.name == "user")
        .and_then(|o| match &o.value { CommandDataOptionValue::User(id) => Some(*id), _ => None })
        .unwrap();

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

    let bot_id = ctx.cache.current_user().id;

    // Lire la categorie depuis la config
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

    // Creer le salon prive
    let channel_name = format!("call-{}", target.name.to_lowercase().replace(' ', "-"));

    let mut builder = CreateChannel::new(&channel_name)
        .kind(ChannelType::Text)
        .topic(format!("[call:{}:{}] {}", command.user.id, target.id, reason))
        .permissions(vec![
            // @everyone : deny tout
            PermissionOverwrite {
                allow: Permissions::empty(),
                deny: Permissions::VIEW_CHANNEL,
                kind: PermissionOverwriteType::Role(serenity::model::id::RoleId::new(guild_id.get())),
            },
            // Target : voir + ecrire
            PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(target.id),
            },
            // Moderateur : voir + ecrire
            PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY | Permissions::MANAGE_MESSAGES,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(command.user.id),
            },
            // Bot : voir + ecrire + gerer
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

    // Message d'accueil
    let close_button = CreateButton::new(CALL_CLOSE_ID)
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

    // Log au backend
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

    // Reponse
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
}

/// Gere le clic sur "Terminer la convocation" → supprime le salon.
pub async fn handle_close(ctx: &Context, component: &ComponentInteraction) {
    let channel_id = component.channel_id;

    // Repondre d'abord
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("Convocation terminee. Suppression du salon dans 3 secondes...")
            .ephemeral(false),
    );
    if let Err(e) = component.create_response(&ctx.http, response).await {
        warn!(error = %e, "Failed to send call close response");
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    if let Err(e) = channel_id.delete(&ctx.http).await {
        error!(error = %e, "Impossible de supprimer le salon de convocation");
    } else {
        info!(channel_id = %channel_id, "Salon de convocation supprime");
    }
}

