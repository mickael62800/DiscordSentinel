use serenity::all::{
    ButtonStyle, CommandDataOptionValue, CommandInteraction, CommandOptionType, Context,
    CreateActionRow, CreateButton, CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, User,
};
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::embeds::{critical_embed, moderate_embed, success_embed};
use sentinel_shared::heartbeat::ApiClientKey;

use crate::api_client::ModerationAction;
use crate::handler::ModerationApiKey;
use crate::risk_check::{
    self, PendingKind, RiskyPending, RiskyPendingKey, CANCEL_PREFIX, CONFIRM_PREFIX,
};

pub fn register() -> CreateCommand {
    CreateCommand::new("mute")
        .description("Mute un utilisateur (permanent ou temporaire)")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Utilisateur a mute")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Raison du mute")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "duration",
                "Duree en minutes (vide = permanent, max 40320 = 28 jours)",
            ),
        )
}

pub fn register_unmute() -> CreateCommand {
    CreateCommand::new("unmute")
        .description("Retirer le mute d'un utilisateur")
        .default_member_permissions(serenity::all::Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Utilisateur a unmute")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    // Deferer immediatement pour eviter le timeout 3s Discord.
    let _ = command.create_response(
        &ctx.http,
        CreateInteractionResponse::Defer(
            CreateInteractionResponseMessage::new().ephemeral(true),
        ),
    ).await;

    let options = &command.data.options;

    let target_id = options.iter().find(|o| o.name == "user")
        .and_then(|o| match &o.value { CommandDataOptionValue::User(id) => Some(*id), _ => None })
        .unwrap();

    let reason_raw = options.iter().find(|o| o.name == "reason")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("Aucune raison");
    let reason: &str = &reason_raw.chars().take(500).collect::<String>();

    let duration_minutes = options.iter().find(|o| o.name == "duration")
        .and_then(|o| match &o.value { CommandDataOptionValue::Integer(n) => Some(*n), _ => None });

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => { reply_text(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    let target = match target_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(_) => { reply_text(ctx, command, "Utilisateur introuvable.").await; return; }
    };

    // Verifier que le membre existe (fail-fast avant le risk check)
    if guild_id.member(&ctx.http, target.id).await.is_err() {
        reply_text(ctx, command, "Membre introuvable sur le serveur.").await;
        return;
    }

    // Charger la config per-guild depuis l'API
    let guild_config = {
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            match api.get_guild_config(&guild_id.to_string()).await {
                Ok(config) => config,
                Err(e) => {
                    warn!(error = %e, "Failed to fetch guild config for mute");
                    std::collections::HashMap::new()
                }
            }
        } else {
            std::collections::HashMap::new()
        }
    };
    let default_mute_duration_secs = BaseApiClient::config_u64(&guild_config, "default_mute_duration_secs", 28 * 24 * 3600);
    let max_mute_duration_secs = BaseApiClient::config_u64(&guild_config, "max_mute_duration_secs", 28 * 24 * 3600);

    let duration_secs = duration_minutes.map(|m| (m as u64) * 60);
    // Discord timeout max = 28 jours. Si permanent, on utilise la valeur par defaut de la config.
    let timeout_secs = duration_secs.unwrap_or(default_mute_duration_secs);
    let timeout_secs = timeout_secs.min(max_mute_duration_secs).min(28 * 24 * 3600); // cap a 28j Discord max

    let is_permanent = duration_minutes.is_none();
    let duration_label = if is_permanent {
        "permanent".to_string()
    } else {
        format!("{}min", duration_minutes.unwrap())
    };

    // MOD #4 ext — detection cible a risque avant execution
    if let Some(risk_reason) = risk_check::check_target_risk(ctx, guild_id, &target).await {
        defer_with_confirmation(
            ctx,
            command,
            &target,
            reason,
            duration_secs,
            &duration_label,
            timeout_secs,
            &risk_reason,
        )
        .await;
        return;
    }

    execute_mute(
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
        timeout_secs,
        Some(command),
    )
    .await;
}

/// Defere l'execution du mute : stocke l'action et poste une confirmation.
#[allow(clippy::too_many_arguments)]
async fn defer_with_confirmation(
    ctx: &Context,
    command: &CommandInteraction,
    target: &User,
    reason: &str,
    duration_secs: Option<u64>,
    duration_label: &str,
    timeout_secs: u64,
    risk_reason: &str,
) {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return,
    };

    let pending_id = uuid::Uuid::new_v4().to_string();

    let pending = RiskyPending {
        kind: PendingKind::Mute {
            timeout_secs,
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
             Action demandee : **Mute ({})**\n\
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
        warn!(error = %e, "Failed to send risky mute confirmation prompt");
    }

    info!(
        moderator = %command.user.name,
        target = %target.name,
        risk = %risk_reason,
        "Mute deferred pending confirmation"
    );
}

/// Execute un mute apres verification ou apres confirmation.
#[allow(clippy::too_many_arguments)]
pub async fn execute_mute(
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
    timeout_secs: u64,
    command: Option<&CommandInteraction>,
) {
    let mut member = match guild_id.member(&ctx.http, target.id).await {
        Ok(m) => m,
        Err(_) => {
            if let Some(cmd) = command {
                reply_text(ctx, cmd, "Membre introuvable sur le serveur.").await;
            }
            return;
        }
    };

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + timeout_secs as i64;

    let datetime = time::OffsetDateTime::from_unix_timestamp(ts).expect("timestamp invalide");
    let timeout = serenity::model::Timestamp::from(datetime);

    if let Err(e) = member
        .disable_communication_until_datetime(&ctx.http, timeout)
        .await
    {
        error!(error = %e, "Impossible de mute l'utilisateur");
        if let Some(cmd) = command {
            reply_text(ctx, cmd, &format!("Erreur Discord : {e}")).await;
        }
        return;
    }

    // Log dans le backend
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
            "mute_permanent".to_string()
        } else {
            "mute_temp".to_string()
        },
        reason: reason.to_string(),
        gravity: None,
        duration: duration_secs,
    };

    if let Err(e) = api.log_action(&action).await {
        error!(error = %e, "Erreur log mute");
    }

    info!(target = %target.name, duration = %duration_label, "Mute applique");

    let guild_name = guild_id
        .to_partial_guild(&ctx.http)
        .await
        .map(|g| g.name)
        .unwrap_or_else(|_| "le serveur".into());

    // DM
    if let Ok(dm) = target.create_dm_channel(&ctx.http).await {
        let dm_embed = moderate_embed(format!("🔇 Mute ({duration_label}) sur **{guild_name}**"))
            .field("Duree", duration_label, true)
            .field("Raison", reason, false);

        if let Err(e) = dm
            .send_message(&ctx.http, CreateMessage::new().embed(dm_embed))
            .await
        {
            warn!(error = %e, "Failed to send mute DM to user");
        }
    }

    let channel_embed = moderate_embed(format!("🔇 Mute ({duration_label})"))
        .field("Cible", format!("<@{}>", target.id), true)
        .field("Moderateur", format!("<@{}>", moderator_id), true)
        .field("Duree", duration_label.to_string(), true)
        .field("Raison", reason, false);

    // Editer la reponse deferee pour confirmer au moderateur
    if let Some(cmd) = command {
        if let Err(e) = cmd
            .edit_response(
                &ctx.http,
                serenity::builder::EditInteractionResponse::new()
                    .content(format!("✅ Mute applique sur <@{}> ({}).", target.id, duration_label)),
            )
            .await
        {
            warn!(error = %e, "Failed to edit mute response");
        }
    }

    // Log dans le salon de logs
    super::log_to_channel(ctx, &guild_id.to_string(), channel_embed).await;
}

pub async fn handle_unmute(ctx: &Context, command: &CommandInteraction) {
    let target_id = command.data.options.iter().find(|o| o.name == "user")
        .and_then(|o| match &o.value { CommandDataOptionValue::User(id) => Some(*id), _ => None })
        .unwrap();

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => { reply_text(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    let mut member = match guild_id.member(&ctx.http, target_id).await {
        Ok(m) => m,
        Err(_) => { reply_text(ctx, command, "Membre introuvable.").await; return; }
    };

    if let Err(e) = member.enable_communication(&ctx.http).await {
        error!(error = %e, "Impossible de unmute");
        reply_text(ctx, command, &format!("Erreur : {e}")).await;
        return;
    }

    // Log unmute
    let data = ctx.data.read().await;
    let api = match data.get::<ModerationApiKey>() {
        Some(a) => a,
        None => { tracing::error!("ModerationApiKey manquant"); return; }
    };
    let target = target_id.to_user(&ctx.http).await.ok();
    let target_name = target.as_ref().map(|u| u.name.as_str()).unwrap_or("inconnu");

    let action = ModerationAction {
        guild_id: guild_id.to_string(),
        channel_id: command.channel_id.to_string(),
        moderator_id: command.user.id.to_string(),
        moderator_name: command.user.name.clone(),
        target_id: target_id.to_string(),
        target_name: target_name.to_string(),
        action_type: "unmute".to_string(),
        reason: "Unmute manuel".to_string(),
        gravity: None,
        duration: None,
    };

    if let Err(e) = api.log_action(&action).await {
        warn!(error = %e, "Failed to log unmute action");
    }

    info!(target = %target_name, "Unmute applique");

    let unmute_embed = success_embed("🔊 Unmute")
        .field("Cible", format!("<@{target_id}>"), false);

    if let Err(e) = command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().embed(unmute_embed),
        ),
    ).await {
        warn!(error = %e, "Failed to send unmute response embed");
    }
}

async fn reply_text(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(content).ephemeral(false),
        ),
    ).await {
        warn!(error = %e, "Failed to send reply text");
    }
}
