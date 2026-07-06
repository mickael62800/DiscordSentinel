use serenity::builder::CreateChannel;
use serenity::model::application::ComponentInteraction;
use serenity::model::channel::ChannelType;
use serenity::model::id::{ChannelId, UserId};
use serenity::model::Permissions;
use serenity::prelude::*;
use tracing::{error, info, warn};

use super::api_client::{ApiClient, UpdateVoiceChannelRequest};

/// Handle queue interactions: toggle queue, accept/refuse.
pub async fn handle(ctx: &Context, component: &ComponentInteraction) {
    let custom_id = component.data.custom_id.as_str();

    match custom_id {
        "btn_queue" => handle_toggle_queue(ctx, component).await,
        other if other.starts_with("queue_accept_") => handle_queue_accept(ctx, component).await,
        other if other.starts_with("queue_refuse_") => handle_queue_refuse(ctx, component).await,
        _ => {
            warn!(custom_id = %custom_id, "Queue interaction inconnue");
        }
    }
}

// ── Toggle Queue ──

async fn handle_toggle_queue(ctx: &Context, component: &ComponentInteraction) {
    super::defer_ephemeral(ctx, component).await;
    let Some((voice_channel_id, ch)) = super::require_admin_deferred(ctx, component).await else {
        return;
    };

    let guild_id = component.guild_id.unwrap_or_default();
    let queue_enabled = ch.queue_enabled;

    if queue_enabled {
        // Disable queue: delete the queue voice channel
        if let Some(ref queue_id_str) = ch.queue_channel_id {
            if let Ok(queue_id) = queue_id_str.parse::<u64>() {
                if let Err(e) = ChannelId::new(queue_id).delete(&ctx.http).await {
                    tracing::warn!(error = %e, "failed to delete queue channel");
                }
            }
        }

        // Retirer le deny CONNECT sur @everyone du vocal principal : maintenant
        // que la queue est desactivee, tout le monde peut rejoindre a nouveau.
        let everyone_role = serenity::model::id::RoleId::new(guild_id.get());
        // On delete l'override @everyone pour revenir a l'etat par defaut.
        // Si le salon etait "hidden" (VIEW_CHANNEL deny), on recree l'override
        // uniquement avec VIEW_CHANNEL deny.
        if let Err(e) = voice_channel_id
            .delete_permission(
                &ctx.http,
                serenity::model::channel::PermissionOverwriteType::Role(everyone_role),
            )
            .await
        {
            tracing::warn!(error = %e, "failed to remove @everyone deny CONNECT on voice when disabling queue");
        }
        // Restaurer le hidden state si le salon etait cache
        if ch.visibility == "hidden" {
            let overwrite = serenity::model::channel::PermissionOverwrite {
                allow: Permissions::empty(),
                deny: Permissions::VIEW_CHANNEL,
                kind: serenity::model::channel::PermissionOverwriteType::Role(everyone_role),
            };
            if let Err(e) = voice_channel_id
                .create_permission(&ctx.http, overwrite)
                .await
            {
                tracing::warn!(error = %e, "failed to restore hidden state after disabling queue");
            }
        }

        // Update API
        let update = UpdateVoiceChannelRequest {
            visibility: None,
            locked: None,
            queue_enabled: Some(false),
            name: None,
            status: None,
            member_limit: None,
            queue_channel_id: Some(None),
        };

        {
            let data = ctx.data.read().await;
            let Some(api) = ApiClient::from_data(&data) else {
                error!("ApiClient ou GrpcClient manquants dans TypeMap");
                return;
            };
            if let Err(e) = api
                .update_channel(&voice_channel_id.get().to_string(), &update)
                .await
            {
                error!(error = %e, "Erreur API disable queue");
            }
        }

        super::respond_followup_ephemeral(
            ctx,
            component,
            "La file d'attente a ete **desactivee**.",
        )
        .await;
        info!(voice = %voice_channel_id, "File d'attente desactivee");
    } else {
        // Enable queue: create the queue voice channel dans la MEME categorie
        // que le vocal principal. On lit le parent_id REEL du salon (cache)
        // plutot que ch.category_id (persiste a None en DB cote bot), sinon la
        // file se cree a la racine du serveur (tout en haut) au lieu d'etre
        // ancree sous la categorie selectionnee.
        let category_id = ctx
            .cache
            .guild(guild_id)
            .and_then(|g| g.channels.get(&voice_channel_id).and_then(|c| c.parent_id))
            .map(|p| p.get())
            .or_else(|| ch.category_id.as_ref().and_then(|s| s.parse::<u64>().ok()));

        let queue_name = format!("File d'attente - {}", ch.channel_name);
        // Limite de membres de la file : reglable par serveur (defaut/cap 99).
        let queue_limit = super::max_user_limit(ctx, Some(guild_id)).await as u32;
        let mut queue_builder = CreateChannel::new(&queue_name)
            .kind(ChannelType::Voice)
            .user_limit(queue_limit);

        if let Some(cat_id) = category_id {
            queue_builder = queue_builder.category(ChannelId::new(cat_id));
        }

        let queue_channel = match guild_id.create_channel(&ctx.http, queue_builder).await {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "Erreur creation queue channel");
                super::respond_followup_ephemeral(
                    ctx,
                    component,
                    "Erreur lors de la creation de la file d'attente.",
                )
                .await;
                return;
            }
        };

        let queue_channel_id = queue_channel.id;

        // Permissions on queue: everyone can join but not speak
        let everyone_role = serenity::model::id::RoleId::new(guild_id.get());
        let queue_overwrite = serenity::model::channel::PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL | Permissions::CONNECT,
            deny: Permissions::SPEAK,
            kind: serenity::model::channel::PermissionOverwriteType::Role(everyone_role),
        };
        if let Err(e) = queue_channel_id
            .create_permission(&ctx.http, queue_overwrite)
            .await
        {
            tracing::warn!(error = %e, "failed to set queue channel permissions");
        }

        // FIX QUEUE BYPASS : deny CONNECT sur le vocal principal pour @everyone.
        // Sans ca, les users non-invites peuvent cliquer directement sur le
        // vocal principal et entrer sans passer par la file d'attente.
        // Les user overrides (ALLOW CONNECT pour l'owner + users acceptes)
        // priment sur le role deny donc ils peuvent toujours rejoindre.
        // On preserve le VIEW_CHANNEL deny si le salon etait hidden.
        let voice_deny = if ch.visibility == "hidden" {
            Permissions::VIEW_CHANNEL | Permissions::CONNECT
        } else {
            Permissions::CONNECT
        };
        let voice_overwrite = serenity::model::channel::PermissionOverwrite {
            allow: Permissions::empty(),
            deny: voice_deny,
            kind: serenity::model::channel::PermissionOverwriteType::Role(everyone_role),
        };
        if let Err(e) = voice_channel_id
            .create_permission(&ctx.http, voice_overwrite)
            .await
        {
            tracing::warn!(error = %e, "failed to lock voice channel behind queue");
        }

        // Placer la file d'attente au-dessus du vocal pour la rendre visible
        super::handlers::voice::channel_lifecycle::place_queue_above_voice(
            ctx,
            guild_id,
            queue_channel_id,
            voice_channel_id,
        )
        .await;

        // Update API
        let update = UpdateVoiceChannelRequest {
            visibility: None,
            locked: None,
            queue_enabled: Some(true),
            name: None,
            status: None,
            member_limit: None,
            queue_channel_id: Some(Some(queue_channel_id.get().to_string())),
        };

        {
            let data = ctx.data.read().await;
            let Some(api) = ApiClient::from_data(&data) else {
                error!("ApiClient ou GrpcClient manquants dans TypeMap");
                return;
            };
            if let Err(e) = api
                .update_channel(&voice_channel_id.get().to_string(), &update)
                .await
            {
                error!(error = %e, "Erreur API enable queue");
            }
        }

        super::respond_followup_ephemeral(ctx, component, "La file d'attente a ete **activee**.")
            .await;
        info!(voice = %voice_channel_id, queue = %queue_channel_id, "File d'attente activee");
    }
}

// ── Accept from Queue ──

async fn handle_queue_accept(ctx: &Context, component: &ComponentInteraction) {
    // Defense en profondeur : meme si le salon admin-panel n'est visible que
    // par l'owner/co-admin, on re-verifie l'ownership via l'API.
    let Some((voice_channel_id, ch)) = super::require_admin(ctx, component).await else {
        return;
    };

    let custom_id = component.data.custom_id.as_str();
    let target_id_str = custom_id.strip_prefix("queue_accept_").unwrap_or("");
    let target_id: u64 = match target_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            super::respond_ephemeral(ctx, component, "ID utilisateur invalide.").await;
            return;
        }
    };

    let target_user_id = UserId::new(target_id);

    let guild_id = component.guild_id.unwrap_or_default();

    // La cible doit REELLEMENT etre dans la file d'attente (queue_channel_id) ->
    // empeche un custom_id forge de deplacer de force un membre d'un autre salon
    // du serveur vers le salon de l'owner.
    let in_queue = ch
        .queue_channel_id
        .as_ref()
        .and_then(|q| q.parse::<u64>().ok())
        .map(serenity::model::id::ChannelId::new)
        .map(|queue_ch| {
            ctx.cache
                .guild(guild_id)
                .map(|g| {
                    g.voice_states
                        .get(&target_user_id)
                        .and_then(|vs| vs.channel_id)
                        == Some(queue_ch)
                })
                .unwrap_or(false)
        })
        .unwrap_or(false);
    if !in_queue {
        super::respond_ephemeral(ctx, component, "Ce membre n'est pas dans la file d'attente.")
            .await;
        return;
    }

    // Move the user from the queue channel to the voice channel
    let edit = serenity::builder::EditMember::new().voice_channel(voice_channel_id);
    match guild_id.edit_member(&ctx.http, target_user_id, edit).await {
        Ok(_) => {
            info!(
                voice = %voice_channel_id,
                user = %target_user_id,
                "Utilisateur accepte depuis la file"
            );
        }
        Err(e) => {
            warn!(error = %e, "Erreur deplacement depuis la file");
            super::respond_ephemeral(
                ctx,
                component,
                "Erreur : l'utilisateur n'est peut-etre plus dans la file d'attente.",
            )
            .await;
            return;
        }
    }

    // Grant permissions on voice channel
    let overwrite = serenity::model::channel::PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL | Permissions::CONNECT | Permissions::SPEAK,
        deny: Permissions::empty(),
        kind: serenity::model::channel::PermissionOverwriteType::Member(target_user_id),
    };
    if let Err(e) = voice_channel_id
        .create_permission(&ctx.http, overwrite)
        .await
    {
        tracing::warn!(error = %e, "failed to grant accepted user permission on voice channel");
    }

    super::respond_ephemeral(
        ctx,
        component,
        &format!("<@{target_id}> a ete accepte dans le salon."),
    )
    .await;
}

// ── Refuse from Queue ──

async fn handle_queue_refuse(ctx: &Context, component: &ComponentInteraction) {
    // Defense en profondeur : re-verifie l'ownership via l'API.
    if super::require_admin(ctx, component).await.is_none() {
        return;
    }

    let custom_id = component.data.custom_id.as_str();
    let target_id_str = custom_id.strip_prefix("queue_refuse_").unwrap_or("");
    let target_id: u64 = match target_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            super::respond_ephemeral(ctx, component, "ID utilisateur invalide.").await;
            return;
        }
    };

    let target_user_id = UserId::new(target_id);
    let guild_id = component.guild_id.unwrap_or_default();

    // Disconnect the user from the queue voice channel
    match guild_id.disconnect_member(&ctx.http, target_user_id).await {
        Ok(_) => {
            info!(user = %target_user_id, "Utilisateur refuse de la file");
        }
        Err(e) => {
            warn!(error = %e, "Erreur disconnect depuis la file");
            super::respond_ephemeral(
                ctx,
                component,
                "Erreur : l'utilisateur n'est peut-etre plus dans la file d'attente.",
            )
            .await;
            return;
        }
    }

    // Notifier l'utilisateur refuse par DM
    if let Ok(user) = target_user_id.to_user(&ctx.http).await {
        if let Ok(dm) = user.create_dm_channel(&ctx.http).await {
            let embed = serenity::builder::CreateEmbed::new()
                .title("\u{274c} Acces refuse")
                .description("Votre demande d'acces au salon vocal a ete refusee.")
                .color(0xED4245)
                .timestamp(serenity::model::Timestamp::now());

            if let Err(e) = dm
                .send_message(
                    &ctx.http,
                    serenity::builder::CreateMessage::new().embed(embed),
                )
                .await
            {
                tracing::warn!(error = %e, "failed to send queue refusal DM");
            }
        }
    }

    super::respond_ephemeral(
        ctx,
        component,
        &format!("<@{target_id}> a ete refuse et notifie."),
    )
    .await;
}
