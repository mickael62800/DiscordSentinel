use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateMessage,
};
use tracing::{error, info};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::embeds::{critical_embed, success_embed};
use sentinel_shared::heartbeat::ApiClientKey;

use crate::api_client::ModerationAction;
use crate::handler::ModerationApiKey;

pub fn register() -> CreateCommand {
    CreateCommand::new("ban")
        .description("Bannir un utilisateur (permanent ou temporaire)")
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
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "user_id", "ID de l'utilisateur a debannir")
                .required(true),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let options = &command.data.options;

    let target_id = options.iter().find(|o| o.name == "user")
        .and_then(|o| match &o.value { CommandDataOptionValue::User(id) => Some(*id), _ => None })
        .unwrap();

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

    let is_permanent = duration_hours.is_none();
    let duration_secs = duration_hours.map(|h| (h as u64) * 3600);
    let duration_label = if is_permanent {
        "permanent".to_string()
    } else {
        format!("{}h", duration_hours.unwrap())
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
    let ban_delete_message_days = BaseApiClient::config_u64(&guild_config, "ban_delete_message_days", 1) as u8;

    let guild_name = guild_id.to_partial_guild(&ctx.http).await
        .map(|g| g.name).unwrap_or_else(|_| "le serveur".into());

    // DM avant le ban (apres le ban on ne peut plus DM)
    if let Ok(dm) = target.create_dm_channel(&ctx.http).await {
        let dm_embed = critical_embed(format!("🔨 Ban ({duration_label}) sur **{guild_name}**"))
            .field("Raison", reason, false);

        dm.send_message(
            &ctx.http,
            CreateMessage::new().embed(dm_embed),
        ).await.ok();
    }

    // Executer le ban Discord (supprime les messages des derniers N jours)
    if let Err(e) = guild_id.ban_with_reason(&ctx.http, target.id, ban_delete_message_days, reason).await {
        error!(error = %e, "Impossible de bannir");
        reply_text(ctx, command, &format!("Erreur Discord : {e}")).await;
        return;
    }

    // Log dans le backend
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
        action_type: if is_permanent { "ban_permanent".to_string() } else { "ban_temp".to_string() },
        reason: reason.to_string(),
        gravity: None,
        duration: duration_secs,
    };

    if let Err(e) = api.log_action(&action).await {
        error!(error = %e, "Erreur log ban");
    }

    info!(target = %target.name, duration = %duration_label, "Ban applique");

    let channel_embed = critical_embed(format!("🔨 Ban ({duration_label})"))
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

    // Log
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

    api.log_action(&action).await.ok();

    info!(target_id = user_id_str, "Unban applique");

    let unban_embed = success_embed("✅ Unban")
        .field("Utilisateur", format!("`{user_id_str}`"), false);

    command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().embed(unban_embed),
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
