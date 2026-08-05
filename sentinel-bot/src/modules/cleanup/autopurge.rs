//! Auto-nettoyage programme de salons.
//!
//! Se configure entierement dans la page Composants -> Nettoyage automatique :
//! liste des salons (`autopurge_channel_ids`), frequence, periode de grace,
//! garder les messages du bot, journalisation. Une boucle de fond supprime
//! periodiquement les messages NON epingles des salons choisis.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serenity::all::GetMessages;
use serenity::model::id::{ChannelId, MessageId};
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

/// Cadence du scan (10 min). Un salon n'est reellement nettoye que si sa
/// frequence configuree est ecoulee depuis le dernier passage.
const TICK_SECS: u64 = 600;
/// bulk_delete Discord : uniquement < 14 jours.
const DISCORD_BULK_DELETE_MAX_AGE_SECS: i64 = 14 * 24 * 60 * 60;
/// Plafond de pages par salon et par passage (evite de bloquer sur un enorme
/// historique au 1er nettoyage ; le reste part au tick suivant).
const MAX_PAGES_PER_RUN: usize = 30;

pub fn spawn(ctx: Context) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    tokio::spawn(async move {
        // Dernier nettoyage par (guild, salon), en memoire.
        let mut last: HashMap<(u64, u64), Instant> = HashMap::new();
        tokio::time::sleep(Duration::from_secs(60)).await;
        loop {
            tick(&ctx, &mut last).await;
            tokio::time::sleep(Duration::from_secs(TICK_SECS)).await;
        }
    });
}

async fn tick(ctx: &Context, last: &mut HashMap<(u64, u64), Instant>) {
    let self_id = ctx.cache.current_user().id.get();
    for guild_id in ctx.cache.guilds() {
        let gid = guild_id.to_string();
        let cfg = crate::shared::discord_helpers::guild_config_or_default(
            ctx,
            &gid,
            super::MODULE_BOT_NAME,
        )
        .await;
        if !BaseApiClient::config_bool(&cfg, "enabled", false)
            || !BaseApiClient::config_bool(&cfg, "autopurge_enabled", false)
        {
            continue;
        }
        let interval_h = BaseApiClient::config_u64(&cfg, "autopurge_interval_hours", 24).max(1);
        let grace_h = BaseApiClient::config_u64(&cfg, "autopurge_grace_hours", 0);
        let keep_bot = BaseApiClient::config_bool(&cfg, "autopurge_keep_bot", false);
        let do_log = BaseApiClient::config_bool(&cfg, "autopurge_log", true);
        let channels_csv = BaseApiClient::config_or(&cfg, "autopurge_channel_ids", "");

        for id_str in channels_csv.split(',') {
            let Ok(cid) = id_str.trim().parse::<u64>() else {
                continue;
            };
            let key = (guild_id.get(), cid);
            let due = match last.get(&key) {
                Some(t) => t.elapsed() >= Duration::from_secs(interval_h * 3600),
                None => true, // premier passage
            };
            if !due {
                continue;
            }
            let (deleted, errors) = purge_channel(
                ctx,
                ChannelId::new(cid),
                grace_h as i64 * 3600,
                keep_bot,
                self_id,
            )
            .await;
            last.insert(key, Instant::now());

            if deleted > 0 || errors > 0 {
                info!(guild = %gid, channel = cid, deleted, errors, "auto-purge salon");
                if do_log {
                    let api = {
                        let data = ctx.data.read().await;
                        data.get::<ApiClientKey>().cloned()
                    };
                    if let Some(api) = api {
                        let suffix = if errors > 0 {
                            format!(", {errors} erreur(s)")
                        } else {
                            String::new()
                        };
                        api.send_log(
                            "info",
                            &gid,
                            &format!(
                                "Auto-nettoyage <#{cid}> : {deleted} message(s) supprime(s){suffix}."
                            ),
                        );
                    }
                }
            }
        }
    }
}

/// Supprime les messages NON epingles d'un salon, plus vieux que `grace_secs`
/// secondes. Si `keep_bot`, epargne les messages postes par CE bot (`self_id`).
/// Bulk delete pour les < 14 j, suppression individuelle au-dela.
async fn purge_channel(
    ctx: &Context,
    channel_id: ChannelId,
    grace_secs: i64,
    keep_bot: bool,
    self_id: u64,
) -> (u64, u64) {
    let mut deleted: u64 = 0;
    let mut errors: u64 = 0;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mut before: Option<MessageId> = None;

    for _ in 0..MAX_PAGES_PER_RUN {
        let mut req = GetMessages::new().limit(100);
        if let Some(b) = before {
            req = req.before(b);
        }
        let messages = match channel_id.messages(&ctx.http, req).await {
            Ok(m) => m,
            Err(e) => {
                warn!(error = %e, "auto-purge : echec fetch messages");
                break;
            }
        };
        if messages.is_empty() {
            break;
        }
        before = messages.last().map(|m| m.id);

        let mut recent: Vec<MessageId> = Vec::new();
        let mut old: Vec<MessageId> = Vec::new();
        for m in &messages {
            if m.pinned {
                continue;
            }
            if keep_bot && m.author.id.get() == self_id {
                continue;
            }
            let age = now - m.timestamp.unix_timestamp();
            if age < grace_secs {
                continue; // periode de grace
            }
            if age < DISCORD_BULK_DELETE_MAX_AGE_SECS {
                recent.push(m.id);
            } else {
                old.push(m.id);
            }
        }

        for chunk in recent.chunks(100) {
            if chunk.len() == 1 {
                if channel_id.delete_message(&ctx.http, chunk[0]).await.is_ok() {
                    deleted += 1;
                } else {
                    errors += 1;
                }
            } else {
                match channel_id.delete_messages(&ctx.http, chunk).await {
                    Ok(_) => deleted += chunk.len() as u64,
                    Err(_) => {
                        for &id in chunk {
                            if channel_id.delete_message(&ctx.http, id).await.is_ok() {
                                deleted += 1;
                            } else {
                                errors += 1;
                            }
                            tokio::time::sleep(Duration::from_millis(300)).await;
                        }
                    }
                }
            }
        }
        for &id in &old {
            if channel_id.delete_message(&ctx.http, id).await.is_ok() {
                deleted += 1;
            } else {
                errors += 1;
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }

    (deleted, errors)
}
