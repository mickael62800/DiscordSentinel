use serenity::all::{
    CommandDataOptionValue, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage,
    GetMessages, MessageId,
};
use serenity::builder::CreateEmbed;
pub fn register() -> CreateCommand {
    CreateCommand::new("context")
        .description("Afficher les messages autour d'un message specifique")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "message_id", "ID du message cible")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "count", "Nombre de messages avant et apres (defaut: 5)")
                .min_int_value(1)
                .max_int_value(15),
        )
}

pub async fn handle(ctx: &Context, command: &CommandInteraction) {
    let message_id_str = command.data.options.iter().find(|o| o.name == "message_id")
        .and_then(|o| match &o.value { CommandDataOptionValue::String(s) => Some(s.as_str()), _ => None })
        .unwrap_or("");

    let count = command.data.options.iter().find(|o| o.name == "count")
        .and_then(|o| match &o.value { CommandDataOptionValue::Integer(i) => Some(*i as u8), _ => None })
        .unwrap_or(5);

    let message_id: u64 = match message_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            sentinel_shared::discord_helpers::reply_ephemeral(ctx, command, "ID de message invalide. Clic droit > Copier l'ID du message.").await;
            return;
        }
    };

    let target_id = MessageId::new(message_id);
    let channel_id = command.channel_id;

    // Recuperer les messages avant
    let before = channel_id
        .messages(&ctx.http, GetMessages::new().before(target_id).limit(count))
        .await
        .unwrap_or_default();

    // Recuperer le message cible
    let target_msg = channel_id.message(&ctx.http, target_id).await.ok();

    // Recuperer les messages apres
    let after = channel_id
        .messages(&ctx.http, GetMessages::new().after(target_id).limit(count))
        .await
        .unwrap_or_default();

    // Construire le contexte (avant en ordre chronologique + cible + apres)
    let mut context_lines = Vec::new();

    // Before est retourne du plus recent au plus ancien → reverse
    for msg in before.iter().rev() {
        let time = format_timestamp(&msg.timestamp);
        let preview = truncate(&msg.content, 100);
        context_lines.push(format!("`[{}]` **@{}**: {}", time, msg.author.name, preview));
    }

    // Message cible (en gras/surligne)
    if let Some(ref msg) = target_msg {
        let time = format_timestamp(&msg.timestamp);
        let preview = truncate(&msg.content, 100);
        context_lines.push(format!("> **`[{}]` @{}: {}** ← cible", time, msg.author.name, preview));
    } else {
        context_lines.push("> *(message introuvable ou supprime)* ← cible".to_string());
    }

    // After est retourne du plus ancien au plus recent (dans l'ordre)
    // Mais serenity retourne after() du plus recent au plus ancien aussi → reverse
    for msg in after.iter().rev() {
        let time = format_timestamp(&msg.timestamp);
        let preview = truncate(&msg.content, 100);
        context_lines.push(format!("`[{}]` **@{}**: {}", time, msg.author.name, preview));
    }

    if context_lines.is_empty() {
        sentinel_shared::discord_helpers::reply_ephemeral(ctx, command, "Aucun message trouve autour de cet ID.").await;
        return;
    }

    let description = context_lines.join("\n");

    let embed = CreateEmbed::new()
        .title(format!("Contexte autour du message {}", message_id_str))
        .description(if description.len() > 4000 {
            format!("{}...", &description[..3997])
        } else {
            description
        })
        .color(0x3498db)
        .footer(serenity::builder::CreateEmbedFooter::new(
            format!("{} messages avant + cible + {} messages apres", count, count),
        ));

    command.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().embed(embed).ephemeral(true),
        ),
    ).await.ok();
}

fn format_timestamp(ts: &serenity::model::Timestamp) -> String {
    let dt = *ts;
    let hour = (dt.unix_timestamp() % 86400) / 3600;
    let min = (dt.unix_timestamp() % 3600) / 60;
    format!("{:02}:{:02}", hour, min)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.min(s.len())])
    } else {
        s.to_string()
    }
}

