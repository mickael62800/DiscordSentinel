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
use sentinel_shared::heartbeat::{ApiClientKey, register_guilds};

use crate::anomaly::AnomalyDetector;
use crate::api_client::{ApiClient, AuditEvent};
use crate::commands;
use crate::config::Config;
use crate::handlers;
use crate::message_cache::{CachedMessage, MessageCache};
use crate::weekly_report::WeeklyTracker;

// ── TypeMap keys ──

pub struct MessageCacheKey;
impl TypeMapKey for MessageCacheKey {
    type Value = MessageCache;
}

pub struct AnomalyDetectorKey;
impl TypeMapKey for AnomalyDetectorKey {
    type Value = AnomalyDetector;
}

pub struct WeeklyTrackerKey;
impl TypeMapKey for WeeklyTrackerKey {
    type Value = WeeklyTracker;
}

pub struct ConfigKey;
impl TypeMapKey for ConfigKey {
    type Value = Config;
}

pub struct Handler;

impl Handler {
    pub async fn send_event(ctx: &Context, event: AuditEvent) {
        let data = ctx.data.read().await;
        if let Some(base) = data.get::<ApiClientKey>() {
            let api = ApiClient::new(base.clone());
            if let Err(e) = api.send_audit_event(&event).await {
                warn!(error = %e, event_type = %event.event_type, "Erreur envoi audit event");
            }
        }
    }

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
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            if let Some(guild_id) = command.guild_id {
                let data = ctx.data.read().await;
                if let Some(api) = data.get::<ApiClientKey>() {
                    if !sentinel_shared::discord_helpers::is_bot_enabled(api, &guild_id.to_string()).await {
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
                let config = base.get_guild_config(&guild_id.to_string()).await.unwrap_or_default();
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
                CachedMessage {
                    author_id: msg.author.id.to_string(),
                    author_name: msg.author.name.clone(),
                    content: msg.content.clone(),
                    channel_id: msg.channel_id.to_string(),
                },
            );
        }
    }

    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        message_id: MessageId,
        guild_id: Option<GuildId>,
    ) {
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
        handlers::message::handle_delete_bulk(&ctx, channel_id, multiple_deleted, guild_id).await;
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        handlers::member::handle_addition(&ctx, &new_member).await;
    }

    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: User,
        _member: Option<Member>,
    ) {
        handlers::member::handle_removal(&ctx, guild_id, &user).await;
    }

    async fn guild_ban_addition(&self, ctx: Context, guild_id: GuildId, banned_user: User) {
        handlers::member::handle_ban_addition(&ctx, guild_id, &banned_user).await;
    }

    async fn guild_ban_removal(&self, ctx: Context, guild_id: GuildId, unbanned_user: User) {
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

    async fn guild_update(&self, ctx: Context, old: Option<Guild>, new_incomplete: serenity::model::guild::PartialGuild) {
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
