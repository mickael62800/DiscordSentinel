use serenity::model::channel::Message;
use serenity::model::event::MessageUpdateEvent;
use serenity::model::id::{ChannelId, GuildId, MessageId};
use serenity::prelude::*;

use sentinel_shared::embeds::critical_embed;

use super::audit_event;
use super::handler_impl::{AnomalyDetectorKey, Handler, MessageCacheKey, WeeklyTrackerKey};
use super::weekly_report::StatField;

/// Helper : construit et envoie un embed d'anomalie dans anomaly_channel_id.
async fn post_anomaly_embed(
    ctx: &Context,
    guild_id: &str,
    anomaly_type: &str,
    count: usize,
    window_secs: u64,
    context_info: &str,
) {
    let embed = critical_embed(format!("🚨 ANOMALIE — {}", anomaly_type))
        .field("Count", count.to_string(), true)
        .field("Fenetre", format!("{}s", window_secs), true)
        .description(format!(
            "Un pattern anormal a ete detecte sur la guild.\n{}",
            context_info
        ))
        .timestamp(serenity::model::Timestamp::now())
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Audit | Sentinel — Urgence",
        ));
    Handler::post_to_channel(ctx, guild_id, &["anomaly_channel_id"], embed).await;
}

pub async fn handle_delete(
    ctx: &Context,
    channel_id: ChannelId,
    message_id: MessageId,
    guild_id: Option<GuildId>,
) {
    let gid = match guild_id {
        Some(g) => g,
        None => return,
    };
    let gid_str = gid.to_string();

    let channel_name = super::resolve_channel_name(ctx, channel_id).await;
    let chan_label = channel_name.clone().unwrap_or_else(|| "?".to_string());

    // Chercher dans le cache
    let data = ctx.data.read().await;
    let cached = data
        .get::<MessageCacheKey>()
        .and_then(|cache| cache.remove(gid, message_id));

    let (log_msg, details) = match &cached {
        Some(c) => {
            let preview = if c.content.chars().count() > 100 {
                format!("{}...", c.content.chars().take(100).collect::<String>())
            } else {
                c.content.clone()
            };
            (
                format!(
                    "Message de {} supprime dans #{} : \"{}\"",
                    c.author_name, chan_label, preview
                ),
                serde_json::json!({
                    "author_id": c.author_id,
                    "author_name": c.author_name,
                    "content": c.content,
                }),
            )
        }
        None => (
            format!("Message {} supprime dans #{}", message_id, chan_label),
            serde_json::json!({}),
        ),
    };

    Handler::log(ctx, "warn", &gid_str, &log_msg).await;

    let mut evt = audit_event::simple(gid_str.clone(), "message_delete")
        .with_channel(channel_id, channel_name)
        .with_details(details);
    evt.target_id = Some(message_id.to_string());
    if let Some(c) = &cached {
        evt.actor_id = Some(c.author_id.clone());
        evt.actor_name = Some(c.author_name.clone());
    }

    Handler::send_event(ctx, evt).await;

    // Surveillance : tracker la suppression si l'auteur est surveille
    if let Some(c) = &cached {
        Handler::track_activity(
            ctx, &gid_str, &c.author_id, "message_deleted",
            Some(&channel_id.to_string()), Some(&chan_label),
            Some(&c.content),
            serde_json::json!({"message_id": message_id.to_string()}),
        ).await;
    }

    // Weekly stats
    if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
        tracker.increment(gid, StatField::MessageDeleted);
    }
    drop(data);

    // Anomaly detection (on release le lock data d'abord pour pouvoir poster)
    let alert_opt = {
        let data = ctx.data.read().await;
        data.get::<AnomalyDetectorKey>()
            .and_then(|anomaly| anomaly.record(gid, "delete"))
    };
    if let Some(alert) = alert_opt {
        Handler::log(
            ctx,
            "error",
            &gid_str,
            &format!("ANOMALIE : {} ({} en {}s)", alert.anomaly_type, alert.count, alert.window_secs),
        ).await;

        post_anomaly_embed(
            ctx, &gid_str,
            &alert.anomaly_type,
            alert.count,
            alert.window_secs,
            &format!("Dernier salon : <#{}>", channel_id),
        ).await;

        Handler::send_event(
            ctx,
            audit_event::simple(gid_str.clone(), "anomaly_detected")
                .with_details(serde_json::json!({
                    "anomaly_type": alert.anomaly_type,
                    "count": alert.count,
                    "window_secs": alert.window_secs,
                })),
        ).await;

        let data = ctx.data.read().await;
        if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
            tracker.increment(gid, StatField::Anomaly);
        }
    }
}

pub async fn handle_update(
    ctx: &Context,
    old: Option<Message>,
    _new: Option<Message>,
    event: MessageUpdateEvent,
) {
    let gid = match event.guild_id {
        Some(g) => g.to_string(),
        None => return,
    };

    // Ignorer les messages edites par des bots
    if event.author.as_ref().map(|a| a.bot).unwrap_or(false) {
        return;
    }

    let author_id = event.author.as_ref().map(|a| a.id.to_string());
    let author_name = event.author.as_ref().map(|a| a.name.clone());
    let new_content = event.content.clone().unwrap_or_default();
    let old_content = old.as_ref().map(|m| m.content.clone()).unwrap_or_default();

    let name = author_name.as_deref().unwrap_or("?");
    Handler::log(ctx, "info", &gid, &format!(
        "{} a modifie un message — avant: \"{}\" | apres: \"{}\"",
        name,
        if old_content.is_empty() { "(inconnu)" } else { &old_content },
        new_content
    )).await;

    let mut evt = audit_event::simple(gid.clone(), "message_edit")
        .with_channel(event.channel_id, None)
        .with_details(serde_json::json!({
            "old_content": old_content,
            "new_content": new_content,
        }));
    evt.target_id = Some(event.id.to_string());
    evt.actor_id = author_id;
    evt.actor_name = author_name;

    Handler::send_event(ctx, evt).await;

    // Surveillance : tracker l'edition si l'auteur est surveille
    if let Some(ref author) = event.author {
        Handler::track_activity(
            ctx, &gid, &author.id.to_string(), "message_edited",
            Some(&event.channel_id.to_string()), None,
            Some(&new_content),
            serde_json::json!({"old_content": old_content, "message_id": event.id.to_string()}),
        ).await;
    }

    // Weekly stats
    let data = ctx.data.read().await;
    if let Some(guild_id) = event.guild_id {
        if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
            tracker.increment(guild_id, StatField::MessageEdited);
        }
    }
}

pub async fn handle_delete_bulk(
    ctx: &Context,
    channel_id: ChannelId,
    multiple_deleted: Vec<MessageId>,
    guild_id: Option<GuildId>,
) {
    let gid = match guild_id {
        Some(g) => g,
        None => return,
    };
    let gid_str = gid.to_string();

    let count = multiple_deleted.len();
    let channel_name = super::resolve_channel_name(ctx, channel_id).await;
    let chan_label = channel_name.as_deref().unwrap_or("?");

    Handler::log(ctx, "error", &gid_str, &format!(
        "Purge : {} messages supprimes dans #{}", count, chan_label
    )).await;

    Handler::send_event(
        ctx,
        audit_event::simple(gid_str.clone(), "message_delete_bulk")
            .with_channel(channel_id, channel_name)
            .with_details(serde_json::json!({
                "count": count,
                "message_ids": multiple_deleted.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            })),
    )
    .await;

    // Anomaly : compter comme N deletes, capturer l'alerte eventuelle sans
    // tenir le lock pendant l'envoi Discord.
    let alert_opt = {
        let data = ctx.data.read().await;
        let mut found = None;
        if let Some(anomaly) = data.get::<AnomalyDetectorKey>() {
            for _ in 0..count {
                if let Some(alert) = anomaly.record(gid, "delete") {
                    found = Some(alert);
                    break;
                }
            }
        }
        found
    };

    if let Some(alert) = alert_opt {
        Handler::log(
            ctx,
            "error",
            &gid_str,
            &format!("ANOMALIE : {} ({} en {}s)", alert.anomaly_type, alert.count, alert.window_secs),
        ).await;

        post_anomaly_embed(
            ctx, &gid_str,
            &alert.anomaly_type,
            alert.count,
            alert.window_secs,
            &format!("Purge bulk dans <#{}> ({} messages)", channel_id, count),
        ).await;

        Handler::send_event(
            ctx,
            audit_event::simple(gid_str.clone(), "anomaly_detected")
                .with_details(serde_json::json!({
                    "anomaly_type": alert.anomaly_type,
                    "count": alert.count,
                    "window_secs": alert.window_secs,
                })),
        ).await;

        let data = ctx.data.read().await;
        if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
            tracker.increment(gid, StatField::Anomaly);
        }
    }

    // Weekly stats
    let data = ctx.data.read().await;
    if let Some(tracker) = data.get::<WeeklyTrackerKey>() {
        tracker.increment_deleted(gid, count as u64);
    }
}
