//! Bump rewards : detecte un /bump Disboard reussi, recompense l'auteur en
//! coins (recompense graduee selon le nombre de bumps de la semaine, calculee
//! cote API) et rappelle quand un nouveau bump est possible apres le cooldown.

use std::time::Duration;

use serenity::all::MessageInteractionMetadata;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

pub const MODULE_BOT_NAME: &str = "bump-bot";

/// User id du bot Disboard (auteur des messages de confirmation de /bump).
const DISBOARD_ID: u64 = 302050872383242240;

/// `true` si l'embed Disboard indique un bump REUSSI (et pas un cooldown/echec).
fn is_bump_success(msg: &Message) -> bool {
    let mut positive = false;
    for e in &msg.embeds {
        let desc = e.description.as_deref().unwrap_or("").to_lowercase();
        // Echec / cooldown Disboard : "please wait ... minutes", "patienter".
        if desc.contains("minutes") || desc.contains("wait") || desc.contains("patient") {
            return false;
        }
        if desc.contains("done")
            || desc.contains("effectu")
            || desc.contains("👍")
            || desc.contains(":thumbsup:")
        {
            positive = true;
        }
    }
    positive
}

#[derive(serde::Serialize)]
struct RecordBumpBody {
    username: String,
    channel_id: String,
}

#[derive(serde::Deserialize, Default)]
struct BumpRewardResp {
    #[serde(default)]
    rewarded: bool,
    #[serde(default)]
    reward: i64,
    #[serde(default)]
    weekly_count: i64,
    /// Role VIP a poser (None si feature off ou seuil pas atteint).
    #[serde(default)]
    vip_role_id: Option<String>,
    /// True uniquement au bump qui debloque le VIP (annonce one-shot).
    #[serde(default)]
    vip_just_unlocked: bool,
}

/// Appele pour chaque message : si c'est une confirmation de bump Disboard
/// reussie, recompense l'auteur du /bump.
pub async fn on_message(ctx: &Context, msg: &Message) {
    if msg.author.id.get() != DISBOARD_ID {
        return;
    }
    let Some(guild_id) = msg.guild_id else { return };
    let guild_id = guild_id.to_string();

    // DIAGNOSTIC TEMPORAIRE : trace l'etat du message Disboard pour comprendre
    // pourquoi la recompense ne se declenche pas (interaction_metadata absent ?
    // embed non reconnu ?). A retirer une fois le bug bump identifie.
    info!(
        guild_id,
        has_interaction_metadata = msg.interaction_metadata.is_some(),
        is_command_interaction = matches!(
            msg.interaction_metadata.as_deref(),
            Some(MessageInteractionMetadata::Command(_))
        ),
        embed_count = msg.embeds.len(),
        embed_desc = msg
            .embeds
            .first()
            .and_then(|e| e.description.as_deref())
            .unwrap_or("<aucune>"),
        content = %msg.content,
        detected_success = is_bump_success(msg),
        "DIAG bump: message Disboard recu"
    );

    // L'auteur du /bump : present dans interaction_metadata (reponse de commande).
    let bumper = match msg.interaction_metadata.as_deref() {
        Some(MessageInteractionMetadata::Command(meta)) => &meta.user,
        _ => return,
    };
    if bumper.bot {
        return;
    }
    if !is_bump_success(msg) {
        return;
    }

    // Module actif pour cette guild ?
    let cfg =
        crate::shared::discord_helpers::guild_config_or_default(ctx, &guild_id, MODULE_BOT_NAME)
            .await;
    if !BaseApiClient::config_bool(&cfg, "enabled", false) {
        return;
    }

    let api = {
        let data = ctx.data.read().await;
        match data.get::<ApiClientKey>() {
            Some(a) => a.clone(),
            None => return,
        }
    };

    let body = RecordBumpBody {
        username: bumper.name.clone(),
        channel_id: msg.channel_id.to_string(),
    };
    let resp: BumpRewardResp = match api
        .post_json(&format!("/api/bump/{}/{}", guild_id, bumper.id), &body)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, guild_id, "Echec enregistrement bump");
            return;
        }
    };
    // Role VIP : pose le role (idempotent) des que l'API le renvoie, et
    // annonce le passage VIP une seule fois (vip_just_unlocked). Independant
    // du montant de coins : un bump qui rapporte 0 compte quand meme.
    if let Some(role_id_str) = resp.vip_role_id.as_deref() {
        if let (Ok(gid), Ok(rid)) = (guild_id.parse::<u64>(), role_id_str.parse::<u64>()) {
            let gid = serenity::model::id::GuildId::new(gid);
            let rid = serenity::model::id::RoleId::new(rid);
            if let Err(e) = ctx
                .http
                .add_member_role(
                    gid,
                    bumper.id,
                    rid,
                    Some("Bump VIP — seuil de bumps atteint"),
                )
                .await
            {
                warn!(error = %e, user = %bumper.id, role = %rid, "Echec attribution role VIP bump");
            } else if resp.vip_just_unlocked {
                let vip_msg = format!(
                    "👑 <@{}> est maintenant **VIP** grâce à ses bumps ! Merci pour ton soutien au serveur 🙌",
                    bumper.id
                );
                if let Err(e) = msg.channel_id.say(&ctx.http, vip_msg).await {
                    warn!(error = %e, "Echec annonce passage VIP bump");
                }
                info!(guild_id, user = %bumper.id, "Membre passe VIP via bumps");
            }
        }
    }

    if !resp.rewarded || resp.reward <= 0 {
        return;
    }

    let content = format!(
        "🎉 Merci <@{}> pour le **bump** ! **+{} coins** (bump #{} de la semaine)",
        bumper.id, resp.reward, resp.weekly_count
    );
    if let Err(e) = msg.channel_id.say(&ctx.http, content).await {
        warn!(error = %e, "Echec annonce recompense bump");
    }
    info!(guild_id, user = %bumper.id, reward = resp.reward, "Bump recompense");
}

#[derive(serde::Deserialize)]
struct DueReminder {
    guild_id: String,
    channel_id: String,
}

/// Tache de fond : poste un rappel quand le cooldown de bump est ecoule.
pub fn spawn_background(ctx: Context) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let api = {
                let data = ctx.data.read().await;
                data.get::<ApiClientKey>().cloned()
            };
            let Some(api) = api else { continue };
            let due: Vec<DueReminder> = api
                .get_json("/api/bump/due-reminders")
                .await
                .unwrap_or_default();
            for d in due {
                if let Ok(cid) = d.channel_id.parse::<u64>() {
                    let _ = ChannelId::new(cid)
                        .say(
                            &ctx.http,
                            "⏰ Le serveur peut être **bumpé** à nouveau ! Faites `/bump` (Disboard) pour le faire remonter — et gagner des coins.",
                        )
                        .await;
                }
                // Best-effort : on marque envoye meme si le post echoue, pour ne
                // pas spammer le rappel a chaque tick.
                let _ = api
                    .post_json::<_, serde_json::Value>(
                        &format!("/api/bump/{}/reminder-sent", d.guild_id),
                        &serde_json::json!({}),
                    )
                    .await;
            }
        }
    });
}
