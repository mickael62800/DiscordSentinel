//! Commande /taunts-channel (Phase 9 Part D) — admin only.
//!
//! Configure le salon ou seront postees les railleries automatiques.
//! Thin : set via RPC API, rien d'autre.

use serenity::all::{
    ChannelType, CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
    Permissions,
};

use crate::GameApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("taunts-channel")
        .description("(Admin) Configure le salon des railleries automatiques")
        .default_member_permissions(Permissions::MANAGE_GUILD)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Channel,
                "salon",
                "Salon texte. Omettre pour desactiver.",
            )
            .required(false)
            .channel_types(vec![ChannelType::Text]),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let guild_id = match command.guild_id {
        Some(id) => id.to_string(),
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            return;
        }
    };

    let channel_id = command
        .data
        .options
        .iter()
        .find(|o| o.name == "salon")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Channel(id) => Some(id.to_string()),
            _ => None,
        });

    let data = ctx.data.read().await;
    let api = match data.get::<GameApiKey>() {
        Some(a) => a,
        None => return,
    };

    if let Err(e) = api
        .set_taunts_channel(&guild_id, channel_id.as_deref())
        .await
    {
        reply_ephemeral(ctx, command, &format!("Erreur API : {e}")).await;
        return;
    }

    let msg = match channel_id {
        Some(c) => format!(
            "\u{2705} Les railleries seront postees dans <#{}>.\n_Pense a donner au bot la permission **Gerer les pseudos** si tu veux les renommages automatiques._",
            c
        ),
        None => "\u{1f6d1} Salon des railleries retire. La feature est maintenant desactivee.".into(),
    };
    reply_ephemeral(ctx, command, &msg).await;
}

async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
    }
}
