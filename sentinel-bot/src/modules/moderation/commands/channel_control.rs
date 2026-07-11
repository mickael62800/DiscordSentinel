//! Actions de moderation MANUELLES au niveau salon : verrouillage ciblé
//! (`/lock` / `/unlock`) et slowmode a la demande (`/slowmode`).
//!
//! Complement des mecanismes AUTOMATIQUES (anti-raid guild-wide) : ici un modo
//! peut agir immediatement sur UN salon precis.

use serenity::all::{
    ChannelId, CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    EditChannel, PermissionOverwrite, PermissionOverwriteType, Permissions,
};

use crate::shared::discord_helpers::edit_response_text;

const CHANNEL_PERM: Permissions = Permissions::MANAGE_CHANNELS;

pub fn register_lock() -> CreateCommand {
    CreateCommand::new("lock")
        .description("Verrouille un salon (empeche @everyone d'y ecrire)")
        .default_member_permissions(CHANNEL_PERM)
        .add_option(CreateCommandOption::new(
            CommandOptionType::Channel,
            "channel",
            "Salon a verrouiller (defaut : salon courant)",
        ))
}

pub fn register_unlock() -> CreateCommand {
    CreateCommand::new("unlock")
        .description("Deverrouille un salon precedemment verrouille")
        .default_member_permissions(CHANNEL_PERM)
        .add_option(CreateCommandOption::new(
            CommandOptionType::Channel,
            "channel",
            "Salon a deverrouiller (defaut : salon courant)",
        ))
}

pub fn register_slowmode() -> CreateCommand {
    CreateCommand::new("slowmode")
        .description("Definit le mode lent d'un salon (0 = desactive)")
        .default_member_permissions(CHANNEL_PERM)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "seconds",
                "Delai entre messages en secondes (0-21600)",
            )
            .required(true)
            .min_int_value(0)
            .max_int_value(21600),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::Channel,
            "channel",
            "Salon cible (defaut : salon courant)",
        ))
}

/// Extrait une option de type salon.
fn option_channel(
    options: &[serenity::all::CommandDataOption],
    name: &str,
) -> Option<ChannelId> {
    options.iter().find(|o| o.name == name).and_then(|o| {
        match &o.value {
            serenity::all::CommandDataOptionValue::Channel(id) => Some(*id),
            _ => None,
        }
    })
}

/// Salon cible : option `channel` si fournie, sinon le salon de la commande.
fn target_channel(command: &CommandInteraction) -> ChannelId {
    option_channel(&command.data.options, "channel").unwrap_or(command.channel_id)
}

async fn ensure_ready(ctx: &Context, command: &CommandInteraction) -> bool {
    if !super::has_mod_permission(command, CHANNEL_PERM) {
        let _ = command
            .create_response(
                &ctx.http,
                serenity::all::CreateInteractionResponse::Message(
                    serenity::all::CreateInteractionResponseMessage::new()
                        .content("❌ Permission MANAGE_CHANNELS requise.")
                        .ephemeral(true),
                ),
            )
            .await;
        return false;
    }
    if command.guild_id.is_none() {
        let _ = command
            .create_response(
                &ctx.http,
                serenity::all::CreateInteractionResponse::Message(
                    serenity::all::CreateInteractionResponseMessage::new()
                        .content("Commande serveur uniquement.")
                        .ephemeral(true),
                ),
            )
            .await;
        return false;
    }
    command
        .create_response(
            &ctx.http,
            serenity::all::CreateInteractionResponse::Defer(
                serenity::all::CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
        .is_ok()
}

pub async fn handle_lock(ctx: &Context, command: &CommandInteraction) {
    handle_lock_toggle(ctx, command, true).await;
}

pub async fn handle_unlock(ctx: &Context, command: &CommandInteraction) {
    handle_lock_toggle(ctx, command, false).await;
}

async fn handle_lock_toggle(ctx: &Context, command: &CommandInteraction, lock: bool) {
    if !ensure_ready(ctx, command).await {
        return;
    }
    let guild_id = command.guild_id.unwrap();
    let channel = target_channel(command);
    let everyone = guild_id.everyone_role();

    // Verrouiller = deny SEND_MESSAGES (+ threads) pour @everyone ; deverrouiller
    // = repasser ces permissions en "neutre" (0) au lieu de deny.
    let send = Permissions::SEND_MESSAGES | Permissions::SEND_MESSAGES_IN_THREADS;
    let overwrite = PermissionOverwrite {
        allow: Permissions::empty(),
        deny: if lock { send } else { Permissions::empty() },
        kind: PermissionOverwriteType::Role(everyone),
    };

    if let Err(e) = channel.create_permission(&ctx.http, overwrite).await {
        edit_response_text(ctx, command, &format!("Erreur Discord : {e}")).await;
        return;
    }

    let msg = if lock {
        format!("🔒 <#{channel}> verrouillé (@everyone ne peut plus écrire).")
    } else {
        format!("🔓 <#{channel}> déverrouillé.")
    };
    edit_response_text(ctx, command, &msg).await;
    tracing::info!(guild = %guild_id, channel = %channel, lock, moderator = %command.user.name, "channel lock toggle");
}

pub async fn handle_slowmode(ctx: &Context, command: &CommandInteraction) {
    if !ensure_ready(ctx, command).await {
        return;
    }
    let channel = target_channel(command);
    let seconds =
        crate::shared::discord_helpers::option_i64(&command.data.options, "seconds").unwrap_or(0);
    let secs = seconds.clamp(0, 21600) as u16;

    if let Err(e) = channel
        .edit(&ctx.http, EditChannel::new().rate_limit_per_user(secs))
        .await
    {
        edit_response_text(ctx, command, &format!("Erreur Discord : {e}")).await;
        return;
    }

    let msg = if secs == 0 {
        format!("🐌 Mode lent désactivé sur <#{channel}>.")
    } else {
        format!("🐌 Mode lent réglé à {secs}s sur <#{channel}>.")
    };
    edit_response_text(ctx, command, &msg).await;
}
