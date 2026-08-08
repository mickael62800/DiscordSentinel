//! Consumer des events moderation publies sur Redis (log + rappels + escalades SLA).

use serenity::all::{Context, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage};
use serenity::model::id::UserId;
use tracing::{error, info, warn};

pub(super) async fn handle_redis_moderation_event(ctx: &Context, payload: &str) {
    let event: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return,
    };

    let event_type = event.get("event").and_then(|e| e.as_str()).unwrap_or("");
    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };

    match event_type {
        "moderation_action" => handle_moderation_action_log(ctx, data).await,
        "sanction_expiry_reminder" => handle_sanction_expiry_reminder(ctx, data).await,
        "sanction_expired_unban" => handle_sanction_expired_unban(ctx, data).await,
        "appeal_sla_escalated" => handle_appeal_sla_escalated(ctx, data).await,
        "sursis_ban" => handle_sursis_ban(ctx, data).await,
        _ => {}
    }
}

/// Ban definitif d'un sursis arrive a echeance (emis par le worker via l'API).
async fn handle_sursis_ban(ctx: &Context, data: &serde_json::Value) {
    let guild_id = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let user_id = data.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
    let reason = data
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("Ban en sursis — délai d'appel écoulé");
    let (Ok(gid), Ok(uid)) = (guild_id.parse::<u64>(), user_id.parse::<u64>()) else {
        return;
    };
    let guild = serenity::model::id::GuildId::new(gid);
    if let Err(e) = guild
        .ban_with_reason(&ctx.http, UserId::new(uid), 0, reason)
        .await
    {
        warn!(error = %e, guild_id, user_id, "sursis_ban: echec du ban Discord");
    } else {
        info!(guild_id, user_id, "Sursis expire -> ban definitif applique");
    }
    // Nettoie le salon d'appel si connu.
    if let Some(cid) = data
        .get("channel_id")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok())
    {
        let _ = serenity::model::id::ChannelId::new(cid)
            .delete(&ctx.http)
            .await;
    }
}

async fn handle_moderation_action_log(ctx: &Context, data: &serde_json::Value) {
    let action_type = data
        .get("action_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let target_id = data.get("target_id").and_then(|v| v.as_str()).unwrap_or("");
    let target_name = data
        .get("target_name")
        .and_then(|v| v.as_str())
        .unwrap_or(target_id);
    let moderator = data
        .get("moderator_name")
        .and_then(|v| v.as_str())
        .unwrap_or("Inconnu");
    let guild_id = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let reason_raw = data
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("Aucune raison specifiee");
    let reason: String = reason_raw
        .chars()
        .take(500)
        .collect::<String>()
        .replace("```", "` ` `")
        .replace("||", "| |")
        .replace('@', "@\u{200b}");

    if guild_id.is_empty() {
        return;
    }

    let Some(log_channel) = crate::shared::discord_helpers::get_log_channel(
        ctx,
        guild_id,
        crate::modules::moderation::MODULE_BOT_NAME,
    )
    .await
    else {
        return;
    };

    let (icon, action_label, color) = match action_type {
        "ban_permanent" => ("\u{1f6ab}", "Bannissement permanent", 0xdc2626u32),
        "ban_temp" => ("\u{1f6ab}", "Bannissement temporaire", 0xdc2626),
        "ban" => ("\u{1f6ab}", "Bannissement", 0xdc2626),
        "unban" => ("\u{1f513}", "Debannissement", 0x2ecc71),
        "mute" | "mute_temp" => ("\u{1f507}", "Mute", 0xef4444),
        "unmute" => ("\u{1f50a}", "Unmute", 0x2ecc71),
        "warn" => ("\u{26a0}\u{fe0f}", "Avertissement", 0xf59e0b),
        "kick" => ("\u{1f462}", "Expulsion", 0xf97316),
        "delete" => ("\u{1f5d1}\u{fe0f}", "Message supprime", 0xf97316),
        "call" => ("\u{1f4de}", "Convocation", 0x3498db),
        _ => ("\u{1f4cb}", "Action de moderation", 0x95a5a6),
    };

    let source = if moderator == "Desktop App" {
        "Desktop App"
    } else {
        moderator
    };

    let (avatar_url, real_name) = if let Ok(uid) = target_id.parse::<u64>() {
        let user_id = UserId::new(uid);
        match user_id.to_user(&ctx.http).await {
            Ok(user) => (Some(user.face()), user.name.clone()),
            Err(_) => (None, target_name.to_string()),
        }
    } else {
        (None, target_name.to_string())
    };

    let mut embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::new(format!("{} {}", icon, action_label)))
        .title("Moderation - Action manuelle")
        .color(color)
        .field(
            "\u{1f464} Utilisateur",
            format!("<@{}> (`{}`)", target_id, real_name),
            true,
        )
        .field("\u{1f6e1}\u{fe0f} Moderateur", source, true)
        .field("\u{2699}\u{fe0f} Action", action_label, true)
        .field("\u{1f4dd} Raison", format!("```{}```", reason), false)
        .footer(CreateEmbedFooter::new(format!(
            "Cible: {} | Moderateur: {} \u{2022} {}",
            target_id, source, action_type
        )))
        .timestamp(serenity::model::Timestamp::now());

    if let Some(url) = avatar_url {
        embed = embed.thumbnail(url);
    }

    let msg = CreateMessage::new().embed(embed);
    if let Err(e) = log_channel.send_message(&ctx.http, msg).await {
        error!(error = %e, "Erreur envoi log moderation dans Discord");
    }
}

async fn handle_sanction_expiry_reminder(ctx: &Context, data: &serde_json::Value) {
    let moderator_id = data
        .get("moderator_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let target_name = data
        .get("target_name")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let action_type = data
        .get("action_type")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let reason = data
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("Aucune raison");
    let minutes_left = data
        .get("minutes_left")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let target_id = data.get("target_id").and_then(|v| v.as_str()).unwrap_or("");

    let mod_uid = match moderator_id.parse::<u64>() {
        Ok(u) => u,
        Err(_) => {
            warn!(
                moderator_id,
                "sanction_expiry_reminder: moderator_id invalide"
            );
            return;
        }
    };

    let user_id = UserId::new(mod_uid);
    let user = match user_id.to_user(&ctx.http).await {
        Ok(u) => u,
        Err(e) => {
            warn!(error = %e, moderator_id, "sanction_expiry_reminder: user fetch failed");
            return;
        }
    };

    let action_label = match action_type {
        "mute_temp" => "Mute temporaire",
        "ban_temp" => "Bannissement temporaire",
        _ => "Sanction temporaire",
    };

    let reason_trunc: String = reason
        .chars()
        .take(300)
        .collect::<String>()
        .replace("```", "` ` `");

    let embed = CreateEmbed::new()
        .title("\u{23f0} Rappel — Sanction proche de l'expiration")
        .color(0xf59e0b)
        .field("\u{2699}\u{fe0f} Action", action_label, true)
        .field(
            "\u{1f464} Cible",
            format!("<@{}> (`{}`)", target_id, target_name),
            true,
        )
        .field(
            "\u{23f3} Temps restant",
            format!("{} minutes", minutes_left),
            true,
        )
        .field("\u{1f4dd} Raison", format!("```{}```", reason_trunc), false)
        .footer(CreateEmbedFooter::new(
            "Rappel automatique envoye par le moderation-bot",
        ))
        .timestamp(serenity::model::Timestamp::now());

    let dm = CreateMessage::new().embed(embed);
    let channel = match user.create_dm_channel(&ctx.http).await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, moderator_id, "sanction_expiry_reminder: DM channel failed");
            return;
        }
    };
    if let Err(e) = channel.send_message(&ctx.http, dm).await {
        warn!(error = %e, moderator_id, "sanction_expiry_reminder: DM send failed");
    } else {
        info!(
            moderator_id,
            target_name, action_type, minutes_left, "DM rappel expiration envoye"
        );
    }
}

/// BUG #1/#2 — Auto-unban d'un ban temporaire arrive a expiration.
///
/// Emis par le worker `expire_temp_bans` (fire-once via `unban_status='done'`).
/// On appelle `guild_id.unban(...)` en best-effort : le ban peut avoir deja ete
/// leve manuellement (on log alors l'echec sans bruit). Un embed est poste dans
/// le salon de logs moderation pour tracer la levee automatique.
async fn handle_sanction_expired_unban(ctx: &Context, data: &serde_json::Value) {
    let guild_id_str = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let target_id = data.get("target_id").and_then(|v| v.as_str()).unwrap_or("");
    let target_name = data
        .get("target_name")
        .and_then(|v| v.as_str())
        .unwrap_or(target_id);

    let (Ok(gid), Ok(uid)) = (guild_id_str.parse::<u64>(), target_id.parse::<u64>()) else {
        warn!(
            guild_id = guild_id_str,
            target_id, "sanction_expired_unban: id invalide"
        );
        return;
    };

    let guild_id = serenity::model::id::GuildId::new(gid);
    let user_id = UserId::new(uid);

    match guild_id.unban(&ctx.http, user_id).await {
        Ok(()) => {
            info!(
                guild_id = guild_id_str,
                target_id, "Ban temporaire expire -> unban Discord applique"
            );
        }
        Err(e) => {
            // Best-effort : ban deja leve manuellement, user jamais banni, etc.
            warn!(
                error = %e,
                guild_id = guild_id_str,
                target_id,
                "sanction_expired_unban: unban Discord echoue (ban deja leve ?)"
            );
        }
    }

    let Some(log_channel) = crate::shared::discord_helpers::get_log_channel(
        ctx,
        guild_id_str,
        crate::modules::moderation::MODULE_BOT_NAME,
    )
    .await
    else {
        return;
    };

    let embed = CreateEmbed::new()
        .title("\u{1f513} Ban temporaire expire — debannissement automatique")
        .color(0x2ecc71)
        .field(
            "\u{1f464} Utilisateur",
            format!("<@{}> (`{}`)", target_id, target_name),
            true,
        )
        .footer(CreateEmbedFooter::new(
            "Levee automatique emise par le worker d'expiration",
        ))
        .timestamp(serenity::model::Timestamp::now());

    if let Err(e) = log_channel
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await
    {
        warn!(error = %e, "sanction_expired_unban: log send failed");
    }
}

async fn handle_appeal_sla_escalated(ctx: &Context, data: &serde_json::Value) {
    let guild_id = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let ticket_id = data
        .get("ticket_id")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let author_id = data.get("author_id").and_then(|v| v.as_str()).unwrap_or("");
    let author_name = data
        .get("author_name")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let title = data
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Appel de sanction");
    let age_minutes = data
        .get("age_minutes")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let sla_escalation_minutes = data
        .get("sla_escalation_minutes")
        .and_then(|v| v.as_i64())
        .unwrap_or(60);

    if guild_id.is_empty() {
        return;
    }

    let Some(log_channel) = crate::shared::discord_helpers::get_log_channel(
        ctx,
        guild_id,
        crate::modules::moderation::MODULE_BOT_NAME,
    )
    .await
    else {
        return;
    };

    let short_id = ticket_id.chars().take(8).collect::<String>();
    let title_trunc: String = title
        .chars()
        .take(200)
        .collect::<String>()
        .replace("```", "` ` `");

    let embed = CreateEmbed::new()
        .title("\u{1f6a8} Escalade SLA — Appel de sanction en attente")
        .description(format!(
            "L'appel de sanction `{}` n'a pas recu de premiere reponse depuis **{} minutes** (SLA: {} min). \
             Un moderateur senior doit l'examiner.",
            short_id, age_minutes, sla_escalation_minutes
        ))
        .color(0xdc2626)
        .field(
            "\u{1f464} Auteur",
            format!("<@{}> (`{}`)", author_id, author_name),
            true,
        )
        .field("\u{1f3ab} Ticket", format!("`{}`", short_id), true)
        .field("\u{1f4ac} Titre", format!("```{}```", title_trunc), false)
        .footer(CreateEmbedFooter::new(
            "Escalade automatique emise par appeal-sla-worker",
        ))
        .timestamp(serenity::model::Timestamp::now());

    let msg = CreateMessage::new().content("@here").embed(embed);
    if let Err(e) = log_channel.send_message(&ctx.http, msg).await {
        warn!(error = %e, ticket_id, "appeal_sla_escalated: log send failed");
    } else {
        info!(
            ticket_id,
            guild_id, age_minutes, "Escalade appel SLA postee dans logs"
        );
    }
}
