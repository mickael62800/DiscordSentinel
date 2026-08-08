//! Handler du bouton "Ouvrir une discussion" : creation d'un salon prive
//! (membre concerne + moderateurs) avec message d'ancrage epingle.

use serenity::prelude::*;
use tracing::{info, warn};

use crate::shared::api_client::BaseApiClient;
use crate::shared::grpc_client::GrpcClientKey;
use crate::shared::heartbeat::ApiClientKey;

use super::super::api_client::{ApiClient, ReviewFacts};
use super::cards::edit_ephemeral;
use super::context::{fetch_context_after_ids, fetch_context_before_ids, render_incident_list};
use super::labels::action_label;
use super::DISCUSSION_PREFIX;

/// Handler du bouton "Ouvrir une discussion" (`amdisc:<review_id>`).
/// Cree un salon textuel prive (membre concerne + role modo) sous la categorie
/// configuree, avec un message de contexte epingle ("ancrage").
pub(crate) async fn handle_discussion_button(
    ctx: &Context,
    component: &serenity::model::application::ComponentInteraction,
) {
    use serenity::all::{ChannelId, Permissions, RoleId, UserId};
    use serenity::model::channel::{PermissionOverwrite, PermissionOverwriteType};

    let review_id = match component.data.custom_id.strip_prefix(DISCUSSION_PREFIX) {
        Some(r) => r.to_string(),
        None => return,
    };
    let guild_id = match component.guild_id {
        Some(g) => g,
        None => return,
    };

    // Defer ephemere : la creation de salon peut depasser les 3s d'ack.
    if component
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::Defer(
                serenity::builder::CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
        .is_err()
    {
        return;
    }

    let (api, grpc) = {
        let d = ctx.data.read().await;
        match (d.get::<ApiClientKey>(), d.get::<GrpcClientKey>()) {
            (Some(a), Some(g)) => (a.clone(), g.clone()),
            _ => return,
        }
    };
    let review_api = ApiClient::new(grpc);
    let config = api
        .get_guild_config_for(&guild_id.to_string(), super::super::MODULE_BOT_NAME)
        .await
        .unwrap_or_default();

    if !BaseApiClient::config_bool(&config, "discussion_channel_enabled", false) {
        edit_ephemeral(
            ctx,
            component,
            "La creation de salon de discussion est desactivee.",
        )
        .await;
        return;
    }
    let mod_role_id = config
        .get("vote_mod_role_id")
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|x| *x > 0);

    // La REGLE d'acces est appliquee cote core (full hexa) ; le bot relaie.
    // Idempotence : un salon existe deja ? On s'y refere sans rien creer.
    // MAIS si le salon Discord a ete supprime a la main, l'enregistrement est
    // orphelin -> on le purge cote API pour pouvoir en regenerer un neuf.
    if let Ok(Some(existing)) = review_api.get_discussion(&review_id).await {
        let still_exists = match existing.channel_id.parse::<u64>() {
            Ok(cid) => ChannelId::new(cid).to_channel(&ctx.http).await.is_ok(),
            Err(_) => false,
        };
        if still_exists {
            edit_ephemeral(
                ctx,
                component,
                &format!(
                    "Un salon de discussion existe deja : <#{}>",
                    existing.channel_id
                ),
            )
            .await;
            return;
        }
        // Salon disparu : on purge l'enregistrement orphelin puis on recree.
        if let Err(e) = review_api.delete_discussion(&review_id).await {
            warn!(error = %e, review_id, "Echec purge discussion orpheline -> recreation annulee");
            edit_ephemeral(
                ctx,
                component,
                "Impossible de regenerer le salon (purge de l'ancien echouee).",
            )
            .await;
            return;
        }
        info!(review_id, old_channel = %existing.channel_id, "Salon de discussion disparu : enregistrement purge, recreation");
    }

    // Recupere la review (cible + contexte + incidents agreges).
    let review = match review_api.get_review(&review_id).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, review_id, "Echec fetch review (discussion)");
            edit_ephemeral(ctx, component, "Review introuvable.").await;
            return;
        }
    };
    let target_uid = match review.user_id.parse::<u64>() {
        Ok(v) => UserId::new(v),
        Err(_) => return,
    };

    // Overwrites : @everyone deny view ; cible + role modo + bot allow.
    let participate =
        Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY;
    let mut overwrites = vec![
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
            kind: PermissionOverwriteType::Role(RoleId::new(guild_id.get())),
        },
        PermissionOverwrite {
            // On n'accorde QUE `participate` : accorder MANAGE_MESSAGES ici ferait
            // echouer toute la creation si le bot n'a pas exactement cette perm au
            // niveau serveur (Discord interdit d'accorder une perm qu'on n'a pas).
            // Le pin du message d'ancrage utilise les perms serveur du bot.
            allow: participate,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(ctx.cache.current_user().id),
        },
        // Le moderateur qui ouvre la discussion a toujours acces, meme si aucun
        // role modo n'est configure (sinon le salon lui serait invisible).
        PermissionOverwrite {
            allow: participate,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(component.user.id),
        },
    ];
    if let Some(role) = mod_role_id {
        overwrites.push(PermissionOverwrite {
            allow: participate,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Role(RoleId::new(role)),
        });
    }

    // Nom de salon : "discussion-pseudo" assaini (alnum + tirets, sans doublons
    // de tirets ni tirets en bord ; repli sur l'id si le pseudo donne un nom vide).
    let mapped: String = format!("discussion-{}", review.user_name)
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let mut collapsed = mapped;
    while collapsed.contains("--") {
        collapsed = collapsed.replace("--", "-");
    }
    let trimmed = collapsed.trim_matches('-').to_string();
    let name: String = if trimmed.is_empty() {
        format!("discussion-{}", review.user_id)
    } else {
        trimmed.chars().take(95).collect()
    };

    let cat_id = config
        .get("discussion_category_id")
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|x| *x > 0);

    let build = |with_cat: bool| {
        let mut b = serenity::builder::CreateChannel::new(name.clone())
            .kind(serenity::model::channel::ChannelType::Text)
            .permissions(overwrites.clone());
        if with_cat {
            if let Some(c) = cat_id {
                b = b.category(ChannelId::new(c));
            }
        }
        b
    };

    // Cree le salon ; si echec AVEC categorie, on retente SANS (cause frequente :
    // categorie invalide/pleine). L'erreur Discord reelle est remontee a l'admin.
    let channel = match guild_id.create_channel(&ctx.http, build(true)).await {
        Ok(c) => c,
        Err(e1) if cat_id.is_some() => {
            warn!(error = %e1, "Echec creation salon discussion (avec categorie) -- retry sans categorie");
            match guild_id.create_channel(&ctx.http, build(false)).await {
                Ok(c) => c,
                Err(e2) => {
                    warn!(error = %e2, "Echec creation salon discussion (sans categorie)");
                    edit_ephemeral(ctx, component, &format!("Echec creation du salon : {e2}"))
                        .await;
                    return;
                }
            }
        }
        Err(e) => {
            warn!(error = %e, "Echec creation salon discussion");
            edit_ephemeral(ctx, component, &format!("Echec creation du salon : {e}")).await;
            return;
        }
    };

    // Donne l'acces au membre concerne APRES creation (best-effort) : s'il a
    // quitte/est banni, l'overwrite peut echouer sans bloquer la creation.
    if let Err(e) = channel
        .id
        .create_permission(
            &ctx.http,
            PermissionOverwrite {
                allow: participate,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Member(target_uid),
            },
        )
        .await
    {
        warn!(error = %e, user = %target_uid, "guild discussion: acces membre cible non accorde (a-t-il quitte ?)");
    }

    // Enregistre le salon cote API : le domaine applique la regle d'acces
    // (can_open_discussion) sur les faits Discord du demandeur + idempotence.
    let perms = component.member.as_ref().and_then(|m| m.permissions);
    let has = |p: Permissions| perms.map(|x| x.contains(p)).unwrap_or(false);
    let has_mod_role = match (mod_role_id, component.member.as_ref()) {
        (Some(role), Some(m)) => m.roles.iter().any(|r| r.get() == role),
        _ => false,
    };
    let facts = ReviewFacts {
        is_admin: has(Permissions::ADMINISTRATOR),
        has_moderate_members: has(Permissions::MODERATE_MEMBERS),
        has_manage_messages: has(Permissions::MANAGE_MESSAGES),
        has_mod_role,
        has_admin_role: false,
    };
    let opened = match review_api
        .open_discussion(
            &review_id,
            &guild_id.to_string(),
            &channel.id.to_string(),
            &component.user.id.to_string(),
            &component.user.name,
            facts,
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            // 403 (non autorise) ou autre erreur : on annule le salon cree.
            warn!(error = %e, review_id, "Refus/echec enregistrement discussion -> suppression du salon");
            let _ = channel.id.delete(&ctx.http).await;
            edit_ephemeral(
                ctx,
                component,
                "Discussion non autorisee ou erreur : salon annule.",
            )
            .await;
            return;
        }
    };
    if !opened.created {
        // Course : un salon a ete enregistre entre-temps -> on annule le notre.
        let _ = channel.id.delete(&ctx.http).await;
        edit_ephemeral(
            ctx,
            component,
            &format!(
                "Un salon de discussion existe deja : <#{}>",
                opened.channel_id
            ),
        )
        .await;
        return;
    }

    // Message d'ancrage epingle (contexte de la moderation).
    let action = if review.suggested_action.is_empty() {
        "warn"
    } else {
        review.suggested_action.as_str()
    };
    let origin_url = format!(
        "https://discord.com/channels/{}/{}/{}",
        review.guild_id, review.channel_id, review.message_id
    );
    let mut anchor = serenity::builder::CreateEmbed::new()
        .title("Discussion de moderation")
        .color(0x5865f2)
        .field(
            "Membre",
            format!("<@{}> (`{}`)", review.user_id, review.user_name),
            true,
        )
        .field("Action envisagee", action_label(action), true)
        .field("Score", format!("{:.2}", review.score), true)
        .field(
            "Raison",
            if review.reason.is_empty() {
                "—"
            } else {
                review.reason.as_str()
            },
            false,
        );

    // Liste des infractions (incidents agreges). Affiche le contenu + la raison
    // de chacune, numerotees, pour que le membre voie precisement ce qui pose
    // probleme. Tronque pour rester sous la limite d'un field (1024).
    let infractions = render_incident_list(&review.incidents, review.incident_count);
    if !infractions.is_empty() {
        anchor = anchor.field(
            format!("Infractions ({})", review.incident_count.max(1)),
            infractions,
            false,
        );
    }

    // Contexte conversationnel : X messages AVANT + Y messages APRES la
    // derniere infraction, pour que le membre comprenne le fil.
    let ctx_before = BaseApiClient::config_u64(&config, "vote_context_before", 10) as u8;
    let ctx_after = BaseApiClient::config_u64(&config, "vote_context_after", 10) as u8;
    if let (Ok(cid), Ok(mid)) = (
        review.channel_id.parse::<u64>(),
        review.message_id.parse::<u64>(),
    ) {
        let chan = ChannelId::new(cid);
        let msgid = serenity::model::id::MessageId::new(mid);
        let before = fetch_context_before_ids(ctx, chan, msgid, ctx_before).await;
        if !before.is_empty() {
            anchor = anchor.field("Contexte (messages precedents)", before, false);
        }
        let after = fetch_context_after_ids(ctx, chan, msgid, ctx_after).await;
        if !after.is_empty() {
            anchor = anchor.field("Messages suivants", after, false);
        }
    }

    anchor = anchor
        .field(
            "Message d'origine",
            format!("[Aller au message]({origin_url})"),
            false,
        )
        .footer(serenity::builder::CreateEmbedFooter::new(
            "Salon ouvert pour echanger avant decision.",
        ))
        .timestamp(serenity::model::Timestamp::now());
    let ping = match mod_role_id {
        Some(role) => format!("<@{}> <@&{}>", review.user_id, role),
        None => format!("<@{}>", review.user_id),
    };
    if let Ok(posted) = channel
        .id
        .send_message(
            &ctx.http,
            serenity::builder::CreateMessage::new()
                .content(ping)
                .embed(anchor),
        )
        .await
    {
        // "Ancrage" = epinglage du message de contexte en haut du salon.
        let _ = channel.id.pin(&ctx.http, posted.id).await;
    }

    edit_ephemeral(
        ctx,
        component,
        &format!("Salon de discussion cree : <#{}>", channel.id),
    )
    .await;
    info!(review_id, channel = %channel.id, "Salon de discussion cree");
}
