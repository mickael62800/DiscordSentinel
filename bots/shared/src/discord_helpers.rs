/// Helpers Discord partages entre les bots.
/// Evite la duplication des reply_text, reply_ephemeral, etc.

use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateEmbed,
    CreateInteractionResponse, CreateInteractionResponseMessage,
};

/// Reponse ephemere texte a une slash command.
pub async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
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
        .ok();
}

/// Reponse ephemere embed a une slash command.
pub async fn reply_ephemeral_embed(ctx: &Context, command: &CommandInteraction, embed: CreateEmbed) {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(true),
            ),
        )
        .await
        .ok();
}

/// Reponse publique embed a une slash command.
pub async fn reply_embed(ctx: &Context, command: &CommandInteraction, embed: CreateEmbed) {
    command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed),
            ),
        )
        .await
        .ok();
}

/// Reponse ephemere texte a un component interaction (bouton/menu).
pub async fn component_reply_ephemeral(ctx: &Context, component: &ComponentInteraction, content: &str) {
    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
        .ok();
}

/// Reponse ephemere embed a un component interaction.
pub async fn component_reply_embed(ctx: &Context, component: &ComponentInteraction, embed: CreateEmbed) {
    component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(true),
            ),
        )
        .await
        .ok();
}

/// Verifie si le bot est active pour un guild. Charge la config et check "enabled".
pub async fn is_bot_enabled(
    api: &crate::api_client::BaseApiClient,
    guild_id: &str,
) -> bool {
    let config = api.get_guild_config(guild_id).await.unwrap_or_default();
    crate::api_client::BaseApiClient::config_bool(&config, "enabled", true)
}
