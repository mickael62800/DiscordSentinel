//! Rafraichissement automatique des cartes Tamagotchi dans Discord.
//!
//! Le worker applique la decroissance en base toutes les ~5 min, mais le
//! message Discord (la carte) restait fige tant que le joueur ne cliquait pas.
//! Cette tache parcourt les cartes vivantes (via `GET /api/tamagotchi/cards`,
//! pagine), re-rend le PNG et **edite** le message existant.
//!
//! La frequence est **configurable par serveur** depuis le panel web
//! (cle `card_refresh_interval_minutes` de la config `tamagotchi-bot`). La
//! boucle se reveille a une cadence de base courte et ne rafraichit les cartes
//! d'un serveur que si son intervalle configure est ecoule.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serenity::all::{
    ChannelId, Context, CreateAttachment, CreateEmbed, EditAttachments, EditMessage, MessageId,
};
use tracing::{info, warn};

use crate::shared::heartbeat::ApiClientKey;

use super::panel::{card_embed, care_buttons, render_card, PetDto};
use super::MODULE_BOT_NAME;

/// Cadence de base : resolution a laquelle on verifie si un serveur est "du".
const BASE_POLL_SECS: u64 = 60;
/// Intervalle de refresh par defaut si non configure (minutes).
const DEFAULT_REFRESH_MINUTES: u64 = 60;
/// Taille de page pour la pagination des cartes.
const PAGE: i64 = 500;

#[derive(serde::Deserialize)]
struct CardItem {
    id: String,
    guild_id: String,
    owner_id: String,
    card_channel_id: String,
    card_message_id: String,
    #[serde(flatten)]
    pet: PetDto,
}

/// Spawn la boucle de rafraichissement. Appelee une fois au `ready`.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        // Dernier refresh effectue par serveur (pour gater selon l'intervalle
        // configure de chaque guild).
        let mut last_refresh: HashMap<String, Instant> = HashMap::new();
        loop {
            tokio::time::sleep(Duration::from_secs(BASE_POLL_SECS)).await;
            refresh_due(&ctx, &mut last_refresh).await;
        }
    });
}

async fn refresh_due(ctx: &Context, last_refresh: &mut HashMap<String, Instant>) {
    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };

    // 1. Recupere toutes les cartes vivantes (paginees), groupees par serveur.
    let mut by_guild: HashMap<String, Vec<CardItem>> = HashMap::new();
    let mut after: Option<String> = None;
    loop {
        let path = match &after {
            Some(cursor) => format!("/api/tamagotchi/cards?limit={PAGE}&after={cursor}"),
            None => format!("/api/tamagotchi/cards?limit={PAGE}"),
        };
        let batch: Vec<CardItem> = match api.get_json(&path).await {
            Ok(b) => b,
            Err(e) => {
                warn!(error = %e, "Echec fetch cartes tamagotchi a rafraichir");
                return;
            }
        };
        if batch.is_empty() {
            break;
        }
        let batch_len = batch.len();
        after = batch.last().map(|c| c.id.clone());
        for item in batch {
            by_guild.entry(item.guild_id.clone()).or_default().push(item);
        }
        if batch_len < PAGE as usize {
            break;
        }
    }

    // 2. Pour chaque serveur, refresh seulement si son intervalle est ecoule.
    let now = Instant::now();
    let mut refreshed = 0usize;
    for (guild_id, cards) in by_guild {
        let interval = Duration::from_secs(guild_interval_minutes(&api, &guild_id).await * 60);
        let due = match last_refresh.get(&guild_id) {
            Some(prev) => now.duration_since(*prev) >= interval,
            None => true, // premiere fois : on rafraichit
        };
        if !due {
            continue;
        }
        last_refresh.insert(guild_id.clone(), now);
        for item in &cards {
            if edit_card(ctx, &api, item).await {
                refreshed += 1;
            }
        }
    }

    if refreshed > 0 {
        info!(refreshed, "Cartes tamagotchi rafraichies");
    }
}

/// Lit `card_refresh_interval_minutes` depuis la config guild (defaut 60, min 1).
async fn guild_interval_minutes(api: &crate::shared::api_client::BaseApiClient, guild_id: &str) -> u64 {
    let cfg = api.get_guild_config_for(guild_id, MODULE_BOT_NAME).await.unwrap_or_default();
    cfg.get("card_refresh_interval_minutes")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_REFRESH_MINUTES)
        .max(1)
}

/// Re-rend et edite la carte d'un compagnon. Retourne true si l'edition a reussi.
async fn edit_card(
    ctx: &Context,
    api: &crate::shared::api_client::BaseApiClient,
    item: &CardItem,
) -> bool {
    let channel_id = match item.card_channel_id.parse::<u64>() {
        Ok(n) => ChannelId::new(n),
        Err(_) => return false,
    };
    let message_id = match item.card_message_id.parse::<u64>() {
        Ok(n) => MessageId::new(n),
        Err(_) => return false,
    };

    let edit = match render_card(api, &item.guild_id, &item.owner_id, &item.pet).await {
        Some(png) => EditMessage::new()
            .embed(CreateEmbed::new().image("attachment://card.png").color(0x232838))
            .attachments(EditAttachments::new().add(CreateAttachment::bytes(png, "card.png")))
            .components(care_buttons()),
        None => EditMessage::new()
            .embed(card_embed(&item.pet))
            .components(care_buttons()),
    };

    match channel_id.edit_message(&ctx.http, message_id, edit).await {
        Ok(_) => true,
        Err(e) => {
            // Message probablement supprime (salon ferme) : log et on continue.
            warn!(error = %e, channel = %channel_id, "Echec edition carte tamagotchi");
            false
        }
    }
}
