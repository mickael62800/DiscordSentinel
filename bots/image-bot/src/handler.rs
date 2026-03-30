use std::sync::Arc;

use dashmap::DashSet;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::MessageId;
use serenity::prelude::*;
use serenity::all::CreateMessage;
use tracing::{error, info, warn};

use sentinel_shared::embeds::{warn_embed, moderate_embed, danger_embed, critical_embed};
use sentinel_shared::heartbeat::{ApiClientKey, register_guilds};

use crate::api_client::{Action, AnalyzeImageRequest, ApiClient, Classification};

// ── TypeMap keys ──

pub struct ProcessedMessagesKey;
impl TypeMapKey for ProcessedMessagesKey {
    type Value = Arc<DashSet<MessageId>>;
}

pub struct MaxImageSizeKey;
impl TypeMapKey for MaxImageSizeKey {
    type Value = u64;
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
        register_guilds(&ctx, &ready).await;
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
    let (base, max_image_size) = {
        let data = ctx.data.read().await;
        let base = match data.get::<ApiClientKey>() {
            Some(client) => Arc::clone(client),
            None => {
                error!("BaseApiClient introuvable dans le contexte");
                return;
            }
        };
        let max_size = data.get::<MaxImageSizeKey>().copied().unwrap_or(10 * 1024 * 1024);
        (base, max_size)
    };

    let api_client = ApiClient::new(Arc::clone(&base), max_image_size);

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
                base.send_log("warn", guild_id, &format!(
                    "Image detectee — {} par {} : {:?} ({})",
                    response.action.as_str(), msg.author.name,
                    response.classifications, response.reason.as_deref().unwrap_or("Automod")
                ));
            }

            if let Err(e) = execute_action(
                ctx,
                msg,
                &response.action,
                response.reason.as_deref(),
                &response.classifications,
                response.duration,
            )
                .await
            {
                error!(error = %e, "Erreur execution action image");
                base.send_log("error", guild_id, &format!(
                    "Erreur action image : {}", e
                ));
            }
        }
        Err(e) => {
            warn!(error = %e, "Backend injoignable — suppression preventive de l'image");
            base.send_log("error", guild_id, &format!(
                "Backend injoignable pour analyse image : {}", e
            ));
            // Fallback : en cas de doute sur une image et API down, on supprime
            let _ = msg.delete(&ctx.http).await;
            let embed = moderate_embed("⚠\u{fe0f} Image supprimee preventivement")
                .description(format!("<@{}>", msg.author.id))
                .field("📋 Raison", "API de verification indisponible — l'image a ete supprimee par precaution.", false)
                .thumbnail(msg.author.face());
            let _ = msg
                .channel_id
                .send_message(&ctx.http, CreateMessage::new().embed(embed))
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

/// Formate les classifications en une chaine lisible.
fn format_classifications(classifications: &[Classification]) -> String {
    if classifications.is_empty() {
        return "Aucune".to_string();
    }
    classifications
        .iter()
        .map(|c| format!("{} ({:.0}%)", c.label, c.confidence * 100.0))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Execute l'action decidee par le backend.
async fn execute_action(
    ctx: &Context,
    msg: &Message,
    action: &Action,
    reason: Option<&str>,
    classifications: &[Classification],
    duration: Option<u64>,
) -> Result<(), serenity::Error> {
    let reason_text = reason.unwrap_or("Contenu d'image interdit");
    let detection_text = format_classifications(classifications);

    match action {
        Action::None => {}
        Action::Warn => {
            let embed = warn_embed("⚠\u{fe0f} Avertissement — Image")
                .description(format!("<@{}>", msg.author.id))
                .field("📝 Raison", reason_text, false)
                .field("🏷\u{fe0f} Detection", &detection_text, false)
                .thumbnail(msg.author.face());
            msg.channel_id
                .send_message(&ctx.http, CreateMessage::new().embed(embed))
                .await?;
            info!(user = %msg.author.name, "Avertissement image envoye");
        }
        Action::Delete => {
            let embed = moderate_embed("🗑\u{fe0f} Image supprimee")
                .description(format!("<@{}>", msg.author.id))
                .field("📝 Raison", reason_text, false)
                .field("🏷\u{fe0f} Detection", &detection_text, false)
                .thumbnail(msg.author.face());
            let _ = msg
                .channel_id
                .send_message(&ctx.http, CreateMessage::new().embed(embed))
                .await;
            msg.delete(&ctx.http).await?;
            info!(message_id = %msg.id, "Message avec image supprime");
        }
        Action::Mute => {
            let mute_duration_secs: u64 = duration.unwrap_or(600); // 10 min par defaut
            let mute_minutes = mute_duration_secs / 60;
            let embed = danger_embed("🔇 Mute — Image interdite")
                .description(format!("<@{}>", msg.author.id))
                .field("📝 Raison", reason_text, false)
                .field("⏱\u{fe0f} Duree", format!("{mute_minutes} minutes"), false)
                .thumbnail(msg.author.face());
            let _ = msg
                .channel_id
                .send_message(&ctx.http, CreateMessage::new().embed(embed))
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
                let embed = critical_embed("🔨 Ban — Image interdite")
                    .description(format!("<@{}>", msg.author.id))
                    .field("📝 Raison", reason_text, false)
                    .thumbnail(msg.author.face());
                let _ = msg
                    .channel_id
                    .send_message(&ctx.http, CreateMessage::new().embed(embed))
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
