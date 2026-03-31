use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage};
use serenity::model::id::ChannelId;
use serenity::prelude::*;

use crate::handler::ConfigKey;

async fn send_log(ctx: &Context, embed: CreateEmbed) {
    let log_channel = {
        let data = ctx.data.read().await;
        data.get::<ConfigKey>()
            .and_then(|config| config.log_channel_id)
    };

    if let Some(channel_id) = log_channel {
        let msg = CreateMessage::new().embed(embed);
        if let Err(why) = channel_id.send_message(&ctx.http, msg).await {
            tracing::error!(error = %why, "Erreur envoi log embed");
        }
    }
}

pub async fn log_channel_created(
    ctx: &Context,
    creator_id: u64,
    channel_type: &str,
    channel_name: &str,
    options: &str,
) {
    let type_emoji = if channel_type == "private" { "🔒" } else { "🔊" };
    let type_label = if channel_type == "private" { "Prive" } else { "Public" };

    let embed = CreateEmbed::new()
        .title(format!("{} Salon vocal cree", type_emoji))
        .color(0x2ecc71)
        .field("Createur", format!("<@{creator_id}>"), true)
        .field("Type", type_label, true)
        .field("Nom", channel_name, true)
        .field("Options", if options.is_empty() { "Aucune" } else { options }, false)
        .footer(CreateEmbedFooter::new("Voice Bot"))
        .timestamp(serenity::model::Timestamp::now());

    send_log(ctx, embed).await;
}

pub async fn log_channel_deleted(ctx: &Context, channel_name: &str, channel_type: &str) {
    let type_emoji = if channel_type == "private" { "🔒" } else { "🔊" };

    let embed = CreateEmbed::new()
        .title(format!("🗑️ Salon vocal supprime"))
        .color(0xe74c3c)
        .field("Nom", channel_name, true)
        .field("Type", format!("{} {}", type_emoji, channel_type), true)
        .footer(CreateEmbedFooter::new("Voice Bot"))
        .timestamp(serenity::model::Timestamp::now());

    send_log(ctx, embed).await;
}

pub async fn log_member_joined(ctx: &Context, user_id: u64, channel_name: &str) {
    let embed = CreateEmbed::new()
        .title("➡️ Membre rejoint")
        .color(0x3498db)
        .field("Membre", format!("<@{user_id}>"), true)
        .field("Salon", channel_name, true)
        .footer(CreateEmbedFooter::new("Voice Bot"))
        .timestamp(serenity::model::Timestamp::now());

    send_log(ctx, embed).await;
}

pub async fn log_member_left(ctx: &Context, user_id: u64, channel_name: &str) {
    let embed = CreateEmbed::new()
        .title("⬅️ Membre parti")
        .color(0x95a5a6)
        .field("Membre", format!("<@{user_id}>"), true)
        .field("Salon", channel_name, true)
        .footer(CreateEmbedFooter::new("Voice Bot"))
        .timestamp(serenity::model::Timestamp::now());

    send_log(ctx, embed).await;
}

pub async fn log_vote_kick(ctx: &Context, target_id: u64, channel_name: &str, result: &str) {
    let (emoji, color) = if result == "expulse" {
        ("👢", 0xe74c3c)
    } else {
        ("✅", 0x2ecc71)
    };

    let embed = CreateEmbed::new()
        .title(format!("{} Vote kick — {}", emoji, result))
        .color(color)
        .field("Cible", format!("<@{target_id}>"), true)
        .field("Salon", channel_name, true)
        .field("Resultat", result, true)
        .footer(CreateEmbedFooter::new("Voice Bot"))
        .timestamp(serenity::model::Timestamp::now());

    send_log(ctx, embed).await;
}

pub async fn log_transfer(ctx: &Context, from_id: u64, to_id: u64, channel_name: &str) {
    let embed = CreateEmbed::new()
        .title("🔄 Transfert de propriete")
        .color(0xf39c12)
        .field("Ancien proprietaire", format!("<@{from_id}>"), true)
        .field("Nouveau proprietaire", format!("<@{to_id}>"), true)
        .field("Salon", channel_name, false)
        .footer(CreateEmbedFooter::new("Voice Bot"))
        .timestamp(serenity::model::Timestamp::now());

    send_log(ctx, embed).await;
}

pub async fn log_ban(ctx: &Context, user_id: u64, channel_name: &str, duration: &str) {
    let embed = CreateEmbed::new()
        .title("🚫 Membre banni du salon")
        .color(0xe74c3c)
        .field("Membre", format!("<@{user_id}>"), true)
        .field("Salon", channel_name, true)
        .field("Duree", duration, true)
        .footer(CreateEmbedFooter::new("Voice Bot"))
        .timestamp(serenity::model::Timestamp::now());

    send_log(ctx, embed).await;
}

pub async fn log_flood_mute(ctx: &Context, user_id: u64, channel_name: &str, duration_secs: u64) {
    let embed = CreateEmbed::new()
        .title("🔇 Mute anti-flood")
        .color(0xef4444)
        .field("Membre", format!("<@{user_id}>"), true)
        .field("Salon", channel_name, true)
        .field("Duree", format!("{} secondes", duration_secs), true)
        .footer(CreateEmbedFooter::new("Voice Bot"))
        .timestamp(serenity::model::Timestamp::now());

    send_log(ctx, embed).await;
}

pub async fn log_afk_move(ctx: &Context, user_id: u64, from_channel: &str, to_channel: &str, afk_minutes: u64) {
    let embed = CreateEmbed::new()
        .title("💤 Membre deplace (AFK)")
        .color(0xf39c12)
        .field("Membre", format!("<@{user_id}>"), true)
        .field("Depuis", from_channel, true)
        .field("Vers", to_channel, true)
        .field("Duree AFK", format!("{} minutes", afk_minutes), false)
        .footer(CreateEmbedFooter::new("Voice Bot"))
        .timestamp(serenity::model::Timestamp::now());

    send_log(ctx, embed).await;
}

pub async fn get_channel_name(ctx: &Context, channel_id: ChannelId) -> String {
    channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|ch| ch.guild())
        .map(|gc| gc.name.clone())
        .unwrap_or_else(|| format!("{channel_id}"))
}
