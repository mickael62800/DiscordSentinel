//! Background task : ferme les tables de blackjack inactives après 30 minutes.

use std::sync::Arc;

use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::prelude::*;
use tracing::{info, warn};

use super::ChannelManagerKey;

/// Timeout AFK en secondes (30 minutes).
const AFK_TIMEOUT_SECS: u64 = 1800;

/// Spawn le sweep périodique. Appelé une seule fois au `ready`.
pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

            let data = ctx.data.read().await;
            let mgr = match data.get::<ChannelManagerKey>() {
                Some(m) => Arc::clone(m),
                None => continue,
            };
            drop(data);

            let afk = mgr.afk_channels(AFK_TIMEOUT_SECS);
            for (user_id, table) in afk {
                let embed = CreateEmbed::new()
                    .title("\u{23f0} Table fermee — Inactivite")
                    .description(
                        "Cette table de blackjack a ete fermee apres 30 minutes d'inactivite.",
                    )
                    .color(0x95A5A6);
                let _ = table
                    .channel_id
                    .send_message(&ctx.http, CreateMessage::new().embed(embed))
                    .await;

                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                if let Err(e) = table.channel_id.delete(&ctx.http).await {
                    warn!(error = %e, "Echec suppression channel AFK blackjack");
                } else {
                    info!(user = %user_id, "Table blackjack AFK supprimee");
                }

                mgr.remove(user_id);
            }
        }
    });
}
