use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::event::MessageUpdateEvent;
use serenity::model::gateway::Ready;
use serenity::model::guild::{Guild, Member, Role};
use serenity::model::id::{ChannelId, GuildId, MessageId, RoleId};
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

    async fn log(ctx: &Context, level: &str, guild_id: &str, message: &str) {
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

        let data = ctx.data.read().await;
        if let Some(api) = data.get::<ApiClientKey>() {
            api.send_bot_log("info", "Audit bot demarre");
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

        let chan_label = channel_name.as_deref().unwrap_or("?");
        Self::log(&ctx, "warn", &gid, &format!("Message {} supprime dans #{}", message_id, chan_label)).await;

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
        old: Option<Message>,
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
        let old_content = old.as_ref().map(|m| m.content.clone()).unwrap_or_default();

        let name = author_name.as_deref().unwrap_or("?");
        Self::log(&ctx, "info", &gid, &format!(
            "{} a modifie un message — avant: \"{}\" | apres: \"{}\"",
            name,
            if old_content.is_empty() { "(inconnu)" } else { &old_content },
            new_content
        )).await;

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
                    "old_content": old_content,
                    "new_content": new_content,
                }),
            },
        )
        .await;
    }

    // ── Membre rejoint ──

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        Self::log(&ctx, "info", &new_member.guild_id.to_string(), &format!(
            "Nouveau membre : {} ({}) — compte cree le {}",
            new_member.user.name, new_member.user.id, new_member.user.created_at()
        )).await;

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
        Self::log(&ctx, "warn", &guild_id.to_string(), &format!(
            "Membre parti : {} ({})", user.name, user.id
        )).await;

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
        Self::log(&ctx, "error", &guild_id.to_string(), &format!(
            "Membre banni : {} ({})", banned_user.name, banned_user.id
        )).await;

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
        Self::log(&ctx, "info", &guild_id.to_string(), &format!(
            "Membre debanni : {} ({})", unbanned_user.name, unbanned_user.id
        )).await;

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

        let voice_msg = match event_type {
            "voice_join" => format!("{} a rejoint le salon vocal {}", user_name, new_channel.unwrap()),
            "voice_leave" => format!("{} a quitte le salon vocal {}", user_name, old_channel.unwrap()),
            "voice_move" => format!("{} a change de salon vocal {} -> {}", user_name, old_channel.unwrap(), new_channel.unwrap()),
            _ => String::new(),
        };
        Self::log(&ctx, "info", &gid, &voice_msg).await;

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
        Self::log(&ctx, "info", &channel.guild_id.to_string(), &format!(
            "Salon cree : #{} ({:?})", channel.name, channel.kind
        )).await;

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
        Self::log(&ctx, "warn", &channel.guild_id.to_string(), &format!(
            "Salon supprime : #{}", channel.name
        )).await;

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
        let user_name = &new_member.user.name;
        let user_id = new_member.user.id.to_string();

        // Changement de pseudo (nickname)
        let old_nick = old.as_ref().and_then(|m| m.nick.clone());
        let new_nick = new_member.nick.clone();
        if old_nick != new_nick {
            let old_label = old_nick.as_deref().unwrap_or("(aucun)");
            let new_label = new_nick.as_deref().unwrap_or("(aucun)");
            Self::log(&ctx, "info", &gid, &format!(
                "{} a change de pseudo : {} -> {}", user_name, old_label, new_label
            )).await;

            Self::send_event(
                &ctx,
                AuditEvent {
                    guild_id: gid.clone(),
                    event_type: "member_nickname_update".to_string(),
                    actor_id: None,
                    actor_name: None,
                    target_id: Some(user_id.clone()),
                    target_name: Some(user_name.clone()),
                    channel_id: None,
                    channel_name: None,
                    details: serde_json::json!({
                        "old_nickname": old_label,
                        "new_nickname": new_label,
                    }),
                },
            )
            .await;
        }

        // Changement d'avatar serveur
        let old_avatar = old.as_ref().and_then(|m| m.avatar.map(|a| a.to_string()));
        let new_avatar = new_member.avatar.map(|a| a.to_string());
        if old_avatar != new_avatar {
            Self::log(&ctx, "info", &gid, &format!(
                "{} a change son avatar serveur", user_name
            )).await;

            Self::send_event(
                &ctx,
                AuditEvent {
                    guild_id: gid.clone(),
                    event_type: "member_avatar_update".to_string(),
                    actor_id: None,
                    actor_name: None,
                    target_id: Some(user_id.clone()),
                    target_name: Some(user_name.clone()),
                    channel_id: None,
                    channel_name: None,
                    details: serde_json::json!({}),
                },
            )
            .await;
        }

        // Changement de roles
        let old_roles: Vec<String> = old
            .as_ref()
            .map(|m| m.roles.iter().map(|r| r.to_string()).collect())
            .unwrap_or_default();
        let new_roles: Vec<String> = new_member.roles.iter().map(|r| r.to_string()).collect();

        if old_roles != new_roles {
            Self::log(&ctx, "info", &gid, &format!(
                "{} — roles modifies", user_name
            )).await;

            Self::send_event(
                &ctx,
                AuditEvent {
                    guild_id: gid.clone(),
                    event_type: "member_roles_update".to_string(),
                    actor_id: None,
                    actor_name: None,
                    target_id: Some(user_id.clone()),
                    target_name: Some(user_name.clone()),
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

        // Timeout (mute) detecte
        let old_timeout = old.as_ref().and_then(|m| m.communication_disabled_until);
        let new_timeout = new_member.communication_disabled_until;
        if old_timeout.is_none() && new_timeout.is_some() {
            Self::log(&ctx, "warn", &gid, &format!(
                "{} a ete mute (timeout jusqu'a {})", user_name, new_timeout.unwrap()
            )).await;

            Self::send_event(
                &ctx,
                AuditEvent {
                    guild_id: gid.clone(),
                    event_type: "member_timeout".to_string(),
                    actor_id: None,
                    actor_name: None,
                    target_id: Some(user_id.clone()),
                    target_name: Some(user_name.clone()),
                    channel_id: None,
                    channel_name: None,
                    details: serde_json::json!({
                        "timeout_until": new_timeout.unwrap().to_string(),
                    }),
                },
            )
            .await;
        } else if old_timeout.is_some() && new_timeout.is_none() {
            Self::log(&ctx, "info", &gid, &format!(
                "{} n'est plus mute (timeout leve)", user_name
            )).await;

            Self::send_event(
                &ctx,
                AuditEvent {
                    guild_id: gid,
                    event_type: "member_timeout_removed".to_string(),
                    actor_id: None,
                    actor_name: None,
                    target_id: Some(user_id),
                    target_name: Some(user_name.clone()),
                    channel_id: None,
                    channel_name: None,
                    details: serde_json::json!({}),
                },
            )
            .await;
        }
    }

    // ── Suppression en masse (purge) ──

    async fn message_delete_bulk(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        multiple_deleted: Vec<MessageId>,
        guild_id: Option<GuildId>,
    ) {
        let gid = match guild_id {
            Some(g) => g.to_string(),
            None => return,
        };

        let count = multiple_deleted.len();

        let channel_name = channel_id
            .to_channel(&ctx.http)
            .await
            .ok()
            .and_then(|c| c.guild().map(|gc| gc.name.clone()));

        let chan_label = channel_name.as_deref().unwrap_or("?");
        Self::log(&ctx, "error", &gid, &format!(
            "Purge : {} messages supprimes dans #{}", count, chan_label
        )).await;

        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: gid,
                event_type: "message_delete_bulk".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: None,
                target_name: None,
                channel_id: Some(channel_id.to_string()),
                channel_name,
                details: serde_json::json!({
                    "count": count,
                    "message_ids": multiple_deleted.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                }),
            },
        )
        .await;
    }

    // ── Role cree ──

    async fn guild_role_create(&self, ctx: Context, new: Role) {
        let gid = new.guild_id.to_string();

        Self::log(&ctx, "info", &gid, &format!(
            "Role cree : @{} (couleur: #{:06x}, permissions: {})", new.name, new.colour.0, new.permissions.bits()
        )).await;

        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: gid,
                event_type: "role_create".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: Some(new.id.to_string()),
                target_name: Some(new.name.clone()),
                channel_id: None,
                channel_name: None,
                details: serde_json::json!({
                    "colour": format!("#{:06x}", new.colour.0),
                    "permissions": new.permissions.bits().to_string(),
                    "position": new.position,
                    "mentionable": new.mentionable,
                    "hoist": new.hoist,
                }),
            },
        )
        .await;
    }

    // ── Role supprime ──

    async fn guild_role_delete(
        &self,
        ctx: Context,
        guild_id: GuildId,
        removed_role_id: RoleId,
        removed_role: Option<Role>,
    ) {
        let gid = guild_id.to_string();
        let role_name = removed_role.as_ref().map(|r| r.name.clone()).unwrap_or_else(|| removed_role_id.to_string());

        Self::log(&ctx, "warn", &gid, &format!(
            "Role supprime : @{}", role_name
        )).await;

        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: gid,
                event_type: "role_delete".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: Some(removed_role_id.to_string()),
                target_name: Some(role_name),
                channel_id: None,
                channel_name: None,
                details: serde_json::json!({}),
            },
        )
        .await;
    }

    // ── Role modifie ──

    async fn guild_role_update(&self, ctx: Context, old: Option<Role>, new: Role) {
        let gid = new.guild_id.to_string();

        let mut changes = Vec::new();
        if let Some(ref old_role) = old {
            if old_role.name != new.name {
                changes.push(format!("nom: {} -> {}", old_role.name, new.name));
            }
            if old_role.colour != new.colour {
                changes.push(format!("couleur: #{:06x} -> #{:06x}", old_role.colour.0, new.colour.0));
            }
            if old_role.permissions != new.permissions {
                changes.push("permissions modifiees".to_string());
            }
            if old_role.hoist != new.hoist {
                changes.push(format!("affiche separement: {} -> {}", old_role.hoist, new.hoist));
            }
            if old_role.mentionable != new.mentionable {
                changes.push(format!("mentionnable: {} -> {}", old_role.mentionable, new.mentionable));
            }
        }

        if changes.is_empty() {
            return;
        }

        Self::log(&ctx, "info", &gid, &format!(
            "Role modifie : @{} — {}", new.name, changes.join(", ")
        )).await;

        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: gid,
                event_type: "role_update".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: Some(new.id.to_string()),
                target_name: Some(new.name.clone()),
                channel_id: None,
                channel_name: None,
                details: serde_json::json!({
                    "changes": changes,
                    "old_permissions": old.as_ref().map(|r| r.permissions.bits().to_string()),
                    "new_permissions": new.permissions.bits().to_string(),
                }),
            },
        )
        .await;
    }

    // ── Serveur modifie ──

    async fn guild_update(&self, ctx: Context, old: Option<Guild>, new_incomplete: serenity::model::guild::PartialGuild) {
        let gid = new_incomplete.id.to_string();

        let mut changes = Vec::new();
        if let Some(ref old_guild) = old {
            if old_guild.name != new_incomplete.name {
                changes.push(format!("nom: {} -> {}", old_guild.name, new_incomplete.name));
            }
            if old_guild.icon != new_incomplete.icon {
                changes.push("icone modifiee".to_string());
            }
            if old_guild.verification_level != new_incomplete.verification_level {
                changes.push(format!("niveau verification: {:?} -> {:?}", old_guild.verification_level, new_incomplete.verification_level));
            }
            if old_guild.default_message_notifications != new_incomplete.default_message_notifications {
                changes.push("notifications par defaut modifiees".to_string());
            }
        }

        if changes.is_empty() {
            return;
        }

        Self::log(&ctx, "warn", &gid, &format!(
            "Serveur modifie : {}", changes.join(", ")
        )).await;

        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: gid,
                event_type: "guild_update".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: None,
                target_name: Some(new_incomplete.name.clone()),
                channel_id: None,
                channel_name: None,
                details: serde_json::json!({
                    "changes": changes,
                }),
            },
        )
        .await;
    }

    // ── Fil de discussion cree ──

    async fn thread_create(&self, ctx: Context, thread: serenity::model::channel::GuildChannel) {
        let gid = thread.guild_id.to_string();

        Self::log(&ctx, "info", &gid, &format!(
            "Fil cree : #{} (parent: {})", thread.name, thread.parent_id.map(|p| p.to_string()).unwrap_or_default()
        )).await;

        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: gid,
                event_type: "thread_create".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: Some(thread.id.to_string()),
                target_name: Some(thread.name.clone()),
                channel_id: thread.parent_id.map(|p| p.to_string()),
                channel_name: None,
                details: serde_json::json!({
                    "kind": format!("{:?}", thread.kind),
                }),
            },
        )
        .await;
    }

    // ── Fil de discussion supprime ──

    async fn thread_delete(
        &self,
        ctx: Context,
        thread: serenity::model::channel::PartialGuildChannel,
        full_thread: Option<serenity::model::channel::GuildChannel>,
    ) {
        let gid = thread.guild_id.to_string();
        let thread_name = full_thread.as_ref().map(|t| t.name.clone()).unwrap_or_else(|| thread.id.to_string());

        Self::log(&ctx, "warn", &gid, &format!(
            "Fil supprime : #{}", thread_name
        )).await;

        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: gid,
                event_type: "thread_delete".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: Some(thread.id.to_string()),
                target_name: Some(thread_name),
                channel_id: full_thread.as_ref().and_then(|t| t.parent_id.map(|p| p.to_string())),
                channel_name: None,
                details: serde_json::json!({}),
            },
        )
        .await;
    }

    // ── Invitation creee ──

    async fn invite_create(&self, ctx: Context, data: serenity::model::event::InviteCreateEvent) {
        let gid = match data.guild_id {
            Some(g) => g.to_string(),
            None => return,
        };

        let inviter_name = data.inviter.as_ref().map(|u| u.name.clone()).unwrap_or_else(|| "?".into());
        let inviter_id = data.inviter.as_ref().map(|u| u.id.to_string());
        let max_uses = data.max_uses;
        let max_age = data.max_age;

        Self::log(&ctx, "info", &gid, &format!(
            "Invitation creee par {} — code: {}, max uses: {}, expire: {}s",
            inviter_name, data.code, max_uses, max_age
        )).await;

        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: gid,
                event_type: "invite_create".to_string(),
                actor_id: inviter_id,
                actor_name: Some(inviter_name),
                target_id: None,
                target_name: None,
                channel_id: Some(data.channel_id.to_string()),
                channel_name: None,
                details: serde_json::json!({
                    "code": data.code,
                    "max_uses": max_uses,
                    "max_age": max_age,
                    "temporary": data.temporary,
                }),
            },
        )
        .await;
    }

    // ── Invitation supprimee ──

    async fn invite_delete(&self, ctx: Context, data: serenity::model::event::InviteDeleteEvent) {
        let gid = match data.guild_id {
            Some(g) => g.to_string(),
            None => return,
        };

        Self::log(&ctx, "info", &gid, &format!(
            "Invitation supprimee — code: {}", data.code
        )).await;

        Self::send_event(
            &ctx,
            AuditEvent {
                guild_id: gid,
                event_type: "invite_delete".to_string(),
                actor_id: None,
                actor_name: None,
                target_id: None,
                target_name: None,
                channel_id: Some(data.channel_id.to_string()),
                channel_name: None,
                details: serde_json::json!({
                    "code": data.code,
                }),
            },
        )
        .await;
    }
}
