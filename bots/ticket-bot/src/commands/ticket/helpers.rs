use serenity::all::{
    CommandDataOption, CommandDataOptionValue, CommandInteraction, Context,
    CreateInteractionResponse, CreateInteractionResponseMessage, ChannelId,
};
use super::constants::*;

// ── Helpers ──

/// Extrait l'UUID du ticket depuis le topic du salon.
/// Le topic contient `[ticket:UUID]` au debut.
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

    // ── Tests extract_ticket_id_from_topic ──

    #[test]
    fn test_extract_ticket_id_valid() {
        let topic = "[ticket:550e8400-e29b-41d4-a716-446655440000] Question — testuser";
        let id = extract_ticket_id_from_topic(topic).unwrap();
        assert_eq!(id, "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_extract_ticket_id_no_bracket() {
        let topic = "Juste un topic normal sans ticket";
        assert!(extract_ticket_id_from_topic(topic).is_none());
    }

    #[test]
    fn test_extract_ticket_id_empty() {
        let topic = "[ticket:] Vide";
        assert!(extract_ticket_id_from_topic(topic).is_none());
    }

    #[test]
    fn test_extract_ticket_id_malformed() {
        let topic = "[ticket:abc";
        assert!(extract_ticket_id_from_topic(topic).is_none());
    }

    #[test]
    fn test_extract_ticket_id_middle_of_topic() {
        let topic = "Prefix [ticket:my-uuid-123] Suffix";
        let id = extract_ticket_id_from_topic(topic).unwrap();
        assert_eq!(id, "my-uuid-123");
    }

    // ── Tests is_ticket_modal ──

    #[test]
    fn test_is_ticket_modal_valid() {
        assert!(is_ticket_modal("sentinel_ticket_modal:probleme_serveur"));
        assert!(is_ticket_modal("sentinel_ticket_modal:question"));
    }

    #[test]
    fn test_is_ticket_modal_invalid() {
        assert!(!is_ticket_modal("sentinel_ticket_create"));
        assert!(!is_ticket_modal("other_modal"));
        assert!(!is_ticket_modal(""));
    }

    // ── Tests constantes types ──

    #[test]
    fn test_admin_only_types() {
        assert!(ADMIN_ONLY_TYPES.contains(&"probleme_moderateur"));
        assert!(!ADMIN_ONLY_TYPES.contains(&"question"));
        assert!(!ADMIN_ONLY_TYPES.contains(&"probleme_serveur"));
    }

    #[test]
    fn test_urgent_types() {
        assert!(URGENT_TYPES.contains(&"urgence_detresse"));
        assert!(!URGENT_TYPES.contains(&"question"));
        assert!(!URGENT_TYPES.contains(&"probleme_serveur"));
    }

    // ── Tests TICKET_TYPES ──

    #[test]
    fn test_ticket_types_count() {
        assert_eq!(TICKET_TYPES.len(), 7);
    }

    #[test]
    fn test_ticket_types_no_suggestion() {
        assert!(!TICKET_TYPES.iter().any(|(v, _, _)| *v == "suggestion"));
    }

    #[test]
    fn test_ticket_types_has_moderateur() {
        assert!(TICKET_TYPES.iter().any(|(v, _, _)| *v == "probleme_moderateur"));
    }

    #[test]
    fn test_ticket_types_has_urgence() {
        assert!(TICKET_TYPES.iter().any(|(v, _, _)| *v == "urgence_detresse"));
    }

    #[test]
    fn test_ticket_types_all_have_labels_and_descriptions() {
        for (value, label, desc) in TICKET_TYPES {
            assert!(!value.is_empty(), "value vide");
            assert!(!label.is_empty(), "label vide pour {value}");
            assert!(!desc.is_empty(), "description vide pour {value}");
        }
    }

    // ── Tests custom_id constants ──

    #[test]
    fn test_custom_ids_are_unique() {
        let ids = vec![
            PANEL_BUTTON_ID,
            TYPE_SELECT_ID,
            CLOSE_BUTTON_ID,
            INVITE_BUTTON_ID,
            INVITE_SELECT_ID,
            VOCAL_BUTTON_ID,
            VOCAL_USER_ACCEPT_ID,
            VOCAL_USER_DECLINE_ID,
            CLOSE_CONFIRM_ID,
            CLOSE_CANCEL_ID,
        ];
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(ids.len(), unique.len(), "Des custom_id sont en doublon");
    }

    #[test]
    fn test_custom_ids_start_with_sentinel() {
        let ids = vec![
            PANEL_BUTTON_ID, TYPE_SELECT_ID, CLOSE_BUTTON_ID,
            INVITE_BUTTON_ID, INVITE_SELECT_ID, VOCAL_BUTTON_ID,
            VOCAL_USER_ACCEPT_ID, VOCAL_USER_DECLINE_ID,
            CLOSE_CONFIRM_ID, CLOSE_CANCEL_ID,
        ];
        for id in ids {
            assert!(id.starts_with("sentinel_"), "'{id}' ne commence pas par sentinel_");
        }
    }
}
