/// Helpers Discord partages entre les bots.
/// Evite la duplication des reply_text, reply_ephemeral, etc.

use serenity::all::{
    ChannelId, CommandInteraction, ComponentInteraction, Context, CreateEmbed,
    CreateInteractionResponse, CreateInteractionResponseFollowup,
    CreateInteractionResponseMessage, ModalInteraction,
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

/// Defer un component interaction en mode ephemere.
/// A appeler en tout debut de handler si le traitement peut depasser 3s.
/// Apres ce defer, utiliser `component_followup_ephemeral` pour repondre.
pub async fn component_defer_ephemeral(ctx: &Context, component: &ComponentInteraction) {
    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Echec defer composant ephemere");
    }
}

/// Defer un component interaction en mode UpdateMessage (acquitte
/// l'interaction sans rien envoyer de visible). Apres ce defer, utiliser
/// `component.edit_response(...)` ou `component.edit_message(...)` pour
/// modifier le message d'origine.
pub async fn component_defer_update(ctx: &Context, component: &ComponentInteraction) {
    if let Err(e) = component
        .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
        .await
    {
        warn!(error = %e, "Echec defer composant update");
    }
}

/// Followup ephemere texte apres `component_defer_ephemeral`.
pub async fn component_followup_ephemeral(
    ctx: &Context,
    component: &ComponentInteraction,
    content: &str,
) {
    if let Err(e) = component
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .content(content)
                .ephemeral(true),
        )
        .await
    {
        warn!(error = %e, "Echec followup composant ephemere texte");
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

/// Verifie si le module est active pour un guild. Charge la config sous le
/// `bot_name` du module (ex: "coude-bot") et check la cle "enabled".
pub async fn is_bot_enabled(
    api: &crate::api_client::BaseApiClient,
    guild_id: &str,
    module_bot_name: &str,
) -> bool {
    let config = match api.get_guild_config_for(guild_id, module_bot_name).await {
        Ok(cfg) => cfg,
        Err(e) => {
            warn!(guild_id = %guild_id, module = %module_bot_name, error = %e, "Impossible de verifier si le module est active, presume actif");
            return true;
        }
    };
    crate::api_client::BaseApiClient::config_bool(&config, "enabled", true)
}

/// Variante de `is_bot_enabled` qui extrait l'ApiClient du TypeMap directement.
/// Retourne `true` par defaut si l'ApiClient est absent ou l'appel API echoue
/// (fail-open : on prefere laisser passer que bloquer tout le bot).
///
/// `module_bot_name` doit correspondre a une ligne dans `bot_definitions`
/// (ex: "coude-bot", "automod-bot", "voice-bot", ...).
pub async fn is_module_enabled(ctx: &Context, guild_id: &str, module_bot_name: &str) -> bool {
    let data = ctx.data.read().await;
    match data.get::<crate::heartbeat::ApiClientKey>() {
        Some(api) => is_bot_enabled(api, guild_id, module_bot_name).await,
        None => true,
    }
}

/// Variante de `is_module_enabled` qui, si desactive, repond en ephemeral
/// a la slash command et retourne false. Si actif, retourne true sans repondre.
pub async fn is_module_enabled_or_reply_command(
    ctx: &Context,
    command: &CommandInteraction,
    module_bot_name: &str,
) -> bool {
    let Some(guild_id) = command.guild_id else { return true; };
    if is_module_enabled(ctx, &guild_id.to_string(), module_bot_name).await {
        return true;
    }
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Cette fonctionnalite est desactivee sur ce serveur.")
                    .ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, command = %command.data.name, "Echec reponse module desactive (command)");
    }
    false
}

/// Idem pour les component interactions (boutons, select menus).
pub async fn is_module_enabled_or_reply_component(
    ctx: &Context,
    component: &ComponentInteraction,
    module_bot_name: &str,
) -> bool {
    let Some(guild_id) = component.guild_id else { return true; };
    if is_module_enabled(ctx, &guild_id.to_string(), module_bot_name).await {
        return true;
    }
    if let Err(e) = component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Cette fonctionnalite est desactivee sur ce serveur.")
                    .ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Echec reponse module desactive (component)");
    }
    false
}

/// Idem pour les modals.
pub async fn is_module_enabled_or_reply_modal(
    ctx: &Context,
    modal: &ModalInteraction,
    module_bot_name: &str,
) -> bool {
    let Some(guild_id) = modal.guild_id else { return true; };
    if is_module_enabled(ctx, &guild_id.to_string(), module_bot_name).await {
        return true;
    }
    if let Err(e) = modal
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Cette fonctionnalite est desactivee sur ce serveur.")
                    .ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, "Echec reponse module desactive (modal)");
    }
    false
}

/// Charge la config guild du module donne, ou retourne une HashMap vide si indisponible.
pub async fn guild_config_or_default(
    ctx: &Context,
    guild_id: &str,
    module_bot_name: &str,
) -> std::collections::HashMap<String, String> {
    let data = ctx.data.read().await;
    let Some(api) = data.get::<crate::heartbeat::ApiClientKey>() else {
        return std::collections::HashMap::new();
    };
    api.get_guild_config_for(guild_id, module_bot_name).await.unwrap_or_default()
}

/// Lit le `log_channel_id` dans la config guild du module donne.
pub async fn get_log_channel(ctx: &Context, guild_id: &str, module_bot_name: &str) -> Option<ChannelId> {
    let config = guild_config_or_default(ctx, guild_id, module_bot_name).await;
    config
        .get("log_channel_id")
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|id| *id > 0)
        .map(ChannelId::new)
}
