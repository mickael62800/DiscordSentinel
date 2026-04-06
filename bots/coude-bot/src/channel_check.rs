use serenity::all::{
    CommandInteraction, Context, CreateInteractionResponse, CreateInteractionResponseMessage,
};

/// Verifie que la commande est utilisee dans le bon salon.
/// Retourne `true` si OK, `false` si bloque (reponse ephemerale deja envoyee).
pub async fn check_channel(
    ctx: &Context,
    command: &CommandInteraction,
    allowed_channel: Option<String>,
) -> bool {
    match allowed_channel {
        None => {
            reply(ctx, command, "Cette commande n'est pas configuree sur ce serveur.").await;
            false
        }
        Some(ref channel_id) => {
            if command.channel_id.to_string() == *channel_id {
                true
            } else {
                reply(
                    ctx,
                    command,
                    &format!("Utilise cette commande dans <#{}>.", channel_id),
                )
                .await;
                false
            }
        }
    }
}

async fn reply(ctx: &Context, command: &CommandInteraction, content: &str) {
    let _ = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await;
}
