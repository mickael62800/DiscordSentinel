use serenity::all::ButtonStyle;
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed, CreateMessage};
use serenity::model::application::{ComponentInteraction, ComponentInteractionDataKind};
use serenity::prelude::*;

use crate::handler::{VoiceOwnerMapKey, VoteTrackerKey};
use crate::handlers::voice;

use super::{find_voice_from_members, respond_ephemeral};

pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = component.data.custom_id.as_str();
    match custom_id {
        "select_votekick" => handle_votekick_select(ctx, component).await,
        "votekick_yes" => handle_votekick_cast(ctx, component, true).await,
        "votekick_no" => handle_votekick_cast(ctx, component, false).await,
        _ => {}
    }
}

async fn handle_votekick_select(ctx: &Context, component: &ComponentInteraction) {
    let members_channel_id = component.channel_id;
    let voter = component.user.id;

    let voice_channel_id = match find_voice_from_members(ctx, members_channel_id).await {
        Some(id) => id,
        None => {
            respond_ephemeral(ctx, component, "Salon introuvable.").await;
            return;
        }
    };

    // Verifier le type du salon via l'API
    let channel_kind = {
        let data = ctx.data.read().await;
        if let Some(api) = crate::api_client::ApiClient::from_data(&data) {
            api.get_channel(&voice_channel_id.get().to_string())
                .await
                .ok()
                .flatten()
                .map(|ch| ch.kind)
                .unwrap_or_else(|| "public".to_string())
        } else {
            "public".to_string()
        }
    };

    let is_private = channel_kind == "private";

    // Pour les salons prives : verifier qu'il n'y a pas d'admin present
    // Pour les salons publics : pas de check admin, tout le monde peut etre vote kick
    let owner = if is_private {
        let data = ctx.data.read().await;
        data.get::<VoiceOwnerMapKey>()
            .and_then(|map| map.get(&voice_channel_id))
            .map(|e| *e)
    } else {
        None
    };

    if is_private {
        if let Some(owner_id) = owner {
            let admin_present = component.guild_id
                .and_then(|gid| ctx.cache.guild(gid))
                .map(|guild| {
                    guild.voice_states.values().any(|vs| {
                        vs.channel_id == Some(voice_channel_id) && vs.user_id == owner_id
                    })
                })
                .unwrap_or(false);

            if admin_present {
                respond_ephemeral(
                    ctx,
                    component,
                    "Un admin est present dans le vocal. Contacte-le directement.",
                )
                .await;
                return;
            }
        }
    }

    // Verifier pas de vote en cours
    let has_vote = {
        let data = ctx.data.read().await;
        data.get::<VoteTrackerKey>()
            .map(|vt| vt.has_active_vote(members_channel_id))
            .unwrap_or(false)
    };

    if has_vote {
        respond_ephemeral(ctx, component, "Un vote est deja en cours.").await;
        return;
    }

    // Recuperer la cible
    let target = match &component.data.kind {
        ComponentInteractionDataKind::UserSelect { values } => match values.first() {
            Some(id) => *id,
            None => {
                respond_ephemeral(ctx, component, "Aucun membre selectionne.").await;
                return;
            }
        },
        _ => {
            respond_ephemeral(ctx, component, "Erreur.").await;
            return;
        }
    };

    // Ne pas voter contre un admin (prive seulement : owner OU co-admin)
    if is_private && owner == Some(target) {
        respond_ephemeral(ctx, component, "Impossible de voter contre le proprietaire.").await;
        return;
    }
    // C5 — proteger aussi les co-admins : un user avec MANAGE_CHANNELS sur
    // le vocal a ete promu co-admin via add_co_admin (ou est l'owner).
    // On inspecte les permission overwrites du channel Discord plutot que
    // de re-fetch l'API (plus rapide + consistant avec l'etat reel).
    if is_private {
        let target_is_staff = component
            .guild_id
            .and_then(|gid| ctx.cache.guild(gid))
            .and_then(|g| g.channels.get(&voice_channel_id).cloned())
            .map(|ch| {
                ch.permission_overwrites.iter().any(|ov| {
                    matches!(
                        ov.kind,
                        serenity::model::channel::PermissionOverwriteType::Member(uid) if uid == target
                    ) && ov.allow.contains(serenity::model::Permissions::MANAGE_CHANNELS)
                })
            })
            .unwrap_or(false);

        if target_is_staff {
            respond_ephemeral(
                ctx,
                component,
                "Impossible de voter contre un co-admin du salon.",
            )
            .await;
            return;
        }
    }

    // Compter les membres dans le vocal
    let total_members = if let Some(guild) = component.guild_id.and_then(|gid| ctx.cache.guild(gid))
    {
        guild
            .voice_states
            .values()
            .filter(|vs| vs.channel_id == Some(voice_channel_id) && vs.user_id != target)
            .count()
    } else {
        0
    };

    if total_members < 2 {
        respond_ephemeral(
            ctx,
            component,
            "Il faut au moins 2 membres (hors la cible) pour lancer un vote.",
        )
        .await;
        return;
    }

    // Lancer le vote
    let started = {
        let data = ctx.data.read().await;
        data.get::<VoteTrackerKey>()
            .map(|vt| {
                vt.start_vote(
                    members_channel_id,
                    voice_channel_id,
                    target,
                    voter,
                    total_members,
                )
            })
            .unwrap_or(false)
    };

    if !started {
        respond_ephemeral(ctx, component, "Impossible de lancer le vote.").await;
        return;
    }

    let needed = (total_members / 2) + 1;
    let buttons = vec![
        CreateButton::new("votekick_yes")
            .label("Expulser")
            .style(ButtonStyle::Danger),
        CreateButton::new("votekick_no")
            .label("Garder")
            .style(ButtonStyle::Secondary),
    ];

    let embed = CreateEmbed::new()
        .title("Vote Kick")
        .description(format!(
            "<@{voter}> demande l'expulsion de <@{target}> !\n\n\
            **{needed}** votes necessaires sur **{total_members}** membres.\n\
            Pour : **1/{needed}** | Contre : **0**\n\n\
            Le vote expire dans 60 secondes."
        ))
        .color(0xff6b6b);

    let msg = CreateMessage::new()
        .embed(embed)
        .components(vec![CreateActionRow::Buttons(buttons)]);

    if let Err(e) = members_channel_id.send_message(&ctx.http, msg).await {
        tracing::warn!(error = %e, "failed to send vote kick message");
    }
    respond_ephemeral(ctx, component, "Vote lance !").await;

    // Timeout configurable (default 60s, lu depuis VoiceConfig).
    let vote_timeout = {
        let data = ctx.data.read().await;
        data.get::<crate::handler::VoiceConfigKey>()
            .map(|c| c.vote_kick_timeout_secs)
            .unwrap_or(60)
    };
    let ctx_clone = ctx.clone();
    let mc = members_channel_id;
    let vc = voice_channel_id;
    let guild_id = component.guild_id;
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(vote_timeout)).await;

        let vote = {
            let data = ctx_clone.data.read().await;
            data.get::<VoteTrackerKey>()
                .and_then(|vt| vt.end_vote(mc))
        };

        if let Some(vote) = vote {
            if vote.majority_reached() {
                if let Some(gid) = guild_id {
                    if let Err(e) = gid.disconnect_member(&ctx_clone.http, vote.target).await {
                        tracing::warn!(error = %e, "failed to disconnect vote-kicked member on timeout");
                    }
                    voice::revoke_members_panel_access(&ctx_clone, vc, vote.target).await;
                }
                if let Err(e) = mc
                    .say(
                        &ctx_clone.http,
                        format!(
                            "Vote termine : <@{}> a ete **expulse** !",
                            vote.target
                        ),
                    )
                    .await
                {
                    tracing::warn!(error = %e, "failed to send vote kick expulsion result");
                }
            } else if let Err(e) = mc
                .say(
                    &ctx_clone.http,
                    format!(
                        "Vote termine : <@{}> **reste** dans le salon.",
                        vote.target
                    ),
                )
                .await
            {
                tracing::warn!(error = %e, "failed to send vote kick rejection result");
            }
        }
    });
}

pub async fn handle_votekick_cast(
    ctx: &Context,
    component: &ComponentInteraction,
    vote_yes: bool,
) {
    let members_channel_id = component.channel_id;
    let voter = component.user.id;

    let vote = {
        let data = ctx.data.read().await;
        data.get::<VoteTrackerKey>()
            .and_then(|vt| vt.cast_vote(members_channel_id, voter, vote_yes))
    };

    let vote = match vote {
        Some(v) => v,
        None => {
            respond_ephemeral(ctx, component, "Aucun vote en cours.").await;
            return;
        }
    };

    let vote_text = if vote_yes { "pour" } else { "contre" };
    respond_ephemeral(ctx, component, &format!("Vote enregistre ({vote_text}).")).await;

    if vote.majority_reached() {
        // Kick immediat
        {
            let data = ctx.data.read().await;
            if let Some(vt) = data.get::<VoteTrackerKey>() {
                vt.end_vote(members_channel_id);
            }
        }

        if let Some(gid) = component.guild_id {
            if let Err(e) = gid.disconnect_member(&ctx.http, vote.target).await {
                tracing::warn!(error = %e, "failed to disconnect vote-kicked member");
            }
            voice::revoke_members_panel_access(ctx, vote.voice_channel_id, vote.target).await;
        }

        if let Err(e) = members_channel_id
            .say(
                &ctx.http,
                format!(
                    "Vote termine : <@{}> a ete **expulse** ! ({})",
                    vote.target,
                    vote.status_text()
                ),
            )
            .await
        {
            tracing::warn!(error = %e, "failed to send vote kick expulsion message");
        }
    } else if vote.rejected() {
        {
            let data = ctx.data.read().await;
            if let Some(vt) = data.get::<VoteTrackerKey>() {
                vt.end_vote(members_channel_id);
            }
        }

        if let Err(e) = members_channel_id
            .say(
                &ctx.http,
                format!(
                    "Vote termine : <@{}> **reste** dans le salon. ({})",
                    vote.target,
                    vote.status_text()
                ),
            )
            .await
        {
            tracing::warn!(error = %e, "failed to send vote kick rejection message");
        }
    }
}
