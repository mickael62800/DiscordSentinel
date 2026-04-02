use std::sync::Arc;

use dashmap::DashSet;
use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::MessageId;
use serenity::prelude::*;
use serenity::all::CreateMessage;
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::embeds::{warn_embed, moderate_embed, danger_embed, critical_embed};
use sentinel_shared::heartbeat::{ApiClientKey, register_guilds};

use crate::analysis_queue::{AnalysisQueue, QueuedImage};
use crate::api_client::{Action, AnalyzeImageRequest, ApiClient, Classification};
use crate::channel_thresholds;
use crate::commands;
use crate::image_hash::{self, ImageHashCache};

// ── Constantes ──

/// Taille max du cache de deduplication avant nettoyage.
const DEDUP_CACHE_LIMIT: usize = 1000;
/// Nombre d'entrees a supprimer lors d'un nettoyage du cache dedup.
const DEDUP_CLEANUP_SIZE: usize = 500;
/// Taille max par defaut d'une image (10 Mo).
pub const DEFAULT_MAX_IMAGE_SIZE: u64 = 10 * 1024 * 1024;
/// Duree de mute par defaut en secondes (10 minutes).
const DEFAULT_MUTE_DURATION_SECS: u64 = 600;
/// Taille minimum d'un GIF pour etre considere comme anime.
const ANIMATED_GIF_MIN_SIZE: usize = 100_000;

// ── TypeMap keys ──

pub struct ProcessedMessagesKey;
impl TypeMapKey for ProcessedMessagesKey {
    type Value = Arc<DashSet<MessageId>>;
}

pub struct MaxImageSizeKey;
impl TypeMapKey for MaxImageSizeKey {
    type Value = u64;
}

pub struct HashCacheKey;
impl TypeMapKey for HashCacheKey {
    type Value = ImageHashCache;
}

pub struct QueueSenderKey;
impl TypeMapKey for QueueSenderKey {
    type Value = AnalysisQueue;
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
                if processed.len() > DEDUP_CACHE_LIMIT {
                    let to_remove: Vec<_> = processed.iter().take(DEDUP_CLEANUP_SIZE).map(|e| *e).collect();
                    for id in to_remove {
                        processed.remove(&id);
                    }
                }
            }
        }

        let guild_id = msg.guild_id.map(|id| id.to_string()).unwrap_or_default();

        {
            let data = ctx.data.read().await;
            if let Some(api) = data.get::<ApiClientKey>() {
                let config = api.get_guild_config(&guild_id).await.unwrap_or_default();
                if !BaseApiClient::config_bool(&config, "enabled", true) {
                    return;
                }
            }
        }

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

        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::all(),
        )
        .await
        {
            error!(error = %e, "Erreur enregistrement commandes");
        } else {
            info!("Slash commands enregistrees : image");
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            if let Some(guild_id) = command.guild_id {
                let data = ctx.data.read().await;
                if let Some(api) = data.get::<ApiClientKey>() {
                    if !sentinel_shared::discord_helpers::is_bot_enabled(api, &guild_id.to_string()).await {
                        return;
                    }
                }
            }

            if command.data.name.as_str() == "image" {
                commands::image::handle(&ctx, &command).await;
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
    let (base, max_image_size) = {
        let data = ctx.data.read().await;
        let base = match data.get::<ApiClientKey>() {
            Some(client) => Arc::clone(client),
            None => {
                error!("BaseApiClient introuvable dans le contexte");
                return;
            }
        };
        let max_size = data.get::<MaxImageSizeKey>().copied().unwrap_or(DEFAULT_MAX_IMAGE_SIZE);
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

    // Hash cache : verifier si l'image a deja ete analysee
    let image_hash = image_hash::compute_hash(&image_bytes);
    {
        let data = ctx.data.read().await;
        if let Some(cache) = data.get::<HashCacheKey>() {
            if let Some(cached_action) = cache.get_cached(image_hash) {
                info!(hash = image_hash, action = %cached_action, "Image cache hit — skip API");
                return; // Action deja connue (none = safe, pas besoin de re-analyser)
            }
        }
    }

    // Encoder en base64
    use base64::Engine;
    let image_b64 = base64::engine::general_purpose::STANDARD.encode(&image_bytes);

    // Determiner le content type
    let content_type = detect_content_type(filename, &image_bytes);

    // Seuil par salon
    let guild_config = {
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            api.get_guild_config(guild_id).await.unwrap_or_default()
        } else {
            std::collections::HashMap::new()
        }
    };

    let thresholds_raw = BaseApiClient::config_or(&guild_config, "channel_thresholds", "");
    let thresholds = channel_thresholds::parse_thresholds(&thresholds_raw);
    let default_threshold = BaseApiClient::config_or(&guild_config, "confidence_threshold", "0.5")
        .parse::<f64>().unwrap_or(0.5);
    let confidence_override = channel_thresholds::get_channel_threshold(
        &thresholds, msg.channel_id.get(), default_threshold,
    );

    // Detection screenshot et GIF anime
    let filename_lower = filename.to_lowercase();
    let is_screenshot = filename_lower.contains("screenshot") || filename_lower.contains("capture");
    let is_animated = content_type == "image/gif" && image_bytes.len() > ANIMATED_GIF_MIN_SIZE;

    let request = AnalyzeImageRequest {
        guild_id: guild_id.to_string(),
        channel_id: msg.channel_id.to_string(),
        user_id: msg.author.id.to_string(),
        username: msg.author.name.clone(),
        message_id: msg.id.to_string(),
        image_data: image_b64,
        content_type,
        filename: filename.to_string(),
        confidence_override: Some(confidence_override),
        is_screenshot,
        is_animated,
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

            // Stocker dans le hash cache
            {
                let data = ctx.data.read().await;
                if let Some(cache) = data.get::<HashCacheKey>() {
                    cache.store(image_hash, response.action.as_str());
                }
            }

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
            // Tenter la queue si activee
            let queue_enabled = BaseApiClient::config_bool(&guild_config, "queue_enabled", false);
            if queue_enabled {
                let data = ctx.data.read().await;
                if let Some(queue) = data.get::<QueueSenderKey>() {
                    let queued = QueuedImage {
                        request,
                        channel_id: msg.channel_id.get(),
                        message_id: msg.id.get(),
                        guild_id: guild_id.to_string(),
                        author_name: msg.author.name.clone(),
                        author_face: msg.author.face(),
                    };
                    if queue.enqueue(queued).await {
                        info!(message_id = %msg.id, "Image mise en queue pour retry");
                        return;
                    }
                }
            }

            // Fallback : laisser passer l'image (ne pas supprimer si le backend est injoignable)
            warn!(error = %e, "Backend injoignable — image laissee en place");
            base.send_log("warn", guild_id, &format!(
                "Backend injoignable pour analyse image (image non supprimee) : {}", e
            ));
        }
    }
}

/// Detecte le content type depuis l'extension ou les magic bytes.
pub(crate) fn detect_content_type(filename: &str, bytes: &[u8]) -> String {
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
        if bytes.starts_with(b"RIFF") && bytes.len() >= 12 {
            if bytes.get(8..12) == Some(b"WEBP") {
                return "image/webp".to_string();
            }
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
pub(crate) fn format_classifications(classifications: &[Classification]) -> String {
    if classifications.is_empty() {
        return "Aucune".to_string();
    }
    classifications
        .iter()
        .map(|c| format!("{} ({:.0}%)", c.label, c.confidence * 100.0))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Construit un embed d'action sur image avec les champs communs.
fn build_action_embed(
    base_embed: serenity::builder::CreateEmbed,
    msg: &Message,
    reason_text: &str,
    detection_text: &str,
) -> serenity::builder::CreateEmbed {
    base_embed
        .description(format!("<@{}>", msg.author.id))
        .field("\u{1f4dd} Raison", reason_text, false)
        .field("\u{1f3f7}\u{fe0f} Detection", detection_text, false)
        .thumbnail(msg.author.face())
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
            let embed = build_action_embed(
                warn_embed("\u{26a0}\u{fe0f} Avertissement — Image"), msg, reason_text, &detection_text,
            );
            if let Err(e) = msg.channel_id
                .send_message(&ctx.http, CreateMessage::new().embed(embed))
                .await
            {
                warn!(error = %e, "Erreur envoi embed avertissement image");
            }
            info!(user = %msg.author.name, "Avertissement image envoye");
        }
        Action::Delete => {
            let embed = build_action_embed(
                moderate_embed("\u{1f5d1}\u{fe0f} Image supprimee"), msg, reason_text, &detection_text,
            );
            if let Err(e) = msg.channel_id
                .send_message(&ctx.http, CreateMessage::new().embed(embed))
                .await
            {
                warn!(error = %e, "Erreur envoi embed suppression image");
            }
            msg.delete(&ctx.http).await?;
            info!(message_id = %msg.id, "Message avec image supprime");
        }
        Action::Mute => {
            let mute_duration_secs: u64 = duration.unwrap_or(DEFAULT_MUTE_DURATION_SECS);
            let mute_minutes = mute_duration_secs / 60;
            let embed = danger_embed("\u{1f507} Mute — Image interdite")
                .description(format!("<@{}>", msg.author.id))
                .field("\u{1f4dd} Raison", reason_text, false)
                .field("\u{23f1}\u{fe0f} Duree", format!("{mute_minutes} minutes"), false)
                .thumbnail(msg.author.face());
            if let Err(e) = msg.channel_id
                .send_message(&ctx.http, CreateMessage::new().embed(embed))
                .await
            {
                warn!(error = %e, "Erreur envoi embed mute image");
            }
            msg.delete(&ctx.http).await?;

            if let (Some(guild_id), Ok(_member)) = (msg.guild_id, msg.member(&ctx.http).await) {
                let mut member = guild_id.member(&ctx.http, msg.author.id).await?;
                let now_secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                let target_secs = now_secs + mute_duration_secs as i64;
                match time::OffsetDateTime::from_unix_timestamp(target_secs) {
                    Ok(datetime) => {
                        let timeout = serenity::model::Timestamp::from(datetime);
                        member
                            .disable_communication_until_datetime(&ctx.http, timeout)
                            .await?;
                        info!(user = %msg.author.name, duration_secs = mute_duration_secs, "Utilisateur mute pour image");
                    }
                    Err(e) => {
                        error!(error = %e, secs = target_secs, "Timestamp invalide pour mute");
                    }
                }
            }
        }
        Action::Ban => {
            if let Some(guild_id) = msg.guild_id {
                let embed = build_action_embed(
                    critical_embed("\u{1f528} Ban — Image interdite"), msg, reason_text, &detection_text,
                );
                if let Err(e) = msg.channel_id
                    .send_message(&ctx.http, CreateMessage::new().embed(embed))
                    .await
                {
                    warn!(error = %e, "Erreur envoi embed ban image");
                }
                guild_id
                    .ban_with_reason(&ctx.http, msg.author.id, 1, reason_text)
                    .await?;
                info!(user = %msg.author.name, "Utilisateur banni pour image");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_client::Classification;

    // ── detect_content_type ──

    #[test]
    fn detect_jpeg_magic_bytes() {
        let bytes = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00];
        assert_eq!(detect_content_type("unknown.bin", bytes), "image/jpeg");
    }

    #[test]
    fn detect_png_magic_bytes() {
        let bytes = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A];
        assert_eq!(detect_content_type("unknown.bin", bytes), "image/png");
    }

    #[test]
    fn detect_gif_magic_bytes() {
        let bytes = b"GIF89a\x00\x01";
        assert_eq!(detect_content_type("unknown.bin", bytes), "image/gif");
    }

    #[test]
    fn detect_webp_magic_bytes() {
        let mut bytes = vec![0u8; 16];
        bytes[0..4].copy_from_slice(b"RIFF");
        bytes[8..12].copy_from_slice(b"WEBP");
        assert_eq!(detect_content_type("unknown.bin", &bytes), "image/webp");
    }

    #[test]
    fn detect_webp_too_short_falls_back() {
        // RIFF header mais moins de 12 bytes
        let bytes = b"RIFF1234";
        // Trop court pour verifier WEBP, fallback sur extension
        assert_eq!(detect_content_type("image.webp", bytes), "image/webp");
    }

    #[test]
    fn detect_by_extension_png() {
        let bytes = &[0x00, 0x00]; // Pas de magic bytes valides
        assert_eq!(detect_content_type("photo.PNG", bytes), "image/png");
    }

    #[test]
    fn detect_by_extension_gif() {
        assert_eq!(detect_content_type("anim.gif", &[0x00]), "image/gif");
    }

    #[test]
    fn detect_by_extension_webp() {
        assert_eq!(detect_content_type("img.webp", &[0x00]), "image/webp");
    }

    #[test]
    fn detect_by_extension_bmp() {
        assert_eq!(detect_content_type("img.bmp", &[0x00]), "image/bmp");
    }

    #[test]
    fn detect_fallback_default_jpeg() {
        // Aucune extension reconnue, aucun magic byte → default jpeg
        assert_eq!(detect_content_type("file.dat", &[0x00]), "image/jpeg");
    }

    #[test]
    fn detect_empty_bytes() {
        assert_eq!(detect_content_type("photo.jpg", &[]), "image/jpeg");
    }

    #[test]
    fn magic_bytes_take_priority_over_extension() {
        // Magic bytes PNG mais extension .jpg → doit retourner PNG
        let bytes = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A];
        assert_eq!(detect_content_type("photo.jpg", bytes), "image/png");
    }

    // ── format_classifications ──

    #[test]
    fn format_empty_classifications() {
        assert_eq!(format_classifications(&[]), "Aucune");
    }

    #[test]
    fn format_single_classification() {
        let c = vec![Classification { label: "nsfw".to_string(), confidence: 0.92 }];
        assert_eq!(format_classifications(&c), "nsfw (92%)");
    }

    #[test]
    fn format_multiple_classifications() {
        let c = vec![
            Classification { label: "nsfw".to_string(), confidence: 0.85 },
            Classification { label: "illicit".to_string(), confidence: 0.12 },
        ];
        assert_eq!(format_classifications(&c), "nsfw (85%), illicit (12%)");
    }

    #[test]
    fn format_zero_confidence() {
        let c = vec![Classification { label: "safe".to_string(), confidence: 0.0 }];
        assert_eq!(format_classifications(&c), "safe (0%)");
    }

    #[test]
    fn format_full_confidence() {
        let c = vec![Classification { label: "nsfw".to_string(), confidence: 1.0 }];
        assert_eq!(format_classifications(&c), "nsfw (100%)");
    }

    // ── Action ──

    #[test]
    fn action_as_str() {
        assert_eq!(Action::None.as_str(), "none");
        assert_eq!(Action::Warn.as_str(), "warn");
        assert_eq!(Action::Delete.as_str(), "delete");
        assert_eq!(Action::Mute.as_str(), "mute");
        assert_eq!(Action::Ban.as_str(), "ban");
    }

    #[test]
    fn action_equality() {
        assert_eq!(Action::None, Action::None);
        assert_ne!(Action::Warn, Action::Delete);
    }

    // ── Constantes ──

    #[test]
    fn default_max_image_size_is_10mb() {
        assert_eq!(DEFAULT_MAX_IMAGE_SIZE, 10 * 1024 * 1024);
    }

    #[test]
    fn default_mute_is_10_minutes() {
        assert_eq!(DEFAULT_MUTE_DURATION_SECS, 600);
    }

    #[test]
    fn dedup_cleanup_smaller_than_limit() {
        assert!(DEDUP_CLEANUP_SIZE < DEDUP_CACHE_LIMIT);
    }

    #[test]
    fn supported_extensions_complete() {
        assert!(SUPPORTED_EXTENSIONS.contains(&"jpg"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"jpeg"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"png"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"gif"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"webp"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"bmp"));
    }

    #[test]
    fn supported_content_types_match_extensions() {
        // Chaque extension devrait avoir un content type correspondant
        assert!(SUPPORTED_CONTENT_TYPES.contains(&"image/jpeg"));
        assert!(SUPPORTED_CONTENT_TYPES.contains(&"image/png"));
        assert!(SUPPORTED_CONTENT_TYPES.contains(&"image/gif"));
        assert!(SUPPORTED_CONTENT_TYPES.contains(&"image/webp"));
        assert!(SUPPORTED_CONTENT_TYPES.contains(&"image/bmp"));
    }
}
