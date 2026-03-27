use serenity::builder::{CreateEmbed, CreateMessage};
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
    let embed = CreateEmbed::new()
        .title("Salon cree")
        .description(format!(
            "**Createur :** <@{creator_id}>\n\
            **Type :** {channel_type}\n\
            **Nom :** {channel_name}\n\
            **Options :** {options}"
        ))
        .color(0x2ecc71)
        .timestamp(serenity::model::Timestamp::now());

    send_log(ctx, embed).await;
}

pub async fn log_channel_deleted(ctx: &Context, channel_name: &str, channel_type: &str) {
    let embed = CreateEmbed::new()
        .title("Salon supprime")
        .description(format!(
            "**Nom :** {channel_name}\n\
            **Type :** {channel_type}"
        ))
        .color(0xe74c3c)
        .timestamp(serenity::model::Timestamp::now());

    send_log(ctx, embed).await;
}

pub async fn log_member_joined(ctx: &Context, user_id: u64, channel_name: &str) {
    let embed = CreateEmbed::new()
        .title("Membre rejoint")
        .description(format!(
            "**Membre :** <@{user_id}>\n\
            **Salon :** {channel_name}"
        ))
        .color(0x3498db)
        .timestamp(serenity::model::Timestamp::now());

    send_log(ctx, embed).await;
}

pub async fn log_member_left(ctx: &Context, user_id: u64, channel_name: &str) {
    let embed = CreateEmbed::new()
        .title("Membre parti")
        .description(format!(
            "**Membre :** <@{user_id}>\n\
            **Salon :** {channel_name}"
        ))
        .color(0x95a5a6)
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
