use serenity::all::{
    CommandDataOption, CommandDataOptionValue, CommandInteraction, Context,
    CreateInteractionResponse, CreateInteractionResponseMessage, ChannelId,
};
use super::constants::*;

// ── Helpers ──

/// Extrait l'UUID du ticket depuis le topic du salon.
pub fn extract_ticket_id_from_topic(topic: &str) -> Option<&str> {
    let start = topic.find("[ticket:")? + "[ticket:".len();
    let end = topic[start..].find(']')? + start;
    let id = &topic[start..end];
    if id.is_empty() { None } else { Some(id) }
}

/// Recupere l'UUID du ticket depuis le topic d'un salon Discord.
pub async fn get_ticket_id_from_channel(ctx: &Context, channel_id: ChannelId) -> Option<String> {
    let channel = channel_id.to_channel(&ctx.http).await.ok()?;
    let guild_channel = channel.guild()?;
    let topic = guild_channel.topic.as_deref()?;
    extract_ticket_id_from_topic(topic).map(|s| s.to_string())
}

/// Verifie si un custom_id correspond a un modal de ticket
pub fn is_ticket_modal(custom_id: &str) -> bool {
    custom_id.starts_with(MODAL_ID_PREFIX)
}

/// Verifie si un utilisateur est admin ou moderateur dans une guild.
pub async fn is_staff_member(
    ctx: &Context,
    guild_id: serenity::model::id::GuildId,
    user_id: serenity::model::id::UserId,
) -> bool {
    match guild_id.member(&ctx.http, user_id).await {
        Ok(member) => {
            if let Some(guild) = guild_id.to_guild_cached(&ctx.cache) {
                let permissions = guild.member_permissions(&member);
                permissions.manage_messages() || permissions.administrator()
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

pub fn get_sub_options(command: &CommandInteraction) -> &[CommandDataOption] {
    match &command.data.options[0].value {
        CommandDataOptionValue::SubCommand(opts) => opts,
        _ => &[],
    }
}

pub async fn reply(
    ctx: &Context,
    command: &CommandInteraction,
    content: &str,
) -> Result<(), serenity::Error> {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ticket_id_valid() {
        let topic = "[ticket:550e8400-e29b-41d4-a716-446655440000] Question — testuser";
        let id = extract_ticket_id_from_topic(topic).unwrap();
        assert_eq!(id, "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_extract_ticket_id_empty() {
        let topic = "[ticket:] Vide";
        assert!(extract_ticket_id_from_topic(topic).is_none());
    }

    #[test]
    fn test_is_ticket_modal_valid() {
        assert!(is_ticket_modal("sentinel_ticket_modal:probleme_serveur"));
    }

    #[test]
    fn test_is_ticket_modal_invalid() {
        assert!(!is_ticket_modal("sentinel_ticket_create"));
    }
}
