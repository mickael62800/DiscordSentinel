//! Consumer stream : reagit aux transitions de cycle de vie d'un compagnon
//! (`tamagotchi_pet_status`, publie par l'API lors du tick worker).
//!
//! Sur maladie / mort / guerison :
//! 1. DM au proprietaire pour le prevenir.
//! 2. Edition immediate de la carte Discord (si sa position est connue) pour
//!    refleter le nouvel etat sans attendre le refresh horaire.

use serenity::all::{
    ChannelId, Context, CreateAttachment, CreateEmbed, EditAttachments, EditMessage, MessageId,
    UserId,
};
use tracing::{info, warn};

use crate::shared::heartbeat::ApiClientKey;

use super::panel::{card_embed, care_buttons, render_card, PetDto};

/// Spawn le consumer durable. Appele une fois au `ready`.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        let consumer = crate::shared::event_bus::default_consumer_name();
        crate::shared::event_bus::listen_stream_group(
            "tamagotchi-bot-lifecycle".to_string(),
            consumer,
            move |payload_json| {
                let ctx = ctx.clone();
                async move {
                    handle_event(&ctx, &payload_json).await;
                }
            },
        )
        .await;
    });
}

async fn handle_event(ctx: &Context, payload_json: &str) {
    let event: serde_json::Value = match serde_json::from_str(payload_json) {
        Ok(v) => v,
        Err(_) => return,
    };
    if event.get("event").and_then(|v| v.as_str()) != Some("tamagotchi_pet_status") {
        return;
    }
    let data = match event.get("data") {
        Some(d) => d,
        None => return,
    };

    let guild_id = data.get("guild_id").and_then(|v| v.as_str()).unwrap_or("");
    let owner_id = data.get("owner_id").and_then(|v| v.as_str()).unwrap_or("");
    let pet_name = data.get("pet_name").and_then(|v| v.as_str()).unwrap_or("Ton compagnon");
    let status = data.get("status").and_then(|v| v.as_str()).unwrap_or("");
    if owner_id.is_empty() {
        return;
    }

    // 1. DM au proprietaire.
    let dm_text = match status {
        "sick" => format!(
            "🤒 Ton compagnon **{pet_name}** est tombé malade ! Soigne-le vite (une potion de soin a la boutique) avant qu'il ne soit trop tard."
        ),
        "death" => format!(
            "🪦 Ton compagnon **{pet_name}** est mort... Son salon va etre supprime. Rouvre-en un depuis le panneau pour adopter un nouveau compagnon."
        ),
        "recovered" => format!("💚 Bonne nouvelle : **{pet_name}** est gueri !"),
        _ => return,
    };
    send_dm(ctx, owner_id, &dm_text).await;

    // 2. Mort : on supprime le salon prive du compagnon (s'il est connu) pour
    // que le joueur puisse en rouvrir un neuf et adopter. Le cadavre est purge
    // cote API a la prochaine creation. Sinon : edition immediate de la carte.
    let (channel_id, message_id) = (
        data.get("card_channel_id").and_then(|v| v.as_str()),
        data.get("card_message_id").and_then(|v| v.as_str()),
    );
    if status == "death" {
        if let Some(ch) = channel_id {
            delete_channel(ctx, ch).await;
        }
    } else if let (Some(ch), Some(msg)) = (channel_id, message_id) {
        if !guild_id.is_empty() {
            edit_card(ctx, guild_id, owner_id, ch, msg).await;
        }
    }

    info!(owner = %owner_id, status, "Notif cycle de vie tamagotchi traitee");
}

/// Supprime le salon prive du compagnon (a la mort). No-op si l'ID est
/// invalide ; on logge l'echec (salon deja supprime, droits manquants...).
async fn delete_channel(ctx: &Context, channel: &str) {
    let channel_id = match channel.parse::<u64>() {
        Ok(n) => ChannelId::new(n),
        Err(_) => return,
    };
    if let Err(e) = channel_id.delete(&ctx.http).await {
        warn!(error = %e, channel = %channel_id, "Echec suppression salon tamagotchi (mort)");
    } else {
        info!(channel = %channel_id, "Salon tamagotchi supprime (mort du compagnon)");
    }
}

async fn send_dm(ctx: &Context, owner_id: &str, text: &str) {
    let uid = match owner_id.parse::<u64>() {
        Ok(n) => UserId::new(n),
        Err(_) => return,
    };
    match uid.create_dm_channel(&ctx.http).await {
        Ok(dm) => {
            if let Err(e) = dm.id.say(&ctx.http, text).await {
                warn!(error = %e, owner = %owner_id, "Echec DM tamagotchi (DM fermes ?)");
            }
        }
        Err(e) => warn!(error = %e, owner = %owner_id, "Echec ouverture DM tamagotchi"),
    }
}

async fn edit_card(ctx: &Context, guild_id: &str, owner_id: &str, channel: &str, message: &str) {
    let channel_id = match channel.parse::<u64>() {
        Ok(n) => ChannelId::new(n),
        Err(_) => return,
    };
    let message_id = match message.parse::<u64>() {
        Ok(n) => MessageId::new(n),
        Err(_) => return,
    };

    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };

    // Re-fetch l'etat courant pour rendre la carte a jour.
    let pet: PetDto = match api
        .get_json(&format!("/api/tamagotchi/{guild_id}/{owner_id}"))
        .await
    {
        Ok(p) => p,
        Err(_) => return,
    };

    let edit = match render_card(&api, guild_id, owner_id, &pet).await {
        Some(png) => EditMessage::new()
            .embed(CreateEmbed::new().image("attachment://card.png").color(0x232838))
            .attachments(EditAttachments::new().add(CreateAttachment::bytes(png, "card.png")))
            .components(care_buttons()),
        None => EditMessage::new().embed(card_embed(&pet)).components(care_buttons()),
    };

    if let Err(e) = channel_id.edit_message(&ctx.http, message_id, edit).await {
        warn!(error = %e, channel = %channel_id, "Echec edition carte (cycle de vie)");
    }
}
