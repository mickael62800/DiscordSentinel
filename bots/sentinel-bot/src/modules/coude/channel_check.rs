use serenity::all::{
    CommandInteraction, Context, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage, CreateMessage,
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
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
    {
        tracing::warn!(error = %e, "Echec response Discord");
    }
}

/// Poste un embed dans le salon d'activites configure.
/// Si pas configure, poste dans le salon courant via create_response.
/// Retourne true si l'embed a ete poste dans le salon d'activites (la reponse reste a faire).
pub async fn post_activity(
    ctx: &Context,
    command: &CommandInteraction,
    activity_channel: Option<String>,
    embed: CreateEmbed,
) {
    match activity_channel.and_then(|id| id.parse::<u64>().ok()) {
        Some(ch_id) => {
            // Poster dans le salon activites
            let channel = serenity::model::id::ChannelId::new(ch_id);
            if let Err(e) = channel.send_message(
                &ctx.http,
                CreateMessage::new().embed(embed),
            ).await {
                tracing::warn!(error = %e, "Echec send_message salon activites");
            }
            // Repondre en ephemere pour confirmer
            if let Err(e) = command.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!("Resultat poste dans <#{}>.", ch_id))
                        .ephemeral(true),
                ),
            ).await {
                tracing::warn!(error = %e, "Echec response Discord");
            }
        }
        None => {
            // Pas de salon configure → poster ici
            if let Err(e) = command.create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(embed),
                ),
            ).await {
                tracing::warn!(error = %e, "Echec response Discord");
            }
        }
    }
}

/// Variante de `post_activity` pour les commandes qui ont deja defer
/// l'interaction. Utilise `create_followup` au lieu de `create_response`.
pub async fn post_activity_followup(
    ctx: &Context,
    command: &CommandInteraction,
    activity_channel: Option<String>,
    embed: CreateEmbed,
) {
    match activity_channel.and_then(|id| id.parse::<u64>().ok()) {
        Some(ch_id) => {
            let channel = serenity::model::id::ChannelId::new(ch_id);
            if let Err(e) = channel.send_message(
                &ctx.http,
                CreateMessage::new().embed(embed),
            ).await {
                tracing::warn!(error = %e, "Echec send_message salon activites (followup)");
            }
            // Followup ephemeral pour confirmer au joueur
            if let Err(e) = command.create_followup(
                &ctx.http,
                CreateInteractionResponseFollowup::new()
                    .content(format!("Resultat poste dans <#{}>.", ch_id))
                    .ephemeral(true),
            ).await {
                tracing::warn!(error = %e, "Echec followup Discord");
            }
        }
        None => {
            // Pas de salon configure → followup ici avec l'embed public
            if let Err(e) = command.create_followup(
                &ctx.http,
                CreateInteractionResponseFollowup::new().embed(embed),
            ).await {
                tracing::warn!(error = %e, "Echec followup Discord");
            }
        }
    }
}
