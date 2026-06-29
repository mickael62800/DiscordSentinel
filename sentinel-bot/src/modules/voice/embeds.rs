use serenity::model::id::{ChannelId, GuildId};
use serenity::prelude::*;

use super::session_card::SessionCard;
use super::{ConfigKey, SessionCardKey};
use crate::shared::heartbeat::ApiClientKey;

/// Si le clone a obtenu un log_message_id (renvoi initial), le reecrit dans le DashMap.
async fn sync_message_id(ctx: &Context, voice_channel_id: ChannelId, card: &SessionCard) {
    if let Some(mid) = card.log_message_id {
        let data = ctx.data.read().await;
        if let Some(cards) = data.get::<SessionCardKey>() {
            if let Some(mut entry) = cards.get_mut(&voice_channel_id) {
                if entry.log_message_id.is_none() {
                    entry.log_message_id = Some(mid);
                }
            }
        }
    }
}

fn get_log_channel(data: &tokio::sync::RwLockReadGuard<'_, TypeMap>) -> Option<ChannelId> {
    data.get::<ConfigKey>()
        .and_then(|config| config.log_channel_id)
}

/// Resout le salon de logs vocaux : priorite a la config guild
/// (`log_channel_id` dans bot_guild_config, configurable depuis la page
/// Composants), fallback sur la variable d'env VOICE_LOG_CHANNEL_ID.
async fn resolve_log_channel(ctx: &Context, guild_id: GuildId) -> Option<ChannelId> {
    let data = ctx.data.read().await;
    if let Some(api) = data.get::<ApiClientKey>() {
        if let Ok(cfg) = api
            .get_guild_config_for(
                &guild_id.to_string(),
                crate::modules::voice::MODULE_BOT_NAME,
            )
            .await
        {
            if let Some(id) = cfg
                .get("log_channel_id")
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|id| *id > 0)
            {
                return Some(ChannelId::new(id));
            }
        }
    }
    // Fallback env (retro-compat).
    get_log_channel(&data)
}

/// Cree et envoie une nouvelle carte de session dans le salon de logs.
pub async fn create_session_card(
    ctx: &Context,
    guild_id: GuildId,
    voice_channel_id: ChannelId,
    creator_name: &str,
    channel_type: &str,
) {
    let log_channel = match resolve_log_channel(ctx, guild_id).await {
        Some(ch) => ch,
        None => return,
    };

    let mut card = SessionCard::new(
        log_channel,
        creator_name.to_string(),
        channel_type.to_string(),
        chrono::Utc::now().timestamp(),
    );

    card.add_event(format!("\u{1f3a4} **{}** a cree le salon", creator_name));
    card.current_members = 1;

    card.send_initial(ctx).await;

    // Stocker la carte
    let data = ctx.data.read().await;
    if let Some(cards) = data.get::<SessionCardKey>() {
        cards.insert(voice_channel_id, card);
    }
}

/// Log un join en garantissant qu'une carte de session existe.
///
/// Les vocaux TEMPORAIRES recoivent leur carte a la creation du salon
/// (`create_session_card`). Les vocaux PERMANENTS observes n'en ont pas :
/// on la cree paresseusement au premier join (en partant du nombre de membres
/// deja presents), puis les join/leave suivants alimentent la meme carte.
/// Cloturee par `session_closed` quand le salon se vide (cf. member_events).
pub async fn ensure_card_and_member_joined(
    ctx: &Context,
    guild_id: GuildId,
    voice_channel_id: ChannelId,
    user_name: &str,
    current_member_count: u32,
) {
    let exists = {
        let data = ctx.data.read().await;
        data.get::<SessionCardKey>()
            .map(|c| c.contains_key(&voice_channel_id))
            .unwrap_or(false)
    };
    if exists {
        session_member_joined(ctx, voice_channel_id, user_name).await;
        return;
    }

    let log_channel = match resolve_log_channel(ctx, guild_id).await {
        Some(ch) => ch,
        None => return,
    };
    let channel_name = get_channel_name(ctx, voice_channel_id).await;
    let mut card = SessionCard::new(
        log_channel,
        channel_name,
        "permanent".to_string(),
        chrono::Utc::now().timestamp(),
    );
    card.current_members = current_member_count.max(1);
    card.add_event("\u{1f3a4} **Session demarree** (salon permanent observe)".to_string());
    card.add_event(format!(
        "\u{1f7e2}\u{27a1}\u{fe0f} **{}** a rejoint",
        user_name
    ));
    card.send_initial(ctx).await;

    let data = ctx.data.read().await;
    if let Some(cards) = data.get::<SessionCardKey>() {
        cards.insert(voice_channel_id, card);
    }
}

/// Ajoute un evenement "membre rejoint" a la carte de session.
pub async fn session_member_joined(ctx: &Context, voice_channel_id: ChannelId, user_name: &str) {
    let mut card_clone = {
        let data = ctx.data.read().await;
        let cards = match data.get::<SessionCardKey>() {
            Some(c) => c,
            None => return,
        };
        let mut entry = match cards.get_mut(&voice_channel_id) {
            Some(e) => e,
            None => return,
        };
        entry.current_members += 1;
        entry.add_event(format!(
            "\u{1f7e2}\u{27a1}\u{fe0f} **{}** a rejoint",
            user_name
        ));
        entry.clone()
    };
    card_clone.update(ctx).await;
    sync_message_id(ctx, voice_channel_id, &card_clone).await;
}

/// Ajoute un evenement "membre parti" a la carte de session.
pub async fn session_member_left(
    ctx: &Context,
    voice_channel_id: ChannelId,
    user_name: &str,
    duration_text: &str,
) {
    let mut card_clone = {
        let data = ctx.data.read().await;
        let cards = match data.get::<SessionCardKey>() {
            Some(c) => c,
            None => return,
        };
        let mut entry = match cards.get_mut(&voice_channel_id) {
            Some(e) => e,
            None => return,
        };
        entry.current_members = entry.current_members.saturating_sub(1);
        entry.add_event(format!(
            "\u{1f534}\u{2b05}\u{fe0f} **{}** a quitte ({})",
            user_name, duration_text
        ));
        entry.clone()
    };
    card_clone.update(ctx).await;
    sync_message_id(ctx, voice_channel_id, &card_clone).await;
}

/// Finalise la carte de session quand le salon est supprime.
pub async fn session_closed(ctx: &Context, voice_channel_id: ChannelId, total_duration: &str) {
    let mut card_clone = {
        let data = ctx.data.read().await;
        let cards = match data.get::<SessionCardKey>() {
            Some(c) => c,
            None => return,
        };
        let mut entry = match cards.get_mut(&voice_channel_id) {
            Some(e) => e,
            None => return,
        };
        entry.closed = true;
        entry.closed_at_unix = Some(chrono::Utc::now().timestamp());
        entry.total_duration = Some(total_duration.to_string());
        entry.add_event(format!(
            "\u{1f6d1} **Salon supprime** | Duree : {}",
            total_duration
        ));
        entry.clone()
    };
    card_clone.update(ctx).await;
    sync_message_id(ctx, voice_channel_id, &card_clone).await;

    // Nettoyer du cache
    let data = ctx.data.read().await;
    if let Some(cards) = data.get::<SessionCardKey>() {
        cards.remove(&voice_channel_id);
    }
}

/// Ajoute un evenement custom a la carte.
#[allow(dead_code)]
pub async fn session_event(ctx: &Context, voice_channel_id: ChannelId, text: &str) {
    let mut card_clone = {
        let data = ctx.data.read().await;
        let cards = match data.get::<SessionCardKey>() {
            Some(c) => c,
            None => return,
        };
        let mut entry = match cards.get_mut(&voice_channel_id) {
            Some(e) => e,
            None => return,
        };
        entry.add_event(text.to_string());
        entry.clone()
    };
    card_clone.update(ctx).await;
    sync_message_id(ctx, voice_channel_id, &card_clone).await;
}

// ── Helpers ──

pub async fn get_channel_name(ctx: &Context, channel_id: ChannelId) -> String {
    channel_id
        .to_channel(&ctx.http)
        .await
        .ok()
        .and_then(|ch| ch.guild())
        .map(|gc| gc.name.clone())
        .unwrap_or_else(|| format!("{channel_id}"))
}

// Legacy stub -- still called by afk_sweep
#[allow(dead_code)]
pub async fn log_afk_move(
    _ctx: &Context,
    _user_id: u64,
    _from_channel: &str,
    _to_channel: &str,
    _afk_minutes: u64,
) {
}
