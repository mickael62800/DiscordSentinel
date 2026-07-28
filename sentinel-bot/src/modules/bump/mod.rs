//! Bump rewards : detecte un /bump reussi (Disboard ou DiscordL), recompense
//! l'auteur en coins (recompense graduee selon le nombre de bumps de la
//! semaine, calculee cote API) et rappelle quand un nouveau bump est possible
//! apres le cooldown.
//!
//! La detection (registre providers, succes/cooldown, resolution du bumpeur)
//! vit dans le core hexagonal (`sentinel_core::domain::services::bump`). Ce
//! module construit le DTO `BumpMessageFacts` depuis le `Message` Serenity et
//! garde l'orchestration Discord/API.

use std::time::Duration;

use sentinel_core::domain::services::bump::detection::{
    is_provider_bot, provider_by_key, provider_for_message, resolve_bumper, BumpAction,
    BumpMessageFacts, EmbedFacts, UserFacts, DISBOARD,
};
use serenity::all::MessageInteractionMetadata;
use serenity::model::channel::Message;
use serenity::model::id::{ChannelId, UserId};
use serenity::prelude::*;
use tracing::{debug, info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

pub const MODULE_BOT_NAME: &str = "bump-bot";

/// Construit le DTO de detection depuis le message Serenity.
fn message_facts(msg: &Message) -> BumpMessageFacts {
    BumpMessageFacts {
        author_id: msg.author.id.get(),
        embeds: msg
            .embeds
            .iter()
            .map(|e| EmbedFacts {
                title: e.title.clone(),
                description: e.description.clone(),
                fields: e
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), f.value.clone()))
                    .collect(),
            })
            .collect(),
        interaction_user: match msg.interaction_metadata.as_deref() {
            Some(MessageInteractionMetadata::Command(meta)) => Some(UserFacts {
                id: meta.user.id.get(),
                is_bot: meta.user.bot,
            }),
            _ => None,
        },
        mentions: msg
            .mentions
            .iter()
            .map(|m| UserFacts {
                id: m.id.get(),
                is_bot: m.bot,
            })
            .collect(),
    }
}

#[derive(serde::Serialize)]
struct RecordBumpBody {
    username: String,
    channel_id: String,
    provider: String,
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

/// Appele a chaque EDITION de message. Certains bots de bump (DiscordL)
/// repondent d'abord un message VIDE a l'interaction `/bump`, puis l'editent
/// pour y ajouter l'embed de resultat. On ne voit donc rien au MESSAGE_CREATE :
/// il faut re-lire le message a l'edition, quand l'embed est enfin present.
pub async fn on_message_update(
    ctx: &Context,
    event: &serenity::model::event::MessageUpdateEvent,
) {
    // Un embed vient-il d'apparaitre ? (sinon rien a detecter).
    let has_embeds = event.embeds.as_ref().map(|e| !e.is_empty()).unwrap_or(false);
    if !has_embeds {
        return;
    }
    // On ne refetch que les editions d'un bot PROVIDER connu (evite de refetch
    // tous les messages edites du serveur, ex: notre propre panneau d'aide).
    if let Some(author) = &event.author {
        if !is_provider_bot(author.id.get()) {
            return;
        }
    }
    // Re-lit le message complet (embed + interaction_metadata desormais la).
    if let Ok(msg) = event.channel_id.message(&ctx.http, event.id).await {
        on_message(ctx, &msg).await;
    }
}

/// Appele pour chaque message : si c'est une confirmation de bump reussie d'un
/// provider connu, recompense l'auteur du /bump.
pub async fn on_message(ctx: &Context, msg: &Message) {
    let facts = message_facts(msg);
    let Some(provider) = provider_for_message(&facts) else {
        // Un bot PROVIDER connu (bon bot_id) a poste un embed qu'aucune action
        // (bump/vote) n'a reconnu : probablement un changement de format du
        // provider -> a recalibrer. Log defensif (rare, uniquement en cas de casse).
        if is_provider_bot(msg.author.id.get()) && !msg.embeds.is_empty() {
            warn!(
                bot_id = msg.author.id.get(),
                bot_name = %msg.author.name,
                embed_desc = msg.embeds.first().and_then(|e| e.description.as_deref()).unwrap_or("<aucune>"),
                "bump: message d'un provider connu non reconnu (format change ?)"
            );
        }
        return;
    };
    let Some(guild_id) = msg.guild_id else { return };
    let guild_id = guild_id.to_string();

    let detected_success = (provider.detect)(&facts);
    let bumper_id = resolve_bumper(&facts).map(UserId::new);

    debug!(
        provider = provider.key,
        guild_id,
        resolved_bumper = bumper_id.map(|u| u.get()),
        detected_success,
        "bump: message provider traite"
    );

    let Some(bumper_id) = bumper_id else { return };
    if !detected_success {
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

    // Nom d'affichage du bumpeur (best-effort).
    let username = msg
        .mentions
        .iter()
        .find(|m| m.id == bumper_id)
        .map(|m| m.name.clone())
        .or_else(|| match msg.interaction_metadata.as_deref() {
            Some(MessageInteractionMetadata::Command(meta)) => Some(meta.user.name.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let body = RecordBumpBody {
        username,
        channel_id: msg.channel_id.to_string(),
        provider: provider.key.to_string(),
    };
    let resp: BumpRewardResp = match api
        .post_json(&format!("/api/bump/{}/{}", guild_id, bumper_id), &body)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, guild_id, provider = provider.key, "Echec enregistrement bump");
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
                    bumper_id,
                    rid,
                    Some("Bump VIP — seuil de bumps atteint"),
                )
                .await
            {
                warn!(error = %e, user = %bumper_id, role = %rid, "Echec attribution role VIP bump");
            } else if resp.vip_just_unlocked {
                let vip_msg = format!(
                    "👑 <@{}> est maintenant **VIP** grâce à ses bumps ! Merci pour ton soutien au serveur 🙌",
                    bumper_id
                );
                if let Err(e) = msg.channel_id.say(&ctx.http, vip_msg).await {
                    warn!(error = %e, "Echec annonce passage VIP bump");
                }
                info!(guild_id, user = %bumper_id, "Membre passe VIP via bumps");
            }
        }
    }

    if !resp.rewarded || resp.reward <= 0 {
        return;
    }

    let content = format!(
        "🎉 Merci <@{}> pour le **{}** ({}) ! **+{} coins** ({} #{} de la semaine)",
        bumper_id, provider.action.label(), provider.display, resp.reward, provider.action.label(), resp.weekly_count
    );
    if let Err(e) = msg.channel_id.say(&ctx.http, content).await {
        warn!(error = %e, "Echec annonce recompense bump");
    }
    info!(guild_id, user = %bumper_id, provider = provider.key, reward = resp.reward, "Bump recompense");
}

#[derive(serde::Deserialize)]
struct DueReminder {
    guild_id: String,
    channel_id: String,
    #[serde(default = "default_provider")]
    provider: String,
}

fn default_provider() -> String {
    "disboard".to_string()
}

/// Tache de fond : poste un rappel quand le cooldown de bump est ecoule.
///
/// Idempotent : `ready()` refire a chaque reconnexion Discord, mais on ne veut
/// qu'UNE boucle de rappel par process (sinon chaque boucle poste le meme
/// rappel -> doublons/triplons).
pub fn spawn_background(ctx: Context) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
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
                let provider = provider_by_key(&d.provider).unwrap_or(&DISBOARD);
                if let Ok(cid) = d.channel_id.parse::<u64>() {
                    let text = if provider.action == BumpAction::Vote {
                        format!(
                            "⏰ Tu peux **voter** à nouveau pour le serveur sur **{}** ! Faites `{}` — et gagnez des coins.",
                            provider.display, provider.bump_hint
                        )
                    } else {
                        format!(
                            "⏰ Le serveur peut être **bumpé** à nouveau sur **{}** ! Faites `{}` pour le faire remonter — et gagner des coins.",
                            provider.display, provider.bump_hint
                        )
                    };
                    let _ = ChannelId::new(cid).say(&ctx.http, text).await;
                }
                // Best-effort : on marque envoye meme si le post echoue, pour ne
                // pas spammer le rappel a chaque tick.
                let _ = api
                    .post_json::<_, serde_json::Value>(
                        &format!("/api/bump/{}/reminder-sent", d.guild_id),
                        &serde_json::json!({ "provider": d.provider }),
                    )
                    .await;
            }
        }
    });
}
