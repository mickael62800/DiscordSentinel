//! Carte de changement de roles « vivante » (anti-spam).
//!
//! Probleme : Discord emet un `GUILD_MEMBER_UPDATE` par changement de role ->
//! une carte par role ajoute/retire = spam. Solution : une SEULE carte par
//! membre qui reste active pendant une fenetre glissante (defaut 5 min) et se
//! met a jour (edition) avec l'HISTORIQUE COMPLET des mouvements.
//!
//! L'ÉTAT (map fenêtrée, bornes, troncature) vit dans le core
//! (`services::audit::role_card`) ; ce module garde la config, le post/édit
//! Discord et l'embed.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use sentinel_core::domain::services::audit::role_card::{
    clamp_role_log_window, visible_movements, RoleMovement,
};
use serenity::all::{
    ChannelId, Context, CreateEmbed, CreateMessage, EditMessage, Member, MessageId, RoleId,
};
use serenity::prelude::TypeMapKey;

use crate::shared::heartbeat::ApiClientKey;

pub type RoleCardTracker =
    sentinel_core::domain::services::audit::role_card::RoleCardTracker<(String, String)>;

pub struct RoleCardTrackerKey;
impl TypeMapKey for RoleCardTrackerKey {
    type Value = std::sync::Arc<RoleCardTracker>;
}

/// Nombre max de lignes affichees dans la carte (limite champ embed).
const MAX_LINES: usize = 25;

/// Traite un changement de roles : cree ou met a jour la carte vivante.
pub async fn handle_role_change(
    ctx: &Context,
    guild_id: &str,
    member: &Member,
    added_now: &[RoleId],
    removed_now: &[RoleId],
) {
    if added_now.is_empty() && removed_now.is_empty() {
        return;
    }

    // Config audit-bot (fenetre + salon).
    let cfg = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(api) => api
                .get_guild_config_for(guild_id, super::MODULE_BOT_NAME)
                .await
                .unwrap_or_default(),
            None => return,
        }
    };
    let window = clamp_role_log_window(
        cfg.get("role_log_window_secs")
            .and_then(|v| v.parse::<u64>().ok()),
    );

    let tracker = {
        let data = ctx.data.read().await;
        match data.get::<RoleCardTrackerKey>() {
            Some(t) => t.clone(),
            None => return,
        }
    };

    let now = Instant::now();
    let key = (guild_id.to_string(), member.user.id.to_string());

    // Snapshot de la carte active (et purge des expirees). Lock court, pas d'await.
    let active = tracker.active(&key, now);

    // Historique cumule = existant + mouvements de cet evenement.
    let mut movements = active
        .as_ref()
        .map(|(_, _, m)| m.clone())
        .unwrap_or_default();
    for r in added_now {
        movements.push((true, r.to_string()));
    }
    for r in removed_now {
        movements.push((false, r.to_string()));
    }

    let embed = build_embed(member, &movements, window);
    let expires_at = now + Duration::from_secs(window);

    if let Some((chan, msg, _)) = active {
        // Edite la carte existante.
        let _ = ChannelId::new(chan)
            .edit_message(
                &ctx.http,
                MessageId::new(msg),
                EditMessage::new().embed(embed),
            )
            .await;
        tracker.update(&key, movements, expires_at);
    } else {
        // Nouvelle carte : resout le salon puis poste.
        let Some(chan) = resolve_channel(&cfg) else {
            return;
        };
        if let Ok(m) = chan
            .send_message(&ctx.http, CreateMessage::new().embed(embed))
            .await
        {
            tracker.insert(key, chan.get(), m.id.get(), movements, expires_at);
        }
    }
}

/// Salon cible : `profile_edit_channel_id` puis fallback `log_channel_id`.
fn resolve_channel(cfg: &HashMap<String, String>) -> Option<ChannelId> {
    for key in ["profile_edit_channel_id", "log_channel_id"] {
        if let Some(id) = cfg
            .get(key)
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&n| n > 0)
        {
            return Some(ChannelId::new(id));
        }
    }
    None
}

fn build_embed(member: &Member, movements: &[RoleMovement], window: u64) -> CreateEmbed {
    let total = movements.len();
    // Affiche les MAX_LINES plus recents (ordre chronologique conserve).
    let (hidden, shown) = visible_movements(movements, MAX_LINES);
    let mut body = String::new();
    if hidden > 0 {
        body.push_str(&format!("… ({hidden} mouvements plus anciens)\n"));
    }
    for (added, role) in shown {
        if *added {
            body.push_str(&format!("➕ <@&{role}>\n"));
        } else {
            body.push_str(&format!("➖ <@&{role}>\n"));
        }
    }
    if body.is_empty() {
        body.push('-');
    }

    let minutes = window / 60;
    crate::shared::embeds::info_embed("🎭 Rôles modifiés")
        .field("Membre", format!("<@{}>", member.user.id), true)
        .field("ID", member.user.id.to_string(), true)
        .field(format!("Changements ({total})"), body, false)
        .thumbnail(member.user.face())
        .timestamp(serenity::model::Timestamp::now())
        .footer(serenity::builder::CreateEmbedFooter::new(format!(
            "Audit | Sentinel — carte active {minutes} min",
        )))
}
