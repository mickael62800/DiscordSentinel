use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use tracing::info;

use sentinel_shared::embeds::danger_embed;

use crate::api_client::ModerationAction;
use crate::handler::ModerationApiKey;

pub fn register_massmute() -> CreateCommand {
    CreateCommand::new("massmute")
        .description("Mute plusieurs utilisateurs en une seule commande")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "users", "IDs des utilisateurs (separes par des espaces ou virgules)")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Raison du mute")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "duration", "Duree en minutes (defaut: 10)")
                .min_int_value(1)
                .max_int_value(40320), // 28 jours max Discord
        )
}

pub fn register_massban() -> CreateCommand {
    CreateCommand::new("massban")
        .description("Bannir plusieurs utilisateurs en une seule commande")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "users", "IDs des utilisateurs (separes par des espaces ou virgules)")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "reason", "Raison du ban")
                .required(true),
        )
}

pub async fn handle_massmute(ctx: &Context, command: &CommandInteraction) {
    let users_str = command.data.options.iter().find(|o| o.name == "users")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("");

    let reason = command.data.options.iter().find(|o| o.name == "reason")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("Mass mute");

    let duration_min = command.data.options.iter().find(|o| o.name == "duration")
        .and_then(|o| match &o.value { CommandDataOptionValue::Integer(i) => Some(*i as u64), _ => None })
        .unwrap_or(10);

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => { sentinel_shared::discord_helpers::reply_ephemeral(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    let user_ids = parse_user_ids(users_str);
    if user_ids.is_empty() {
        sentinel_shared::discord_helpers::reply_ephemeral(ctx, command, "Aucun ID utilisateur valide detecte.").await;
        return;
    }

    // Repondre immediatement (defer)
    command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!("Mute en cours de {} utilisateurs...", user_ids.len())),
        ),
    ).await.ok();

    let duration_secs = duration_min * 60;
    let mut success = 0u32;
    let mut failures = 0u32;

    let data = ctx.data.read().await;
    let api = data.get::<ModerationApiKey>().unwrap();

    for uid in &user_ids {
        let user_id = serenity::model::id::UserId::new(*uid);
        match guild_id.member(&ctx.http, user_id).await {
            Ok(mut member) => {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64 + duration_secs as i64;
                let datetime = time::OffsetDateTime::from_unix_timestamp(ts).expect("timestamp");
                let timeout = serenity::model::Timestamp::from(datetime);

                if member.disable_communication_until_datetime(&ctx.http, timeout).await.is_ok() {
                    success += 1;
                    api.log_action(&ModerationAction {
                        guild_id: guild_id.to_string(),
                        channel_id: command.channel_id.to_string(),
                        moderator_id: command.user.id.to_string(),
                        moderator_name: command.user.name.clone(),
                        target_id: uid.to_string(),
                        target_name: member.user.name.clone(),
                        action_type: "mute_temp".to_string(),
                        reason: reason.to_string(),
                        gravity: None,
                        duration: Some(duration_secs),
                    }).await.ok();
                } else {
                    failures += 1;
                }
            }
            Err(_) => failures += 1,
        }
    }

    let embed = danger_embed(format!("Mass Mute — {} utilisateurs", user_ids.len()))
        .field("Reussi", success.to_string(), true)
        .field("Echoue", failures.to_string(), true)
        .field("Duree", format!("{}min", duration_min), true)
        .field("Raison", reason, false);

    let _ = command.channel_id.send_message(
        &ctx.http,
        serenity::builder::CreateMessage::new().embed(embed),
    ).await;

    info!(
        moderator = %command.user.name,
        success, failures,
        total = user_ids.len(),
        "Mass mute execute"
    );
}

pub async fn handle_massban(ctx: &Context, command: &CommandInteraction) {
    let users_str = command.data.options.iter().find(|o| o.name == "users")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("");

    let reason = command.data.options.iter().find(|o| o.name == "reason")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("Mass ban");

    let guild_id = match command.guild_id {
        Some(id) => id,
        None => { sentinel_shared::discord_helpers::reply_ephemeral(ctx, command, "Commande serveur uniquement.").await; return; }
    };

    let user_ids = parse_user_ids(users_str);
    if user_ids.is_empty() {
        sentinel_shared::discord_helpers::reply_ephemeral(ctx, command, "Aucun ID utilisateur valide detecte.").await;
        return;
    }

    command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(format!("Ban en cours de {} utilisateurs...", user_ids.len())),
        ),
    ).await.ok();

    let mut success = 0u32;
    let mut failures = 0u32;

    let data = ctx.data.read().await;
    let api = data.get::<ModerationApiKey>().unwrap();

    for uid in &user_ids {
        let user_id = serenity::model::id::UserId::new(*uid);
        let target_name = user_id.to_user(&ctx.http).await
            .map(|u| u.name.clone())
            .unwrap_or_else(|_| uid.to_string());

        if guild_id.ban_with_reason(&ctx.http, user_id, 1, reason).await.is_ok() {
            success += 1;
            api.log_action(&ModerationAction {
                guild_id: guild_id.to_string(),
                channel_id: command.channel_id.to_string(),
                moderator_id: command.user.id.to_string(),
                moderator_name: command.user.name.clone(),
                target_id: uid.to_string(),
                target_name,
                action_type: "ban_permanent".to_string(),
                reason: reason.to_string(),
                gravity: None,
                duration: None,
            }).await.ok();
        } else {
            failures += 1;
        }
    }

    let embed = danger_embed(format!("Mass Ban — {} utilisateurs", user_ids.len()))
        .field("Reussi", success.to_string(), true)
        .field("Echoue", failures.to_string(), true)
        .field("Raison", reason, false);

    let _ = command.channel_id.send_message(
        &ctx.http,
        serenity::builder::CreateMessage::new().embed(embed),
    ).await;

    info!(
        moderator = %command.user.name,
        success, failures,
        total = user_ids.len(),
        "Mass ban execute"
    );
}

/// Parse les IDs utilisateurs depuis une chaine (espaces, virgules, ou les deux).
pub fn parse_user_ids(input: &str) -> Vec<u64> {
    input
        .split(|c: char| c == ',' || c == ' ' || c == '\n')
        .filter_map(|s| {
            let trimmed = s.trim().trim_start_matches("<@").trim_start_matches('!').trim_end_matches('>');
            trimmed.parse::<u64>().ok()
        })
        .collect()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_space_separated() {
        let ids = parse_user_ids("123456 789012 345678");
        assert_eq!(ids, vec![123456, 789012, 345678]);
    }

    #[test]
    fn parse_comma_separated() {
        let ids = parse_user_ids("123456,789012,345678");
        assert_eq!(ids, vec![123456, 789012, 345678]);
    }

    #[test]
    fn parse_mixed_separators() {
        let ids = parse_user_ids("123456, 789012 345678");
        assert_eq!(ids, vec![123456, 789012, 345678]);
    }

    #[test]
    fn parse_mention_format() {
        let ids = parse_user_ids("<@123456> <@!789012>");
        assert_eq!(ids, vec![123456, 789012]);
    }

    #[test]
    fn parse_ignores_invalid() {
        let ids = parse_user_ids("123456 invalid 789012 abc");
        assert_eq!(ids, vec![123456, 789012]);
    }

    #[test]
    fn parse_empty() {
        assert!(parse_user_ids("").is_empty());
        assert!(parse_user_ids("   ").is_empty());
    }

    #[test]
    fn parse_single() {
        assert_eq!(parse_user_ids("123456"), vec![123456]);
    }

    #[test]
    fn parse_with_newlines() {
        let ids = parse_user_ids("123456\n789012\n345678");
        assert_eq!(ids, vec![123456, 789012, 345678]);
    }
}
