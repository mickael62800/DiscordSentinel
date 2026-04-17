/// Helpers Discord partages entre les bots.
/// Evite la duplication des reply_text, reply_ephemeral, etc.

use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateEmbed,
    CreateInteractionResponse, CreateInteractionResponseFollowup,
    CreateInteractionResponseMessage,
};
use tracing::warn;

/// Defer une slash command en mode ephemere.
/// A appeler en tout debut de handler si le traitement peut depasser 3s.
/// Apres un defer, utiliser `followup_ephemeral_embed` au lieu de `reply_*`.
pub async fn defer_ephemeral(ctx: &Context, command: &CommandInteraction) {
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, command = %command.data.name, "Echec defer ephemere");
    }
}

/// Followup ephemere embed apres un `defer_ephemeral`.
pub async fn followup_ephemeral_embed(ctx: &Context, command: &CommandInteraction, embed: CreateEmbed) {
    if let Err(e) = command
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .embed(embed)
                .ephemeral(true),
        )
        .await
    {
        warn!(error = %e, command = %command.data.name, "Echec followup ephemere embed");
    }
}

/// Reponse ephemere texte a une slash command.
pub async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
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
        warn!(error = %e, command = %command.data.name, "Echec reponse ephemere texte");
    }
}

/// Reponse ephemere embed a une slash command.
pub async fn reply_ephemeral_embed(ctx: &Context, command: &CommandInteraction, embed: CreateEmbed) {
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, command = %command.data.name, "Echec reponse ephemere embed");
    }
}

/// Reponse publique embed a une slash command.
pub async fn reply_embed(ctx: &Context, command: &CommandInteraction, embed: CreateEmbed) {
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed),
            ),
        )
        .await
    {
        warn!(error = %e, command = %command.data.name, "Echec reponse embed");
    }
}

/// Reponse ephemere texte a un component interaction (bouton/menu).
pub async fn component_reply_ephemeral(ctx: &Context, component: &ComponentInteraction, content: &str) {
    if let Err(e) = component
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
        warn!(error = %e, "Echec reponse composant ephemere texte");
    }
}

/// Reponse ephemere embed a un component interaction.
pub async fn component_reply_embed(ctx: &Context, component: &ComponentInteraction, embed: CreateEmbed) {
    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Echec reponse composant ephemere embed");
    }
}

/// Verifie si le bot est active pour un guild. Charge la config et check "enabled".
pub async fn is_bot_enabled(
    api: &crate::api_client::BaseApiClient,
    guild_id: &str,
) -> bool {
    let config = match api.get_guild_config(guild_id).await {
        Ok(cfg) => cfg,
        Err(e) => {
            warn!(guild_id = %guild_id, error = %e, "Impossible de verifier si le bot est active, presume actif");
            return true;
        }
    };
    crate::api_client::BaseApiClient::config_bool(&config, "enabled", true)
}

/// Variante de `is_bot_enabled` qui extrait l'ApiClient du TypeMap directement.
/// Retourne `true` par defaut si l'ApiClient est absent ou l'appel API echoue
/// (fail-open : on prefere laisser passer que bloquer tout le bot).
pub async fn is_module_enabled(ctx: &Context, guild_id: &str) -> bool {
    let data = ctx.data.read().await;
    match data.get::<crate::heartbeat::ApiClientKey>() {
        Some(api) => is_bot_enabled(api, guild_id).await,
        None => true,
    }
}

/// Charge la config guild ou retourne une HashMap vide si indisponible.
/// Factorise le pattern `get::<ApiClientKey>() + get_guild_config()` present
/// dans la plupart des handlers de modules.
pub async fn guild_config_or_default(
    ctx: &Context,
    guild_id: &str,
) -> std::collections::HashMap<String, String> {
    let data = ctx.data.read().await;
    let Some(api) = data.get::<crate::heartbeat::ApiClientKey>() else {
        return std::collections::HashMap::new();
    };
    api.get_guild_config(guild_id).await.unwrap_or_default()
}
