/// Helpers Discord partages entre les bots.
/// Evite la duplication des reply_text, reply_ephemeral, etc.
use serenity::all::{
    ChannelId, Colour, CommandInteraction, ComponentInteraction, Context, CreateEmbed,
    CreateInteractionResponse, CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
    CreateMessage, ModalInteraction,
};
use tracing::warn;

/// Extrait le `guild_id` d'une slash command. Si la commande est utilisee
/// en DM (pas de guild), repond ephemerement et retourne `None`.
///
/// Pattern type au call site :
/// ```ignore
/// let Some(guild_id) = require_guild_id(ctx, command).await else { return; };
/// ```
///
/// Elimine le bloc `match command.guild_id { Some(id) => id.to_string(),
/// None => { reply_ephemeral(...); return; } }` duplique dans ~40 commandes.
pub async fn require_guild_id(ctx: &Context, command: &CommandInteraction) -> Option<String> {
    match command.guild_id {
        Some(id) => Some(id.to_string()),
        None => {
            reply_ephemeral(ctx, command, "Commande serveur uniquement.").await;
            None
        }
    }
}

/// Repond ephemerement avec le message d'erreur API standard.
///
/// Remplace le pattern `reply_ephemeral(ctx, command, &e.to_string()).await`
/// duplique ~42 fois dans les commandes du bot.
pub async fn reply_api_err<E: std::fmt::Display>(
    ctx: &Context,
    command: &CommandInteraction,
    e: E,
) {
    reply_ephemeral(ctx, command, &e.to_string()).await;
}

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
pub async fn followup_ephemeral_embed(
    ctx: &Context,
    command: &CommandInteraction,
    embed: CreateEmbed,
) {
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

/// Edit la reponse texte apres un defer (ex: defer + traitement long, puis reply texte).
/// Pattern courant dans les commandes de moderation apres un `defer_with_confirmation`.
pub async fn edit_response_text(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().content(content),
        )
        .await
    {
        warn!(error = %e, command = %command.data.name, "Echec edit response texte");
    }
}

/// Edit la reponse apres un defer avec un embed de feedback colore (cf.
/// `embeds::feedback_embed`). A utiliser pour les retours d'erreur/succes apres
/// `defer_with_confirmation`, quand on veut la coloration par severite plutot
/// que `edit_response_text` (texte brut).
pub async fn edit_response_feedback(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new()
                .embed(crate::shared::embeds::feedback_embed(content)),
        )
        .await
    {
        warn!(error = %e, command = %command.data.name, "Echec edit response feedback");
    }
}

/// Reponse ephemere texte a une slash command.
pub async fn reply_ephemeral(ctx: &Context, command: &CommandInteraction, content: &str) {
    if let Err(e) = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(crate::shared::embeds::feedback_embed(content))
                    .ephemeral(true),
            ),
        )
        .await
    {
        warn!(error = %e, command = %command.data.name, "Echec reponse ephemere texte");
    }
}

/// Reponse ephemere embed a une slash command.
pub async fn reply_ephemeral_embed(
    ctx: &Context,
    command: &CommandInteraction,
    embed: CreateEmbed,
) {
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

/// Reponse publique (non ephemere) embed a une slash command.
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
        warn!(error = %e, "Echec response Discord");
    }
}

/// Reponse ephemere texte a un component interaction (bouton/menu).
pub async fn component_reply_ephemeral(
    ctx: &Context,
    component: &ComponentInteraction,
    content: &str,
) {
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

/// Verifie si le module est active pour un guild. Charge la config sous le
/// `bot_name` du module (ex: "coude-bot") et check la cle "enabled".
pub async fn is_bot_enabled(
    api: &super::api_client::BaseApiClient,
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
    super::api_client::BaseApiClient::config_bool(&config, "enabled", true)
}

/// Variante de `is_bot_enabled` qui extrait l'ApiClient du TypeMap directement.
/// Retourne `true` par defaut si l'ApiClient est absent ou l'appel API echoue
/// (fail-open : on prefere laisser passer que bloquer tout le bot).
///
/// `module_bot_name` doit correspondre a une ligne dans `bot_definitions`
/// (ex: "coude-bot", "automod-bot", "voice-bot", ...).
pub async fn is_module_enabled(ctx: &Context, guild_id: &str, module_bot_name: &str) -> bool {
    let data = ctx.data.read().await;
    match data.get::<super::heartbeat::ApiClientKey>() {
        Some(api) => is_bot_enabled(api, guild_id, module_bot_name).await,
        None => true,
    }
}

/// Lit une sous-feature booleenne d'un module (ex: anomaly_enabled,
/// weekly_report_enabled, chaos_enabled). Cascade : si le module top-level
/// est OFF, retourne false directement. Sinon, lit la cle `feature_key`
/// dans la config guild + module, fallback sur `default_value`.
///
/// Pattern : pour un toggle UI sub-feature (depends_on enabled), ce helper
/// applique la meme logique cote bot pour stopper le job correspondant.
pub async fn is_feature_enabled(
    ctx: &Context,
    guild_id: &str,
    module_bot_name: &str,
    feature_key: &str,
    default_value: bool,
) -> bool {
    if !is_module_enabled(ctx, guild_id, module_bot_name).await {
        return false;
    }
    let data = ctx.data.read().await;
    let api = match data.get::<super::heartbeat::ApiClientKey>() {
        Some(api) => std::sync::Arc::clone(api),
        None => return default_value,
    };
    drop(data);
    let config = match api.get_guild_config_for(guild_id, module_bot_name).await {
        Ok(cfg) => cfg,
        Err(_) => return default_value,
    };
    super::api_client::BaseApiClient::config_bool(&config, feature_key, default_value)
}

/// Variante de `is_module_enabled` qui, si desactive, repond en ephemeral
/// a la slash command et retourne false. Si actif, retourne true sans repondre.
pub async fn is_module_enabled_or_reply_command(
    ctx: &Context,
    command: &CommandInteraction,
    module_bot_name: &str,
) -> bool {
    let Some(guild_id) = command.guild_id else {
        return true;
    };
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
    let Some(guild_id) = component.guild_id else {
        return true;
    };
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
    let Some(guild_id) = modal.guild_id else {
        return true;
    };
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
    let Some(api) = data.get::<super::heartbeat::ApiClientKey>() else {
        return std::collections::HashMap::new();
    };
    api.get_guild_config_for(guild_id, module_bot_name)
        .await
        .unwrap_or_default()
}

/// Lit une option String d'une slash command par son nom.
/// Remplace le boilerplate `options.iter().find(|o| o.name == name)
/// .and_then(|o| match &o.value { String(s) => Some(s.as_str()), _ => None })`.
pub fn option_str<'a>(
    options: &'a [serenity::all::CommandDataOption],
    name: &str,
) -> Option<&'a str> {
    options
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| match &o.value {
            serenity::all::CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
}

/// Lit une option Integer d'une slash command par son nom.
pub fn option_i64(options: &[serenity::all::CommandDataOption], name: &str) -> Option<i64> {
    options
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| match &o.value {
            serenity::all::CommandDataOptionValue::Integer(n) => Some(*n),
            _ => None,
        })
}

/// Lit une option Boolean d'une slash command par son nom.
pub fn option_bool(options: &[serenity::all::CommandDataOption], name: &str) -> Option<bool> {
    options
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| match &o.value {
            serenity::all::CommandDataOptionValue::Boolean(b) => Some(*b),
            _ => None,
        })
}

/// Lit une option User d'une slash command par son nom.
pub fn option_user(
    options: &[serenity::all::CommandDataOption],
    name: &str,
) -> Option<serenity::all::UserId> {
    options
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| match &o.value {
            serenity::all::CommandDataOptionValue::User(id) => Some(*id),
            _ => None,
        })
}

/// Lit un `ChannelId` depuis une cle arbitraire de la config guild du module.
/// Retourne `None` si la cle est absente, vide, ou ne parse pas en id > 0.
pub async fn get_channel_from_config(
    ctx: &Context,
    guild_id: &str,
    module_bot_name: &str,
    key: &str,
) -> Option<ChannelId> {
    let config = guild_config_or_default(ctx, guild_id, module_bot_name).await;
    config
        .get(key)
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|id| *id > 0)
        .map(ChannelId::new)
}

/// Lit le `log_channel_id` dans la config guild du module donne.
pub async fn get_log_channel(
    ctx: &Context,
    guild_id: &str,
    module_bot_name: &str,
) -> Option<ChannelId> {
    get_channel_from_config(ctx, guild_id, module_bot_name, "log_channel_id").await
}

/// Type de sanction pour la "card" compacte postee dans le salon dedie.
/// Chaque variante porte son libelle, son emoji et sa couleur de barre laterale.
#[derive(Debug, Clone, Copy)]
pub enum SanctionKind {
    Warn,
    Mute,
    Ban,
    // Aucune commande /kick n'existe encore ; variante prevue pour le jour ou
    // elle sera ajoutee (la card la supporte deja).
    #[allow(dead_code)]
    Kick,
}

impl SanctionKind {
    pub fn label(self) -> &'static str {
        match self {
            SanctionKind::Warn => "Warn",
            SanctionKind::Mute => "Mute",
            SanctionKind::Ban => "Ban",
            SanctionKind::Kick => "Kick",
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            SanctionKind::Warn => "⚠️",
            SanctionKind::Mute => "🔇",
            SanctionKind::Ban => "🔨",
            SanctionKind::Kick => "👢",
        }
    }

    pub fn color(self) -> Colour {
        match self {
            SanctionKind::Warn => Colour::new(0xF1C40F), // jaune
            SanctionKind::Mute => Colour::new(0xE67E22), // orange
            SanctionKind::Ban => Colour::new(0xE74C3C),  // rouge
            SanctionKind::Kick => Colour::new(0xD35400), // orange fonce
        }
    }
}

/// Poste une "card de sanction" compacte (embed 2 lignes, colore) dans le salon
/// `sanctions_log_channel_id` du **moderation-bot**. Best-effort : si le salon
/// n'est pas configure -> no-op ; si l'envoi echoue -> `warn!`, jamais de panic,
/// jamais de blocage de la sanction. Pas de fallback sur `log_channel_id`.
#[allow(clippy::too_many_arguments)]
pub async fn post_sanction_card(
    ctx: &Context,
    guild_id: &str,
    action: SanctionKind,
    target_id: u64,
    target_tag: Option<&str>,
    actor: &str,
    reason: &str,
    duration: Option<&str>,
) {
    let Some(channel) = get_channel_from_config(
        ctx,
        guild_id,
        crate::modules::moderation::MODULE_BOT_NAME,
        "sanctions_log_channel_id",
    )
    .await
    else {
        return;
    };

    // Ligne 1 : emoji + action + mention + id brut (+ tag optionnel, garde 1 ligne).
    let tag_suffix = target_tag.map(|t| format!(" — {t}")).unwrap_or_default();
    let line1 = format!(
        "{} {} appliqué — <@{}> (`{}`){}",
        action.emoji(),
        action.label(),
        target_id,
        target_id,
        tag_suffix
    );

    // Ligne 2 : acteur + raison (tronquee ~120 chars) + duree optionnelle.
    const MAX_REASON: usize = 120;
    let reason_trimmed = reason.trim();
    let reason_short = if reason_trimmed.chars().count() > MAX_REASON {
        let truncated: String = reason_trimmed.chars().take(MAX_REASON).collect();
        format!("{truncated}…")
    } else {
        reason_trimmed.to_string()
    };
    let mut line2 = format!("Par {actor} · Raison : {reason_short}");
    if let Some(d) = duration {
        line2.push_str(&format!(" · Durée : {d}"));
    }

    let embed = CreateEmbed::new()
        .description(format!("{line1}\n{line2}"))
        .colour(action.color());

    if let Err(e) = channel
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
    {
        warn!(error = %e, guild_id = %guild_id, "Echec envoi card de sanction");
    }
}
