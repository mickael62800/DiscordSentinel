//! EventHandler du bot audit.
//!
//! Le fichier `mod.rs` ne contient que :
//! - la structure `Handler` et son `impl EventHandler` (dispatch des events
//!   Discord vers les sous-handlers `crate::handlers::*`),
//! - quelques helpers transverses (`is_guild_enabled`, `send_event`, `log`).
//!
//! Les autres responsabilités vivent dans :
//! - `type_keys` : toutes les `TypeMapKey` du bot
//! - `watched_users` : `Handler::is_watched` + `Handler::track_activity`

use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::Message;
use serenity::model::event::MessageUpdateEvent;
use serenity::model::gateway::Ready;
use serenity::model::guild::{Guild, Member, Role};
use serenity::model::id::{ChannelId, GuildId, MessageId, RoleId};
use serenity::model::user::User;
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use tracing::{error, info, warn};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::heartbeat::{register_guilds, ApiClientKey};

use crate::api_client::{ApiClient, AuditEvent};
use crate::commands;
use crate::handlers;

mod type_keys;
mod watched_users;

// Re-exports pour préserver `crate::handler::{WeeklyTrackerKey, ...}`
// consommés par les sous-handlers et les commandes.
pub use type_keys::{
    AnomalyDetectorKey, ConfigKey, MessageCacheKey, WatchedUserIdsKey, WeeklyTrackerKey,
};
use watched_users::{bootstrap_watched_users, handle_watched_refresh_event};

pub struct Handler;

impl Handler {
    /// Vérifie si le bot est activé pour une guild donnée (défaut : oui en cas d'erreur).
    async fn is_guild_enabled(ctx: &Context, guild_id: &str) -> bool {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let config = match base.get_guild_config(guild_id).await {
                Ok(c) => c,
                Err(_) => return true,
            };
            return BaseApiClient::config_bool(&config, "enabled", true);
        }
        true
    }

    /// Envoie un événement d'audit à l'API.
    pub async fn send_event(ctx: &Context, event: AuditEvent) {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let api = ApiClient::new(base.clone());
            if let Err(e) = api.send_audit_event(&event).await {
                warn!(error = %e, event_type = %event.event_type, "Erreur envoi audit event");
            }
        }
    }

    /// Pousse un log structuré dans la queue d'envoi de l'API client.
    pub async fn log(ctx: &Context, level: &str, guild_id: &str, message: &str) {
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            api.send_log(level, guild_id, message);
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, guilds = ready.guilds.len(), "Audit bot connecte");
        register_guilds(&ctx, &ready).await;

        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands::all(),
        )
        .await
        {
            error!(error = %e, "Erreur enregistrement commandes");
        } else {
            info!("Slash commands enregistrees : audit");
        }

        // Phase 6A — bootstrap + consumer stream Redis.
        //
        // Le refresh periodique est desormais delegue a `audit-cache-worker`
        // (permet le scaling horizontal d'audit-bot sans N appels API
        // dupliques). Le worker :
        //   1. query Postgres toutes les 60s
        //   2. push dans Redis cle `audit:watched_users` (TTL 5 min)
        //   3. publie un event `watched_users_refreshed` sur `sentinel:events`
        //
        // Le bot :
        //   - bootstrap son DashSet depuis Redis au startup (fallback API si
        //     Redis vide, par exemple si le worker n'a pas encore tourne)
        //   - consume le stream + refresh depuis Redis a chaque event
        let ctx_bootstrap = ctx.clone();
        tokio::spawn(async move {
            bootstrap_watched_users(&ctx_bootstrap).await;

            // Consumer durable Phase 5B — XREADGROUP + XACK. Le group
            // "audit-bot-watched-cache" est partage si multi-replicas, ce qui
            // fait qu'un seul replica re-fetch par event (les autres sont en
            // idle). Pattern Phase 5B identique a ticket-bot / moderation-bot.
            let consumer = sentinel_shared::event_bus::default_consumer_name();
            sentinel_shared::event_bus::listen_stream_group(
                "audit-bot-watched-cache".to_string(),
                consumer,
                move |payload_json| {
                    let ctx = ctx_bootstrap.clone();
                    async move {
                        handle_watched_refresh_event(&ctx, &payload_json).await;
                    }
                },
            )
            .await;
        });
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            if let Some(guild_id) = command.guild_id {
                let data = ctx.data.read().await;
                if let Some(api) = data.get::<ApiClientKey>() {
                    if !sentinel_shared::discord_helpers::is_bot_enabled(
                        api,
                        &guild_id.to_string(),
                    )
                    .await
                    {
                        return;
                    }
                }
            }

            if command.data.name.as_str() == "audit" {
                commands::audit::handle(&ctx, &command).await;
            }
        }
    }

    /// Intercepte tous les messages pour les cacher.
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let guild_id = match msg.guild_id {
            Some(g) => g,
            None => return,
        };

        // Verifier si le bot est active
        {
            let data = ctx.data.read().await;
            if let Some(base) = data.get::<ApiClientKey>() {
                let config = match base.get_guild_config(&guild_id.to_string()).await {
                    Ok(c) => c,
                    Err(e) => {
                        warn!(error = %e, "Failed to fetch guild config");
                        return;
                    }
                };
                if !BaseApiClient::config_bool(&config, "enabled", true) {
                    return;
                }
            }
        }

        let data = ctx.data.read().await;
        if let Some(cache) = data.get::<MessageCacheKey>() {
            cache.store(
                guild_id,
                msg.id,
                crate::message_cache::CachedMessage {
                    author_id: msg.author.id.to_string(),
                    author_name: msg.author.name.clone(),
                    content: msg.content.clone(),
                    channel_id: msg.channel_id.to_string(),
                },
            );
        }

        // Surveillance : tracker les messages des utilisateurs surveilles
        let user_id = msg.author.id.to_string();
        if Self::is_watched(&data, &user_id) {
            drop(data);
            let channel_name = msg
                .channel_id
                .to_channel(&ctx.http)
                .await
                .ok()
                .and_then(|c| c.guild())
                .map(|c| c.name.clone());

            Self::track_activity(
                &ctx,
                &guild_id.to_string(),
                &user_id,
                "message_sent",
                Some(&msg.channel_id.to_string()),
                channel_name.as_deref(),
                Some(&msg.content),
                serde_json::json!({"message_id": msg.id.to_string()}),
            )
            .await;
        }
    }

    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        message_id: MessageId,
        guild_id: Option<GuildId>,
    ) {
        if let Some(gid) = guild_id {
            if !Self::is_guild_enabled(&ctx, &gid.to_string()).await {
                return;
            }
        }
        handlers::message::handle_delete(&ctx, channel_id, message_id, guild_id).await;
    }

    async fn message_update(
        &self,
        ctx: Context,
        old: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        handlers::message::handle_update(&ctx, old, new, event).await;
    }

    async fn message_delete_bulk(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        multiple_deleted: Vec<MessageId>,
        guild_id: Option<GuildId>,
    ) {
        if let Some(gid) = guild_id {
            if !Self::is_guild_enabled(&ctx, &gid.to_string()).await {
                return;
            }
        }
        handlers::message::handle_delete_bulk(&ctx, channel_id, multiple_deleted, guild_id).await;
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        if !Self::is_guild_enabled(&ctx, &new_member.guild_id.to_string()).await {
            return;
        }
        handlers::member::handle_addition(&ctx, &new_member).await;
    }

    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: User,
        _member: Option<Member>,
    ) {
        if !Self::is_guild_enabled(&ctx, &guild_id.to_string()).await {
            return;
        }
        handlers::member::handle_removal(&ctx, guild_id, &user).await;
    }

    async fn guild_ban_addition(&self, ctx: Context, guild_id: GuildId, banned_user: User) {
        if !Self::is_guild_enabled(&ctx, &guild_id.to_string()).await {
            return;
        }
        handlers::member::handle_ban_addition(&ctx, guild_id, &banned_user).await;
    }

    async fn guild_ban_removal(&self, ctx: Context, guild_id: GuildId, unbanned_user: User) {
        if !Self::is_guild_enabled(&ctx, &guild_id.to_string()).await {
            return;
        }
        handlers::member::handle_ban_removal(&ctx, guild_id, &unbanned_user).await;
    }

    async fn guild_member_update(
        &self,
        ctx: Context,
        old: Option<Member>,
        new: Option<Member>,
        _event: serenity::model::event::GuildMemberUpdateEvent,
    ) {
        if let Some(ref new_member) = new {
            handlers::member::handle_update(&ctx, old, new_member).await;
        }
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        handlers::voice::handle_state_update(&ctx, old, &new).await;
    }

    async fn channel_create(&self, ctx: Context, channel: serenity::model::channel::GuildChannel) {
        handlers::channel::handle_create(&ctx, &channel).await;
    }

    async fn channel_delete(
        &self,
        ctx: Context,
        channel: serenity::model::channel::GuildChannel,
        messages: Option<Vec<Message>>,
    ) {
        handlers::channel::handle_delete(&ctx, &channel, messages).await;
    }

    async fn guild_role_create(&self, ctx: Context, new: Role) {
        handlers::role::handle_create(&ctx, &new).await;
    }

    async fn guild_role_delete(
        &self,
        ctx: Context,
        guild_id: GuildId,
        removed_role_id: RoleId,
        removed_role: Option<Role>,
    ) {
        handlers::role::handle_delete(&ctx, guild_id, removed_role_id, removed_role).await;
    }

    async fn guild_role_update(&self, ctx: Context, old: Option<Role>, new: Role) {
        handlers::role::handle_update(&ctx, old, &new).await;
    }

    async fn guild_update(
        &self,
        ctx: Context,
        old: Option<Guild>,
        new_incomplete: serenity::model::guild::PartialGuild,
    ) {
        handlers::guild::handle_update(&ctx, old, &new_incomplete).await;
    }

    async fn thread_create(&self, ctx: Context, thread: serenity::model::channel::GuildChannel) {
        handlers::thread::handle_create(&ctx, &thread).await;
    }

    async fn thread_delete(
        &self,
        ctx: Context,
        thread: serenity::model::channel::PartialGuildChannel,
        full_thread: Option<serenity::model::channel::GuildChannel>,
    ) {
        handlers::thread::handle_delete(&ctx, &thread, full_thread).await;
    }

    async fn invite_create(&self, ctx: Context, data: serenity::model::event::InviteCreateEvent) {
        handlers::invite::handle_create(&ctx, &data).await;
    }

    async fn invite_delete(&self, ctx: Context, data: serenity::model::event::InviteDeleteEvent) {
        handlers::invite::handle_delete(&ctx, &data).await;
    }
}
