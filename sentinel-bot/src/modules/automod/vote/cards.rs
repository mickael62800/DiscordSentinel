//! Edition/archivage des cartes : carte agregee, transcript de discussion,
//! reponses ephemeres.

use std::sync::Arc;

use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;

use super::context::fetch_context_before_ids;
use super::render::{aggregated_vote_embed, render_history_totals, ReviewResp, VoteDto};

/// Archive le salon de discussion lie a une review finalisee : renomme en
/// "clos-…" et retire le droit d'ecrire au membre concerne (les moderateurs
/// gardent l'acces en lecture pour la trace). No-op si aucun salon.
pub(crate) async fn archive_discussion_channel(
    ctx: &Context,
    api: &Arc<BaseApiClient>,
    review_id: &str,
    _target_user_id: &str,
) {
    use serenity::all::ChannelId;

    #[derive(serde::Deserialize)]
    struct DiscussionResp {
        channel_id: String,
    }
    let Ok(Some(disc)) = api
        .get_json::<Option<DiscussionResp>>(&format!("/api/automod/reviews/{review_id}/discussion"))
        .await
    else {
        return;
    };
    let Ok(cid) = disc.channel_id.parse::<u64>() else {
        return;
    };
    let channel = ChannelId::new(cid);

    // Snapshot de la conversation -> DB (trace consultable sur le web) AVANT de
    // supprimer le salon. La trace reste consultable sur le web ensuite.
    snapshot_discussion_messages(ctx, api, review_id, channel).await;

    // Suppression du salon : l'affaire est close, la conversation est archivee.
    if let Err(e) = channel.delete(&ctx.http).await {
        warn!(error = %e, review_id, channel = %channel, "Echec suppression salon de discussion (archive)");
    } else {
        info!(review_id, channel = %channel, "Salon de discussion supprime (trace sauvee en DB)");
    }
}

/// Capture les messages du salon de discussion et les persiste cote API
/// (transcript). Best-effort : recupere jusqu'a 100 messages (ordre chrono),
/// puis POST en batch (idempotent cote serveur sur (review, message_id)).
async fn snapshot_discussion_messages(
    ctx: &Context,
    api: &Arc<BaseApiClient>,
    review_id: &str,
    channel: serenity::all::ChannelId,
) {
    let msgs = match channel
        .messages(&ctx.http, serenity::builder::GetMessages::new().limit(100))
        .await
    {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, review_id, "Echec recuperation messages discussion (snapshot)");
            return;
        }
    };
    if msgs.is_empty() {
        return;
    }
    // L'API renvoie du plus recent au plus ancien -> ordre chronologique.
    let messages: Vec<serde_json::Value> = msgs
        .iter()
        .rev()
        .map(|m| {
            serde_json::json!({
                "discord_message_id": m.id.to_string(),
                "author_id": m.author.id.to_string(),
                "author_name": m.author.name,
                "author_is_bot": m.author.bot,
                "content": m.content,
                "sent_at": m.timestamp.to_string(),
            })
        })
        .collect();
    let body = serde_json::json!({ "messages": messages });
    if let Err(e) = api
        .post_json::<_, serde_json::Value>(
            &format!("/api/automod/reviews/{review_id}/discussion/messages"),
            &body,
        )
        .await
    {
        warn!(error = %e, review_id, "Echec persistance transcript discussion");
    }
}

/// Edite la carte existante d'une review agregee : recharge le mapping Discord
/// + les votes, puis remplace l'embed (les boutons de vote sont conserves).
///
/// Retourne `true` si la carte a ete editee (ou si l'echec est transitoire et
/// ne justifie pas de recreer la carte), `false` si le message de la carte a
/// disparu (supprime cote Discord) — dans ce cas l'appelant doit reposter une
/// carte neuve (le mapping `discord_action_messages` etant upserte, le nouveau
/// message_id remplace l'ancien).
pub(super) async fn edit_aggregated_card(
    ctx: &Context,
    api: &Arc<BaseApiClient>,
    resp: &ReviewResp,
) -> bool {
    use serenity::all::{ChannelId, MessageId};

    #[derive(serde::Deserialize)]
    struct Mapping {
        kind: String,
        channel_id: String,
        message_id: String,
    }
    let mappings: Vec<Mapping> = match api
        .get_json(&format!("/api/discord-messages/{}", resp.id))
        .await
    {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, review_id = %resp.id, "Echec fetch mapping (agregation)");
            return true;
        }
    };
    // Pas de mapping connu -> on ne peut pas editer : il faut (re)creer la carte.
    let Some(mapping) = mappings.into_iter().find(|m| m.kind == "automod_review") else {
        return false;
    };
    let (Ok(cid), Ok(mid)) = (
        mapping.channel_id.parse::<u64>(),
        mapping.message_id.parse::<u64>(),
    ) else {
        return false;
    };
    let channel_id = ChannelId::new(cid);
    let msg_id = MessageId::new(mid);

    // Recharge les votes pour les conserver dans l'embed reconstruit.
    let votes: Vec<VoteDto> = api
        .get_json(&format!("/api/automod/reviews/{}/votes", resp.id))
        .await
        .unwrap_or_default();

    let mut embed = aggregated_vote_embed(resp, &votes);
    // Re-injecte le CONTEXTE autour du DERNIER message agrege (sinon il
    // disparaitrait a chaque fusion : aggregated_vote_embed ne le porte pas).
    let context_before = {
        let cfg = api
            .get_guild_config_for(&resp.guild_id, super::super::MODULE_BOT_NAME)
            .await
            .unwrap_or_default();
        BaseApiClient::config_u64(&cfg, "vote_context_before", 10) as u8
    };
    if let (Ok(lcid), Ok(lmid)) = (
        resp.channel_id.parse::<u64>(),
        resp.message_id.parse::<u64>(),
    ) {
        let context = fetch_context_before_ids(
            ctx,
            ChannelId::new(lcid),
            MessageId::new(lmid),
            context_before,
        )
        .await;
        if !context.is_empty() {
            embed = embed.field("Contexte (messages precedents)", context, false);
        }
    }
    if let Some(hist) = render_history_totals(ctx, &resp.guild_id, &resp.user_id).await {
        embed = embed.field("📋 Antecedents du membre", hist, false);
    }
    // On n'edite que l'embed : les composants (boutons de vote + lien) restent.
    match channel_id
        .edit_message(
            &ctx.http,
            msg_id,
            serenity::builder::EditMessage::new().embed(embed),
        )
        .await
    {
        Ok(_) => {
            info!(review_id = %resp.id, incidents = resp.incident_count, "Carte agregee mise a jour");
            true
        }
        Err(e) => {
            // Message (ou salon) supprime cote Discord -> il faut recreer la carte.
            // Pour les autres erreurs (rate limit, reseau...) on NE recree PAS,
            // afin d'eviter les doublons : on retourne true (rien a recreer).
            let s = e.to_string();
            if s.contains("Unknown Message")
                || s.contains("Unknown Channel")
                || s.contains("10008")
                || s.contains("10003")
            {
                warn!(review_id = %resp.id, "Carte agregee introuvable (message supprime) -> recreation");
                false
            } else {
                warn!(error = %e, review_id = %resp.id, "Echec edition carte agregee (erreur transitoire, pas de recreation)");
                true
            }
        }
    }
}

/// Edite une reponse deja deferee (ephemere). A utiliser apres un Defer.
pub(super) async fn edit_ephemeral(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
    text: &str,
) {
    let _ = component
        .edit_response(
            &ctx.http,
            serenity::builder::EditInteractionResponse::new().content(text),
        )
        .await;
}

pub(super) async fn reply_ephemeral(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
    text: &str,
) {
    let _ = component
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::Message(
                serenity::builder::CreateInteractionResponseMessage::new()
                    .content(text)
                    .ephemeral(true),
            ),
        )
        .await;
}
