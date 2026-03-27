use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::event::MessageUpdateEvent;
use serenity::model::gateway::Ready;
use serenity::model::guild::Member;
use serenity::model::id::{ChannelId, GuildId, MessageId};
use serenity::model::user::User;
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use tracing::{info, warn};

use crate::api_client::{ApiClient, AuditEvent};

pub struct ApiClientKey;

impl TypeMapKey for ApiClientKey {
    type Value = ApiClient;
}

pub struct Handler;

impl Handler {
    async fn send_event(ctx: &Context, event: AuditEvent) {
        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            if let Err(e) = api.send_audit_event(&event).await {
                warn!(error = %e, event_type = %event.event_type, "Erreur envoi audit event");
            }
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(bot = %ready.user.name, guilds = ready.guilds.len(), "Audit bot connecte");

        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            for guild_status in &ready.guilds {
                let guild_id = guild_status.id;
                if let Ok(guild) = guild_id.to_partial_guild(&ctx.http).await {
                    let member_count = guild.approximate_member_count.unwrap_or(0) as i32;
                    if let Err(e) = api
                        .register_guild(&guild_id.to_string(), &guild.name, member_count)
                        .await
                    {
                        warn!(error = %e, guild = %guild.name, "Erreur enregistrement guild");
                    }
                }
            }
        }
    }

    // ── Message supprime ──

    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        message_id: MessageId,
        guild_id: Option<GuildId>,
    ) {
        let gid = match guild_id {
            Some(g) => g.to_string(),
            None => return,
        };

        let channel_name = channel_id
            .to_channel(&ctx.http)
            .await
            .ok()
            .and_then(|c| c.guild().map(|gc| gc.name.clone()));

        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: gid,
                event_type: "message_delete".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: Some(message_id.to_string()),
                target_name: None,
                channel_id: Some(channel_id.to_string()),
                channel_name,
                details: serde_json::json!({}),
            },
        )
        .await;
    }

    // ── Message edite ──

    async fn message_update(
        &self,
        ctx: Context,
        _old: Option<Message>,
        _new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        let gid = match event.guild_id {
            Some(g) => g.to_string(),
            None => return,
        };

        let author_id = event.author.as_ref().map(|a| a.id.to_string());
        let author_name = event.author.as_ref().map(|a| a.name.clone());
        let new_content = event.content.clone().unwrap_or_default();

        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: gid,
                event_type: "message_edit".to_string(),
                actor_id: author_id,
                actor_name: author_name,
                target_id: Some(event.id.to_string()),
                target_name: None,
                channel_id: Some(event.channel_id.to_string()),
                channel_name: None,
                details: serde_json::json!({
                    "new_content": new_content,
                }),
            },
        )
        .await;
    }

    // ── Membre rejoint ──

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: new_member.guild_id.to_string(),
                event_type: "member_join".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: Some(new_member.user.id.to_string()),
                target_name: Some(new_member.user.name.clone()),
                channel_id: None,
                channel_name: None,
                details: serde_json::json!({
                    "account_created_at": new_member.user.created_at().to_string(),
                }),
            },
        )
        .await;
    }

    // ── Membre quitte ──

    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: User,
        _member: Option<Member>,
    ) {
        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: guild_id.to_string(),
                event_type: "member_leave".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: Some(user.id.to_string()),
                target_name: Some(user.name.clone()),
                channel_id: None,
                channel_name: None,
                details: serde_json::json!({}),
            },
        )
        .await;
    }

    // ── Ban ──

    async fn guild_ban_addition(&self, ctx: Context, guild_id: GuildId, banned_user: User) {
        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: guild_id.to_string(),
                event_type: "member_ban".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: Some(banned_user.id.to_string()),
                target_name: Some(banned_user.name.clone()),
                channel_id: None,
                channel_name: None,
                details: serde_json::json!({}),
            },
        )
        .await;
    }

    // ── Unban ──

    async fn guild_ban_removal(&self, ctx: Context, guild_id: GuildId, unbanned_user: User) {
        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: guild_id.to_string(),
                event_type: "member_unban".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: Some(unbanned_user.id.to_string()),
                target_name: Some(unbanned_user.name.clone()),
                channel_id: None,
                channel_name: None,
                details: serde_json::json!({}),
            },
        )
        .await;
    }

    // ── Changement vocal ──

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        let gid = match new.guild_id {
            Some(g) => g.to_string(),
            None => return,
        };

        let user_id = new.user_id.to_string();
        let user_name = new
            .member
            .as_ref()
            .map(|m| m.user.name.clone())
            .unwrap_or_default();

        let old_channel = old.as_ref().and_then(|o| o.channel_id);
        let new_channel = new.channel_id;

        let event_type = match (old_channel, new_channel) {
            (None, Some(_)) => "voice_join",
            (Some(_), None) => "voice_leave",
            (Some(a), Some(b)) if a != b => "voice_move",
            _ => return,
        };

        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: gid,
                event_type: event_type.to_string(),
                actor_id: Some(user_id),
                actor_name: Some(user_name),
                target_id: None,
                target_name: None,
                channel_id: new_channel.map(|c| c.to_string()),
                channel_name: None,
                details: serde_json::json!({
                    "from_channel": old_channel.map(|c| c.to_string()),
                    "to_channel": new_channel.map(|c| c.to_string()),
                }),
            },
        )
        .await;
    }

    // ── Salon cree ──

    async fn channel_create(&self, ctx: Context, channel: serenity::model::channel::GuildChannel) {
        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: channel.guild_id.to_string(),
                event_type: "channel_create".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: Some(channel.id.to_string()),
                target_name: Some(channel.name.clone()),
                channel_id: Some(channel.id.to_string()),
                channel_name: Some(channel.name.clone()),
                details: serde_json::json!({
                    "kind": format!("{:?}", channel.kind),
                }),
            },
        )
        .await;
    }

    // ── Salon supprime ──

    async fn channel_delete(
        &self,
        ctx: Context,
        channel: serenity::model::channel::GuildChannel,
        _messages: Option<Vec<Message>>,
    ) {
        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: channel.guild_id.to_string(),
                event_type: "channel_delete".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: Some(channel.id.to_string()),
                target_name: Some(channel.name.clone()),
                channel_id: Some(channel.id.to_string()),
                channel_name: Some(channel.name.clone()),
                details: serde_json::json!({}),
            },
        )
        .await;
    }

    // ── Changement de role d'un membre ──

    async fn guild_member_update(
        &self,
        ctx: Context,
        old: Option<Member>,
        new: Option<Member>,
        _event: serenity::model::event::GuildMemberUpdateEvent,
    ) {
        let new_member = match new {
            Some(m) => m,
            None => return,
        };

        let gid = new_member.guild_id.to_string();
        let old_roles: Vec<String> = old
            .as_ref()
            .map(|m| m.roles.iter().map(|r| r.to_string()).collect())
            .unwrap_or_default();
        let new_roles: Vec<String> = new_member.roles.iter().map(|r| r.to_string()).collect();

        if old_roles == new_roles {
            return;
        }

        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: gid,
                event_type: "member_roles_update".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: Some(new_member.user.id.to_string()),
                target_name: Some(new_member.user.name.clone()),
                channel_id: None,
                channel_name: None,
                details: serde_json::json!({
                    "old_roles": old_roles,
                    "new_roles": new_roles,
                }),
            },
        )
        .await;
    }
}
