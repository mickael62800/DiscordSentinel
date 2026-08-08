use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
};
use serenity::builder::CreateEmbedFooter;
use tracing::{error, info, warn};

use crate::shared::discord_helpers::edit_response_text;
use crate::shared::embeds::{moderate_embed, warn_embed};

use super::api_client::ModerationAction;
use super::ModerationApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("kick")
        .description("Expulse un utilisateur du serveur (il peut revenir)")
        .default_member_permissions(serenity::all::Permissions::KICK_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Raison de l'expulsion")
                .required(true),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::User,
            "user",
            "Utilisateur a expulser (ou utilise user_id)",
        ))
        .add_option(CreateCommandOption::new(
            CommandOptionType::String,
            "user_id",
            "ID de l'utilisateur (alternative au selecteur)",
        ))
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, serenity::all::Permissions::KICK_MEMBERS) {
        let _ = command
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Permission KICK_MEMBERS requise pour /kick.")
                        .ephemeral(true),
                ),
            )
            .await;
        warn!(user = %command.user.name, "Tentative /kick sans permission");
        return;
    }

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, cmd = "kick", "Echec defer interaction Discord");
        return;
    }

    let options = &command.data.options;

    let target_id = match super::resolve_target_user_id(command, "user") {
        Some(id) => id,
        None => {
            edit_response_text(ctx, command, "Parametre 'user' manquant.").await;
            return;
        }
    };

    let reason_raw =
        crate::shared::discord_helpers::option_str(options, "reason").unwrap_or("Aucune raison");
    let reason: &str = &reason_raw.chars().take(500).collect::<String>();

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => {
            edit_response_text(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    if let Err(msg) = super::check_hierarchy(ctx, command, guild_id, target_id) {
        edit_response_text(ctx, command, &format!("❌ {msg}")).await;
        return;
    }

    if let Err(msg) =
        super::check_mod_quota(ctx, &guild_id.to_string(), &command.user.id.to_string()).await
    {
        edit_response_text(ctx, command, &msg).await;
        return;
    }

    let target = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => {
            edit_response_text(ctx, command, "Utilisateur introuvable.").await;
            return;
        }
    };

    if guild_id.member(&ctx.http, target.id).await.is_err() {
        edit_response_text(ctx, command, "Membre introuvable sur le serveur.").await;
        return;
    }

    if let Some(role_id) = super::find_immune_role(ctx, guild_id, target.id).await {
        edit_response_text(ctx, command, &super::immunity_message(role_id, "Kick")).await;
        return;
    }

    let guild_name = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map(|g| g.name)
        .unwrap_or_else(|_| "le serveur".into());

    // DM AVANT l'expulsion (apres, le canal DM peut devenir injoignable).
    if let Ok(dm) = target.create_dm_channel(&ctx.http).await {
        let dm_embed = warn_embed(format!("👢 Expulsion de **{guild_name}**"))
            .description(
                "Tu as ete expulse du serveur. Tu peux le rejoindre a nouveau via une invitation.",
            )
            .field("Raison", reason, false)
            .timestamp(serenity::model::Timestamp::now());
        if let Err(e) = dm
            .send_message(&ctx.http, CreateMessage::new().embed(dm_embed))
            .await
        {
            warn!(error = %e, "Failed to send kick DM to user");
        }
    }

    // Expulsion Discord.
    if let Err(e) = guild_id
        .kick_with_reason(&ctx.http, target.id, reason)
        .await
    {
        error!(error = %e, "Impossible d'expulser l'utilisateur");
        edit_response_text(ctx, command, &format!("Erreur Discord : {e}")).await;
        return;
    }

    // Journalisation cote API (best-effort).
    {
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ModerationApiKey>() {
            let action = ModerationAction {
                guild_id: guild_id.to_string(),
                channel_id: command.channel_id.to_string(),
                moderator_id: command.user.id.to_string(),
                moderator_name: command.user.name.clone(),
                target_id: target.id.to_string(),
                target_name: target.name.clone(),
                action_type: "kick".to_string(),
                reason: reason.to_string(),
                gravity: None,
                duration: None,
            };
            if let Err(e) = api.log_action(&action).await {
                error!(error = %e, "Erreur log kick");
            }
        } else {
            tracing::error!("ModerationApiKey manquant");
        }
    }

    info!(target = %target.name, moderator = %command.user.name, "Kick applique");

    let channel_embed = moderate_embed("👢 Expulsion")
        .thumbnail(target.face())
        .field("Cible", format!("<@{}>", target.id), true)
        .field("Moderateur", format!("<@{}>", command.user.id), true)
        .field("ID Cible", target.id.to_string(), true)
        .field("Raison", reason, false)
        .field("Salon", format!("<#{}>", command.channel_id), true)
        .timestamp(serenity::model::Timestamp::now())
        .footer(CreateEmbedFooter::new("Moderation | Sentinel"));

    if let Err(e) = command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new()
                .content(format!("✅ <@{}> a ete expulse.", target.id)),
        )
        .await
    {
        warn!(error = %e, "Failed to edit kick response");
    }

    super::log_to_channel(ctx, &guild_id.to_string(), channel_embed).await;

    crate::shared::discord_helpers::post_sanction_card(
        ctx,
        &guild_id.to_string(),
        crate::shared::discord_helpers::SanctionKind::Kick,
        target.id.get(),
        Some(&target.name),
        &command.user.name,
        reason,
        None,
    )
    .await;
}
