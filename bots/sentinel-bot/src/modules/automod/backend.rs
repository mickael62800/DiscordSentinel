//! Integration backend : envoi au service d'analyse, execution des actions,
//! pipeline d'analyse d'images via ai-worker.

use std::sync::Arc;

use serenity::model::channel::Message;
use serenity::prelude::*;
use tracing::{debug, error, info, warn};

use sentinel_shared::embeds::{warn_embed, moderate_embed, danger_embed, critical_embed};
use sentinel_shared::heartbeat::ApiClientKey;

use super::api_client::{Action, AnalyzeRequest, ApiClient, MessageMetadata};
use super::config::EmbedColors;
use super::detectors;
use super::review::{send_review_card, sanitize_embed_content};

/// Genere une raison descriptive a partir des flags detecteurs
/// quand le backend n'en retourne pas.
fn build_fallback_reason(flags: &detectors::DetectionFlags) -> String {
    let mut parts = Vec::new();
    if flags.phishing { parts.push("lien de phishing"); }
    if flags.insult { parts.push("langage inapproprie"); }
    if flags.spam { parts.push("spam"); }
    if flags.link { parts.push("lien non autorise"); }
    if parts.is_empty() {
        "Contenu inapproprie detecte".to_string()
    } else {
        format!("Detection : {}", parts.join(", "))
    }
}

/// Envoie le message au backend pour analyse et execute l'action.
pub(super) async fn send_to_backend(
    ctx: &Context,
    msg: &Message,
    flags: detectors::DetectionFlags,
    mute_duration_secs: u64,
    log_channel_id: u64,
    ai_review_mode: bool,
    colors: &EmbedColors,
    context_max_messages: u8,
    context_max_chars: usize,
) {
    // Recuperer les N derniers messages du canal pour le contexte conversationnel
    let context_messages = if context_max_messages == 0 {
        Vec::new()
    } else {
        match msg
            .channel_id
            .messages(
                &ctx.http,
                serenity::builder::GetMessages::new()
                    .before(msg.id)
                    .limit(context_max_messages),
            )
            .await
        {
            Ok(messages) => messages
                .into_iter()
                .rev() // ordre chronologique
                .filter(|m| !m.author.bot)
                .map(|m| super::api_client::ContextMessage {
                    username: m.author.name.clone(),
                    content: m.content.chars().take(context_max_chars).collect(),
                })
                .collect(),
            Err(e) => {
                tracing::warn!(error = %e, "Echec recuperation contexte canal");
                Vec::new()
            }
        }
    };

    let request = AnalyzeRequest {
        guild_id: msg.guild_id.map(|id| id.to_string()).unwrap_or_default(),
        channel_id: msg.channel_id.to_string(),
        user_id: msg.author.id.to_string(),
        username: msg.author.name.clone(),
        content: msg.content.clone(),
        flags,
        metadata: MessageMetadata {
            message_id: msg.id.to_string(),
            timestamp: msg.timestamp.to_string(),
        },
        context_messages,
    };

    let data = ctx.data.read().await;
    let base = match data.get::<ApiClientKey>() {
        Some(client) => Arc::clone(client),
        None => {
            error!("BaseApiClient introuvable dans le contexte");
            return;
        }
    };
    let grpc = match data.get::<sentinel_shared::grpc_client::GrpcClientKey>() {
        Some(g) => Arc::clone(g),
        None => {
            error!("SentinelGrpcClient introuvable dans le contexte");
            return;
        }
    };
    drop(data);

    let api_client = ApiClient::new(Arc::clone(&base), grpc);

    match api_client.analyze(&request).await {
        Ok(response) => {
            info!(action = ?response.action, reason = ?response.reason, "Reponse du backend");

            let fallback_reason = build_fallback_reason(&request.flags);
            let effective_reason = response
                .reason
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(fallback_reason);

            if response.action != Action::None {
                let guild_id = msg.guild_id.map(|id| id.to_string()).unwrap_or_default();
                let action_label = match &response.action {
                    Action::Warn => "Avertissement",
                    Action::Delete => "Suppression",
                    Action::Mute => "Mute",
                    Action::Ban => "Proposition de ban",
                    Action::None => "",
                };
                let log_message = format!(
                    "{} -- {} : {}",
                    action_label,
                    msg.author.name,
                    effective_reason,
                );

                base.send_log(
                    if matches!(response.action, Action::Ban) { "error" } else { "warn" },
                    &guild_id,
                    &log_message,
                );
            }

            if ai_review_mode && log_channel_id != 0 {
                send_review_card(
                    ctx, msg, &response.action,
                    &effective_reason,
                    response.score.unwrap_or(0.0),
                    &request.flags,
                    log_channel_id, colors,
                ).await;
            } else {
                // Mode auto ou pas de salon review -> action directe.
                if let Err(e) = execute_action(ctx, msg, &response.action, Some(effective_reason.as_str()), mute_duration_secs, colors).await {
                    error!(error = %e, "Erreur lors de l'execution de l'action");
                }
            }
        }
        Err(e) => {
            error!(error = %e, "Backend injoignable -- action locale par defaut");
            // En mode fallback, supprimer les messages flagges (phishing, insulte, spam, lien)
            let reason = if request.flags.phishing {
                Some("Lien suspect detecte.")
            } else if request.flags.insult {
                Some("Langage inapproprie.")
            } else if request.flags.spam {
                Some("Spam detecte.")
            } else if request.flags.link {
                Some("Lien non autorise.")
            } else {
                None
            };

            if let Some(reason_text) = reason {
                let embed = moderate_embed("Message supprime (mode hors-ligne)")
                    .color(colors.delete)
                    .field("Raison", reason_text, false)
                    .thumbnail(msg.author.face());
                let builder = serenity::builder::CreateMessage::new().embed(embed);
                if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                    warn!(error = %e, "Echec envoi notification mode hors-ligne");
                }
                if let Err(e) = msg.delete(&ctx.http).await {
                    warn!(error = %e, message_id = %msg.id, "Echec suppression message mode hors-ligne");
                }
            }
        }
    }
}

/// Execute l'action decidee par le backend.
pub(super) async fn execute_action(
    ctx: &Context,
    msg: &Message,
    action: &Action,
    reason: Option<&str>,
    mute_duration_secs: u64,
    colors: &EmbedColors,
) -> Result<(), serenity::Error> {
    let reason_text = reason.unwrap_or("Automod");

    match action {
        Action::None => {}
        Action::Warn => {
            let embed = warn_embed("Avertissement AutoMod")
                .color(colors.warn)
                .field("Raison", reason_text, false)
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            msg.channel_id.send_message(&ctx.http, builder).await?;
            info!(user = %msg.author.name, "Avertissement envoye");
        }
        Action::Delete => {
            let content_preview = sanitize_embed_content(&msg.content, 200);
            let embed = moderate_embed("Message supprime")
                .color(colors.delete)
                .field("Raison", reason_text, false)
                .field("Contenu original", format!("```{}```", content_preview), false)
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                warn!(error = %e, "Echec envoi notification suppression");
            }
            msg.delete(&ctx.http).await?;
            info!(message_id = %msg.id, "Message supprime");
        }
        Action::Mute => {
            let mute_minutes = mute_duration_secs / 60;
            let embed = danger_embed("Mute automatique")
                .color(colors.mute)
                .field("Raison", reason_text, false)
                .field("Duree", format!("{} minutes", mute_minutes), false)
                .thumbnail(msg.author.face());
            let builder = serenity::builder::CreateMessage::new().embed(embed);
            if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                warn!(error = %e, "Echec envoi notification mute");
            }
            if let (Some(guild_id_val), Ok(member)) = (msg.guild_id, msg.member(&ctx.http).await) {
                let mut member = guild_id_val.member(&ctx.http, member.user.id).await?;
                const MAX_MUTE_SECS: u64 = 28 * 24 * 3600;
                let safe_duration = mute_duration_secs.min(MAX_MUTE_SECS);
                let secs = match std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                {
                    Ok(d) => match (d.as_secs() as i64).checked_add(safe_duration as i64) {
                        Some(v) => v,
                        None => {
                            error!("Overflow timestamp mute (cas improbable)");
                            return Ok(());
                        }
                    },
                    Err(e) => {
                        error!(error = %e, "Erreur horloge systeme pour le calcul du mute");
                        return Ok(());
                    }
                };
                let datetime = match time::OffsetDateTime::from_unix_timestamp(secs) {
                    Ok(dt) => dt,
                    Err(e) => {
                        error!(error = %e, "Timestamp invalide pour le mute");
                        return Ok(());
                    }
                };
                let timeout = serenity::model::Timestamp::from(datetime);
                member
                    .disable_communication_until_datetime(&ctx.http, timeout)
                    .await?;
                info!(user = %msg.author.name, duration_secs = mute_duration_secs, "Utilisateur mute");
            }
            if let Err(e) = msg.delete(&ctx.http).await {
                warn!(error = %e, message_id = %msg.id, "Echec suppression message apres mute automod");
            }
        }
        Action::Ban => {
            if let Some(_guild_id) = msg.guild_id {
                let embed = critical_embed("Signalement pour bannissement")
                    .color(colors.ban)
                    .field("Raison", reason_text, false)
                    .thumbnail(msg.author.face());
                let builder = serenity::builder::CreateMessage::new().embed(embed);
                if let Err(e) = msg.channel_id.send_message(&ctx.http, builder).await {
                    warn!(error = %e, "Echec envoi notification ban");
                }
                msg.delete(&ctx.http).await?;
                info!(user = %msg.author.name, "Proposition de ban enregistree (ban non execute)");
            }
        }
    }

    Ok(())
}

/// Analyse les images attachees a un message via le ai-worker (async).
///
/// Lit la config automod-bot pour les cles vision_* (fusionnees depuis l ex
/// image-bot par la migration 156) :
///   - vision_max_image_size_mb : taille max d une image traitee (defaut 14 Mo)
///   - vision_scan_embeds       : analyse aussi les images dans les embeds
///   - vision_queue_max_retries : nombre de retries sur echec de submission
///   - vision_auto_delete_nsfw  : force delete si la raison contient "nsfw"
///   - vision_auto_delete_illicit : force delete si la raison contient "illicit"
pub(super) async fn analyze_message_images(
    ctx: &Context,
    msg: &Message,
    mute_duration_secs: u64,
    _log_channel_id: u64,
    colors: &EmbedColors,
) {
    let guild_id = msg.guild_id.map(|g| g.to_string()).unwrap_or_default();

    // Lecture de la config automod-bot (ex-image-bot fusionne par la 156).
    let config = sentinel_shared::discord_helpers::guild_config_or_default(
        ctx, &guild_id, crate::modules::automod::MODULE_BOT_NAME,
    ).await;

    let max_image_size_mb = sentinel_shared::api_client::BaseApiClient::config_u64(
        &config, "vision_max_image_size_mb", 14,
    );
    let max_image_bytes = (max_image_size_mb as usize) * 1024 * 1024;
    let scan_embeds = sentinel_shared::api_client::BaseApiClient::config_bool(
        &config, "vision_scan_embeds", true,
    );
    let queue_max_retries = sentinel_shared::api_client::BaseApiClient::config_u64(
        &config, "vision_queue_max_retries", 3,
    ) as usize;
    let auto_delete_nsfw = sentinel_shared::api_client::BaseApiClient::config_bool(
        &config, "vision_auto_delete_nsfw", false,
    );
    let auto_delete_illicit = sentinel_shared::api_client::BaseApiClient::config_bool(
        &config, "vision_auto_delete_illicit", true,
    );

    // Collecte des URLs : pieces jointes + (optionnel) images dans embeds.
    let mut image_urls: Vec<String> = msg
        .attachments
        .iter()
        .filter(|a| {
            a.content_type
                .as_deref()
                .map(|ct| ct.starts_with("image/"))
                .unwrap_or(false)
        })
        .map(|a| a.url.clone())
        .collect();

    if scan_embeds {
        for embed in &msg.embeds {
            if let Some(img) = &embed.image {
                image_urls.push(img.url.clone());
            }
            if let Some(thumb) = &embed.thumbnail {
                image_urls.push(thumb.url.clone());
            }
        }
    }

    if image_urls.is_empty() {
        return;
    }

    let data = ctx.data.read().await;
    let Some(base) = data.get::<sentinel_shared::heartbeat::ApiClientKey>() else {
        return;
    };

    let http_client = reqwest::Client::new();
    let api_url = base.base_url().to_string();

    for url in &image_urls {
        // 1. Telecharger l'image depuis Discord.
        let bytes = match http_client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                Ok(b) => b.to_vec(),
                Err(e) => { warn!(error = %e, url, "Echec lecture bytes image"); continue; }
            },
            Ok(resp) => { warn!(status = %resp.status(), url, "Image download non-success"); continue; }
            Err(e) => { warn!(error = %e, url, "Echec download image"); continue; }
        };

        if bytes.len() > max_image_bytes {
            debug!(size_bytes = bytes.len(), max_bytes = max_image_bytes, url, "Image > vision_max_image_size_mb, skip");
            continue;
        }

        // 2. Soumettre un job AI via l'API (non-bloquant, queue DB).
        //    Retry jusqu a queue_max_retries fois en cas d echec reseau.
        let payload = serde_json::json!({
            "guild_id": guild_id,
            "channel_id": msg.channel_id.to_string(),
            "user_id": msg.author.id.to_string(),
            "username": msg.author.name,
            "message_id": msg.id.to_string(),
            "image_base64": base64_encode(&bytes),
        });

        let mut job_id: Option<String> = None;
        for attempt in 0..=queue_max_retries {
            let submit_resp = match http_client
                .post(format!("{api_url}/api/ai/jobs"))
                .json(&serde_json::json!({
                    "guild_id": guild_id,
                    "job_type": "analyze_image",
                    "input_payload": payload,
                }))
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!(error = %e, attempt, "Echec soumission job AI image");
                    continue;
                }
            };

            if submit_resp.status().is_success() {
                if let Ok(v) = submit_resp.json::<serde_json::Value>().await {
                    if let Some(id) = v.get("job_id").and_then(|x| x.as_str()) {
                        job_id = Some(id.to_string());
                        break;
                    }
                }
            } else {
                warn!(status = %submit_resp.status(), attempt, "Job AI image refuse par l'API");
            }
        }

        let Some(job_id) = job_id else {
            warn!("Job AI image abandonne apres {queue_max_retries} retries");
            continue;
        };

        // 3. Attendre le resultat via Redis (pub/sub avec timeout 30s).
        let redis_key = format!("ai_result:{job_id}");
        let result = wait_for_ai_result(base, &redis_key).await;

        let Some(result_json) = result else {
            debug!(job_id, "Pas de resultat AI dans le delai (image)");
            continue;
        };

        // 4. Extraire l'action retournee par l'API.
        let action_str = result_json.get("action").and_then(|v| v.as_str()).unwrap_or("none");
        let reason = result_json.get("reason").and_then(|v| v.as_str()).unwrap_or("Image detectee");

        let api_action = match action_str {
            "warn" => Action::Warn,
            "delete" => Action::Delete,
            "mute" => Action::Mute,
            "ban" => Action::Ban,
            _ => Action::None,
        };

        // Override : si la raison signale NSFW / illicit ET le toggle correspondant
        // est ON, force la suppression meme si l action API retournee etait moins
        // severe. Garde les actions plus severes (mute/ban) telles quelles.
        let reason_lower = reason.to_lowercase();
        let action = if (auto_delete_nsfw && reason_lower.contains("nsfw"))
            || (auto_delete_illicit && reason_lower.contains("illicit"))
        {
            match api_action {
                Action::None | Action::Warn => Action::Delete,
                other => other,
            }
        } else {
            api_action
        };

        if action == Action::None {
            continue;
        }

        info!(user = %msg.author.name, action = ?action, reason, "Image moderation (via ai-worker)");
        if let Err(e) = execute_action(ctx, msg, &action, Some(reason), mute_duration_secs, colors).await {
            warn!(error = %e, "Echec execution action image");
        }
        break;
    }
}

/// Encode bytes en base64.
fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Attend le resultat d'un job AI via Redis GET (poll 1s, timeout 30s).
async fn wait_for_ai_result(
    _base: &sentinel_shared::api_client::BaseApiClient,
    redis_key: &str,
) -> Option<serde_json::Value> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let client = redis::Client::open(redis_url.as_str()).ok()?;
    let mut conn = client.get_multiplexed_async_connection().await.ok()?;

    // Poll toutes les secondes pendant 30s max.
    for _ in 0..30 {
        let val: Option<String> = redis::AsyncCommands::get(&mut conn, redis_key).await.ok()?;
        if let Some(json_str) = val {
            return serde_json::from_str(&json_str).ok();
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    None
}
