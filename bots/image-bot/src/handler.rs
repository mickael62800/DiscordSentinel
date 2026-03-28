use std::sync::Arc;

use dashmap::DashSet;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::MessageId;
use serenity::prelude::*;
use tracing::{error, info, warn};

use crate::api_client::{Action, AnalyzeImageRequest, ApiClient};

// ── TypeMap keys ──

pub struct ApiClientKey;
impl TypeMapKey for ApiClientKey {
    type Value = ApiClient;
}

pub struct ProcessedMessagesKey;
impl TypeMapKey for ProcessedMessagesKey {
    type Value = Arc<DashSet<MessageId>>;
}

/// Extensions d'images supportees.
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "bmp",
];

/// Content types images supportes.
const SUPPORTED_CONTENT_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/bmp",
];

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        // Ignorer les bots
        if msg.author.bot {
            return;
        }

        // Verifier si le message contient des images
        let image_attachments: Vec<_> = msg
            .attachments
            .iter()
            .filter(|a| is_image_attachment(a))
            .collect();

        if image_attachments.is_empty() {
            // Verifier aussi les embeds avec images
            let has_embed_images = msg.embeds.iter().any(|e| {
                e.image.is_some() || e.thumbnail.is_some()
            });

            if !has_embed_images {
                return;
            }
        }

        // Deduplication
        {
            let data = ctx.data.read().await;
            if let Some(processed) = data.get::<ProcessedMessagesKey>() {
                if !processed.insert(msg.id) {
                    return;
                }
                // Nettoyage periodique
                if processed.len() > 1000 {
                    let to_remove: Vec<_> = processed.iter().take(500).map(|e| *e).collect();
                    for id in to_remove {
                        processed.remove(&id);
                    }
                }
            }
        }

        let guild_id = msg.guild_id.map(|id| id.to_string()).unwrap_or_default();

        info!(
            guild_id = %guild_id,
            user = %msg.author.name,
            attachments = image_attachments.len(),
            "Image(s) detectee(s)"
        );

        // Traiter chaque image du message
        for attachment in &image_attachments {
            process_image_attachment(&ctx, &msg, &attachment.url, &attachment.filename, &guild_id)
                .await;
        }

        // Traiter les images dans les embeds
        for embed in &msg.embeds {
            if let Some(ref image) = embed.image {
                process_image_attachment(&ctx, &msg, &image.url, "embed_image", &guild_id).await;
            }
            if let Some(ref thumb) = embed.thumbnail {
                process_image_attachment(&ctx, &msg, &thumb.url, "embed_thumbnail", &guild_id)
                    .await;
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, "Image bot connecte");

        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            api.send_log("info", "", "Image bot demarre");
            for guild_status in &ready.guilds {
                let guild_id = guild_status.id;
                if let Ok(guild) = guild_id.to_partial_guild(&ctx.http).await {
                    let member_count = guild.approximate_member_count.unwrap_or(0) as i32;
                    if let Err(e) = api
                        .register_guild(&guild_id.to_string(), &guild.name, member_count)
                        .await
                    {
                        warn!(error = %e, guild = %guild.name, "Erreur enregistrement guild");
                    } else {
                        info!(guild = %guild.name, "Guild enregistree");
                    }
                }
            }
        }
    }
}

/// Verifie si un attachment est une image supportee.
fn is_image_attachment(attachment: &serenity::model::channel::Attachment) -> bool {
    // Verifier le content_type
    if let Some(ref ct) = attachment.content_type {
        if SUPPORTED_CONTENT_TYPES.iter().any(|t| ct.starts_with(t)) {
            return true;
        }
    }

    // Fallback : verifier l'extension
    let filename = attachment.filename.to_lowercase();
    SUPPORTED_EXTENSIONS
        .iter()
        .any(|ext| filename.ends_with(ext))
}

/// Telecharge et envoie une image a l'API pour analyse.
async fn process_image_attachment(
    ctx: &Context,
    msg: &Message,
    image_url: &str,
    filename: &str,
    guild_id: &str,
) {
    let data = ctx.data.read().await;
    let api_client = match data.get::<ApiClientKey>() {
        Some(client) => client,
        None => {
            error!("ApiClient introuvable dans le contexte");
            return;
        }
    };

    // Telecharger l'image
    let image_bytes = match api_client.download_image(image_url).await {
        Ok(bytes) => bytes,
        Err(e) => {
            warn!(error = %e, url = %image_url, "Erreur telechargement image");
            return;
        }
    };

    // Verifier la taille
    if image_bytes.len() as u64 > api_client.max_image_size() {
        warn!(
            size = image_bytes.len(),
            max = api_client.max_image_size(),
            "Image trop volumineuse, ignoree"
        );
        return;
    }

    // Encoder en base64
    use base64::Engine;
    let image_b64 = base64::engine::general_purpose::STANDARD.encode(&image_bytes);

    // Determiner le content type
    let content_type = detect_content_type(filename, &image_bytes);

    let request = AnalyzeImageRequest {
        guild_id: guild_id.to_string(),
        channel_id: msg.channel_id.to_string(),
        user_id: msg.author.id.to_string(),
        username: msg.author.name.clone(),
        message_id: msg.id.to_string(),
        image_data: image_b64,
        content_type,
        filename: filename.to_string(),
    };

    match api_client.analyze_image(&request).await {
        Ok(response) => {
            info!(
                action = ?response.action,
                reason = ?response.reason,
                classifications = ?response.classifications,
                filename = %filename,
                "Reponse analyse image"
            );

            if response.action != Action::None {
                api_client.send_log("warn", &guild_id.to_string(), &format!(
                    "Image detectee — {} par {} : {:?} ({})",
                    response.action.as_str(), msg.author.name,
                    response.classifications, response.reason.as_deref().unwrap_or("Automod")
                ));
            }

            if let Err(e) = execute_action(ctx, msg, &response.action, response.reason.as_deref())
                .await
            {
                error!(error = %e, "Erreur execution action image");
                api_client.send_log("error", &guild_id.to_string(), &format!(
                    "Erreur action image : {}", e
                ));
            }
        }
        Err(e) => {
            warn!(error = %e, "Backend injoignable — suppression preventive de l'image");
            api_client.send_log("error", &guild_id.to_string(), &format!(
                "Backend injoignable pour analyse image : {}", e
            ));
            // Fallback : en cas de doute sur une image et API down, on supprime
            let _ = msg.delete(&ctx.http).await;
            let _ = msg
                .channel_id
                .say(
                    &ctx.http,
                    format!(
                        "<@{}> Ton image a ete supprimee (verification impossible).",
                        msg.author.id
                    ),
                )
                .await;
        }
    }
}

/// Detecte le content type depuis l'extension ou les magic bytes.
fn detect_content_type(filename: &str, bytes: &[u8]) -> String {
    // Magic bytes
    if bytes.len() >= 4 {
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return "image/jpeg".to_string();
        }
        if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            return "image/png".to_string();
        }
        if bytes.starts_with(b"GIF8") {
            return "image/gif".to_string();
        }
        if bytes.starts_with(b"RIFF") && bytes.len() >= 12 && &bytes[8..12] == b"WEBP" {
            return "image/webp".to_string();
        }
    }

    // Fallback extension
    let lower = filename.to_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".bmp") {
        "image/bmp"
    } else {
        "image/jpeg"
    }
    .to_string()
}

/// Execute l'action decidee par le backend.
async fn execute_action(
    ctx: &Context,
    msg: &Message,
    action: &Action,
    reason: Option<&str>,
) -> Result<(), serenity::Error> {
    let reason_text = reason.unwrap_or("Contenu d'image interdit");

    match action {
        Action::None => {}
        Action::Warn => {
            msg.reply(
                &ctx.http,
                format!(
                    "<@{}> Avertissement : {reason_text}",
                    msg.author.id
                ),
            )
            .await?;
            info!(user = %msg.author.name, "Avertissement image envoye");
        }
        Action::Delete => {
            let _ = msg
                .channel_id
                .say(
                    &ctx.http,
                    format!(
                        "<@{}> Ton image a ete supprimee. Raison : {reason_text}",
                        msg.author.id
                    ),
                )
                .await;
            msg.delete(&ctx.http).await?;
            info!(message_id = %msg.id, "Message avec image supprime");
        }
        Action::Mute => {
            let mute_duration_secs: u64 = 600; // 10 min par defaut
            let mute_minutes = mute_duration_secs / 60;
            let _ = msg
                .channel_id
                .say(
                    &ctx.http,
                    format!(
                        "<@{}> Tu as ete mute {mute_minutes} minutes pour image interdite. Raison : {reason_text}",
                        msg.author.id
                    ),
                )
                .await;
            msg.delete(&ctx.http).await?;

            if let (Some(guild_id), Ok(_member)) = (msg.guild_id, msg.member(&ctx.http).await) {
                let mut member = guild_id.member(&ctx.http, msg.author.id).await?;
                let secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
                    + mute_duration_secs as i64;
                let datetime =
                    time::OffsetDateTime::from_unix_timestamp(secs).expect("timestamp invalide");
                let timeout = serenity::model::Timestamp::from(datetime);
                member
                    .disable_communication_until_datetime(&ctx.http, timeout)
                    .await?;
                info!(user = %msg.author.name, "Utilisateur mute pour image");
            }
        }
        Action::Ban => {
            if let Some(guild_id) = msg.guild_id {
                let _ = msg
                    .channel_id
                    .say(
                        &ctx.http,
                        format!(
                            "<@{}> Tu as ete banni pour image interdite. Raison : {reason_text}",
                            msg.author.id
                        ),
                    )
                    .await;
                guild_id
                    .ban_with_reason(&ctx.http, msg.author.id, 1, reason_text)
                    .await?;
                info!(user = %msg.author.name, "Utilisateur banni pour image");
            }
        }
    }

    Ok(())
}
