use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};
use tracing::{error, info};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::embeds::{moderate_embed, success_embed};
use sentinel_shared::heartbeat::ApiClientKey;

use crate::api_client::ModerationAction;
use crate::handler::ModerationApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("mute")
        .description("Mute un utilisateur (permanent ou temporaire)")
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
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "user", "Utilisateur a unmute")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let options = &command.data.options;

    let target_id = options.iter().find(|o| o.name == "user")
        .and_then(|o| match &o.value { CommandDataOptionValue::User(id) => Some(*id), _ => None })
        .unwrap();

    let reason = options.iter().find(|o| o.name == "reason")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("Aucune raison");

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

    // Appliquer le timeout Discord
    let mut member = match guild_id.member(&ctx.http, target.id).await {
        Ok(m) => m,
        Err(_) => { reply_text(ctx, command, "Membre introuvable sur le serveur.").await; return; }
    };

    // Charger la config per-guild depuis l'API
    let guild_config = {
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            api.get_guild_config(&guild_id.to_string()).await.unwrap_or_default()
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

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + timeout_secs as i64;

    let datetime = time::OffsetDateTime::from_unix_timestamp(ts).expect("timestamp invalide");
    let timeout = serenity::model::Timestamp::from(datetime);

    if let Err(e) = member.disable_communication_until_datetime(&ctx.http, timeout).await {
        error!(error = %e, "Impossible de mute l'utilisateur");
        reply_text(ctx, command, &format!("Erreur Discord : {e}")).await;
        return;
    }

    let is_permanent = duration_minutes.is_none();
    let duration_label = if is_permanent {
        "permanent".to_string()
    } else {
        format!("{}min", duration_minutes.unwrap())
    };

    // Log dans le backend
    let data = ctx.data.read().await;
    let api = data.get::<ModerationApiKey>().unwrap();

    let action = ModerationAction {
        guild_id: guild_id.to_string(),
        channel_id: command.channel_id.to_string(),
        moderator_id: command.user.id.to_string(),
        moderator_name: command.user.name.clone(),
        target_id: target.id.to_string(),
        target_name: target.name.clone(),
        action_type: if is_permanent { "mute_permanent".to_string() } else { "mute_temp".to_string() },
        reason: reason.to_string(),
        gravity: None,
        duration: duration_secs,
    };

    if let Err(e) = api.log_action(&action).await {
        error!(error = %e, "Erreur log mute");
    }

    info!(target = %target.name, duration = %duration_label, "Mute applique");

    let guild_name = guild_id.to_partial_guild(&ctx.http).await
        .map(|g| g.name).unwrap_or_else(|_| "le serveur".into());

    // DM
    if let Ok(dm) = target.create_dm_channel(&ctx.http).await {
        let dm_embed = moderate_embed(format!("🔇 Mute ({duration_label}) sur **{guild_name}**"))
            .field("Duree", &duration_label, true)
            .field("Raison", reason, false);

        dm.send_message(
            &ctx.http,
            CreateMessage::new().embed(dm_embed),
        ).await.ok();
    }

    let channel_embed = moderate_embed(format!("🔇 Mute ({duration_label})"))
        .field("Cible", format!("<@{}>", target.id), true)
        .field("Moderateur", format!("<@{}>", command.user.id), true)
        .field("Duree", &duration_label, true)
        .field("Raison", reason, false);

    command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().embed(channel_embed),
        ),
    ).await.ok();
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
    let api = data.get::<ModerationApiKey>().unwrap();
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

    api.log_action(&action).await.ok();

    info!(target = %target_name, "Unmute applique");

    let unmute_embed = success_embed("🔊 Unmute")
        .field("Cible", format!("<@{target_id}>"), false);

    command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().embed(unmute_embed),
        ),
    ).await.ok();
}

async fn reply_text(ctx: &Context, command: &CommandInteraction, content: &str) {
    command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(content).ephemeral(false),
        ),
    ).await.ok();
}
