use serenity::all::{
    ButtonStyle, CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateActionRow, CreateButton, CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, User,
};
use serenity::builder::CreateEmbedFooter;
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::embeds::{critical_embed, success_embed};
use sentinel_shared::heartbeat::ApiClientKey;

use super::api_client::ModerationAction;
use super::ModerationApiKey;
use super::risk_check::{
    self, PendingKind, RiskyPending, RiskyPendingKey, CANCEL_PREFIX, CONFIRM_PREFIX,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("ban")
        .description("Bannir un utilisateur (permanent ou temporaire)")
        .default_member_permissions(serenity::all::Permissions::BAN_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Utilisateur a bannir")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Raison du ban")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "duration",
                "Duree en heures (vide = permanent)",
            ),
        )
}

pub fn register_unban() -> CreateCommand {
    CreateCommand::new("unban")
        .description("Debannir un utilisateur")
        .default_member_permissions(serenity::all::Permissions::BAN_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "user_id", "ID de l'utilisateur a debannir")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    if !super::has_mod_permission(command, serenity::all::Permissions::BAN_MEMBERS) {
        let _ = command.create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("❌ Permission BAN_MEMBERS requise pour /ban.")
                    .ephemeral(true),
            ),
        ).await;
        warn!(user = %command.user.name, "Tentative /ban sans permission");
        return;
    }

    if let Err(e) = command.create_response(
        &ctx.http,
        CreateInteractionResponse::Defer(
            CreateInteractionResponseMessage::new().ephemeral(true),
        ),
    ).await {
        warn!(error = %e, cmd = "ban", "Echec defer interaction Discord");
        return;
    }

    let options = &command.data.options;

    let target_id = match options.iter().find(|o| o.name == "user")
        .and_then(|o| match &o.value { CommandDataOptionValue::User(id) => Some(*id), _ => None })
    {
        Some(id) => id,
        None => { reply_text(ctx, command, "Parametre 'user' manquant.").await; return; }
    };

    let reason_raw = options.iter().find(|o| o.name == "reason")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("Aucune raison");
    let reason: &str = &reason_raw.chars().take(500).collect::<String>();

    let duration_hours = options.iter().find(|o| o.name == "duration")
        .and_then(|o| match &o.value { CommandDataOptionValue::Integer(n) => Some(*n), _ => None });

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => { reply_text(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    let target = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => { reply_text(ctx, command, "Utilisateur introuvable.").await; return; }
    };

    if let Some(role_id) = super::find_immune_role(ctx, guild_id, target.id).await {
        reply_text(ctx, command, &super::immunity_message(role_id, "Ban")).await;
        return;
    }

    let is_permanent = duration_hours.is_none();
    let duration_secs = duration_hours.map(|h| (h as u64) * 3600);
    let duration_label = if is_permanent {
        "permanent".to_string()
    } else {
        format!("{}h", duration_hours.unwrap())
    };

    let guild_config = {
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            match api.get_guild_config(&guild_id.to_string()).await {
                Ok(config) => config,
                Err(e) => {
                    warn!(error = %e, "Failed to fetch guild config for ban");
                    std::collections::HashMap::new()
                }
            }
        } else {
            std::collections::HashMap::new()
        }
    };
    let ban_delete_message_days = BaseApiClient::config_u64(&guild_config, "ban_delete_message_days", 1) as u8;

    if let Some(risk_reason) = risk_check::check_target_risk(ctx, guild_id, &target).await {
        defer_with_confirmation(
            ctx,
            command,
            &target,
            reason,
            duration_secs,
            &duration_label,
            is_permanent,
            ban_delete_message_days,
            &risk_reason,
        )
        .await;
        return;
    }

    execute_ban(
        ctx,
        command.channel_id.to_string(),
        command.user.id.to_string(),
        command.user.name.clone(),
        guild_id,
        &target,
        reason,
        duration_secs,
        &duration_label,
        is_permanent,
        ban_delete_message_days,
        Some(command),
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
async fn defer_with_confirmation(
    ctx: &Context,
    command: &CommandInteraction,
    target: &User,
    reason: &str,
    duration_secs: Option<u64>,
    duration_label: &str,
    is_permanent: bool,
    ban_delete_message_days: u8,
    risk_reason: &str,
) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return,
    };

    let pending_id = uuid::Uuid::new_v4().to_string();

    let pending = RiskyPending {
        kind: PendingKind::Ban {
            delete_message_days: ban_delete_message_days,
            is_permanent,
        },
        guild_id: guild_id.to_string(),
        channel_id: command.channel_id.to_string(),
        target_id: target.id.to_string(),
        target_name: target.name.clone(),
        moderator_id: command.user.id.to_string(),
        moderator_name: command.user.name.clone(),
        reason: reason.to_string(),
        duration_secs,
        duration_label: duration_label.to_string(),
        created_at: std::time::Instant::now(),
    };

    {
        let data = ctx.data.read().await;
        if let Some(store) = data.get::<RiskyPendingKey>() {
            risk_check::purge_expired(store);
            store.insert(pending_id.clone(), pending);
        }
    }

    let embed = critical_embed("\u{26a0}\u{fe0f} Confirmation requise — cible a risque")
        .description(format!(
            "La cible <@{}> (`{}`) presente un risque : **{}**.\n\n\
             Action demandee : **Ban ({})**\n\
             Raison : {}\n\n\
             Confirmer l'execution ?",
            target.id, target.name, risk_reason, duration_label, reason
        ));

    let row = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("{CONFIRM_PREFIX}{pending_id}"))
            .label("Confirmer")
            .style(ButtonStyle::Danger),
        CreateButton::new(format!("{CANCEL_PREFIX}{pending_id}"))
            .label("Annuler")
            .style(ButtonStyle::Secondary),
    ]);

    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(vec![row])
                    .ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Failed to send risky confirmation prompt");
    }

    info!(
        moderator = %command.user.name,
        target = %target.name,
        risk = %risk_reason,
        "Ban deferred pending confirmation"
    );
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_ban(
    ctx: &Context,
    channel_id: String,
    moderator_id: String,
    moderator_name: String,
    guild_id: serenity::model::id::GuildId,
    target: &User,
    reason: &str,
    duration_secs: Option<u64>,
    duration_label: &str,
    is_permanent: bool,
    ban_delete_message_days: u8,
    command: Option<&CommandInteraction>,
) {
    let guild_name = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map(|g| g.name)
        .unwrap_or_else(|_| "le serveur".into());

    if let Ok(dm) = target.create_dm_channel(&ctx.http).await {
        let dm_embed = critical_embed(format!("🔨 Ban ({duration_label}) sur **{guild_name}**"))
            .field("Raison", reason, false);

        if let Err(e) = dm
            .send_message(&ctx.http, CreateMessage::new().embed(dm_embed))
            .await
        {
            warn!(error = %e, "Failed to send ban DM to user");
        }
    }

    if let Err(e) = guild_id
        .ban_with_reason(&ctx.http, target.id, ban_delete_message_days, reason)
        .await
    {
        error!(error = %e, "Impossible de bannir");
        if let Some(cmd) = command {
            reply_text(ctx, cmd, &format!("Erreur Discord : {e}")).await;
        }
        return;
    }

    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => {
            tracing::error!("ModerationApiKey manquant");
            return;
        }
    };

    let action = ModerationAction {
        guild_id: guild_id.to_string(),
        channel_id,
        moderator_id: moderator_id.clone(),
        moderator_name: moderator_name.clone(),
        target_id: target.id.to_string(),
        target_name: target.name.clone(),
        action_type: if is_permanent {
            "ban_permanent".to_string()
        } else {
            "ban_temp".to_string()
        },
        reason: reason.to_string(),
        gravity: None,
        duration: duration_secs,
    };

    if let Err(e) = api.log_action(&action).await {
        error!(error = %e, "Erreur log ban");
    }

    info!(target = %target.name, duration = %duration_label, "Ban applique");

    let mut channel_embed = critical_embed(format!("🔨 Ban ({duration_label})"))
        .thumbnail(target.face())
        .field("Cible", format!("<@{}>", target.id), true)
        .field("Moderateur", format!("<@{}>", moderator_id), true)
        .field("Duree", duration_label.to_string(), true)
        .field("ID Cible", target.id.to_string(), true)
        .field("Raison", reason, false);
    if let Some(cmd) = command {
        channel_embed = channel_embed.field("Salon", format!("<#{}>", cmd.channel_id), true);
    }
    let channel_embed = channel_embed
        .timestamp(serenity::model::Timestamp::now())
        .footer(CreateEmbedFooter::new("Moderation | Sentinel"));

    if let Some(cmd) = command {
        if let Err(e) = cmd
            .edit_response(
                &ctx.http,
                serenity::builder::EditInteractionResponse::new()
                    .content(format!("✅ Ban applique sur <@{}> ({}).", target.id, duration_label)),
            )
            .await
        {
            warn!(error = %e, "Failed to edit ban response");
        }
    }

    super::log_to_channel(ctx, &guild_id.to_string(), channel_embed).await;
}

pub async fn handle_unban(ctx: &Context, command: &CommandInteraction) {
    let user_id_str = command.data.options.iter().find(|o| o.name == "user_id")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("0");

    let user_id: u64 = match user_id_str.parse() {
        Ok(id) => id,
        Err(_) => { reply_text(ctx, command, "ID utilisateur invalide.").await; return; }
    };

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => { reply_text(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    let target_uid = serenity::model::id::UserId::new(user_id);

    if let Err(e) = guild_id.unban(&ctx.http, target_uid).await {
        error!(error = %e, "Impossible de debannir");
        reply_text(ctx, command, &format!("Erreur : {e}")).await;
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
        target_id: user_id_str.to_string(),
        target_name: "inconnu".to_string(),
        action_type: "unban".to_string(),
        reason: "Unban manuel".to_string(),
        gravity: None,
        duration: None,
    };

    if let Err(e) = api.log_action(&action).await {
        warn!(error = %e, "Failed to log unban action");
    }

    info!(target_id = user_id_str, "Unban applique");

    let unban_embed = success_embed("✅ Unban")
        .field("Moderateur", format!("<@{}>", command.user.id), true)
        .field("Utilisateur", format!("`{user_id_str}`"), false)
        .timestamp(serenity::model::Timestamp::now())
        .footer(CreateEmbedFooter::new("Moderation | Sentinel"));

    if let Err(e) = command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!("✅ Unban applique sur `{user_id_str}`."))
                .ephemeral(true),
        ),
    ).await {
        warn!(error = %e, "Failed to send unban response");
    }

    super::log_to_channel(ctx, &guild_id.to_string(), unban_embed).await;
}

async fn reply_text(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command.edit_response(
        &ctx.http,
        serenity::builder::EditInteractionResponse::new().content(content),
    ).await {
        warn!(error = %e, "Failed to send reply text");
    }
}
