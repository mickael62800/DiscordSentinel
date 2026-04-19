//! EventHandler unifie — dispatche vers les modules.

use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::channel::{GuildChannel, Message};
use serenity::model::event::MessageUpdateEvent;
use serenity::model::gateway::Ready;
use serenity::model::guild::{Guild, Member, Role};
use serenity::model::id::{ChannelId, GuildId, MessageId, RoleId};
use serenity::model::user::User;
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use tracing::info;

use sentinel_shared::heartbeat::register_guilds;

use crate::modules;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(
            bot = %ready.user.name,
            guilds = ready.guilds.len(),
            "Sentinel Bot connecte"
        );

        register_guilds(&ctx, &ready).await;

        // Enregistrer toutes les commandes slash en une fois.
        let mut commands = Vec::new();
        commands.extend(modules::cleanup::register_commands());
        commands.extend(modules::games::register_commands());
        commands.extend(modules::community::register_commands());
        commands.extend(modules::audit::register_commands());
        commands.extend(modules::progression::register_commands());
        commands.extend(modules::blackjack::register_commands());
        commands.extend(modules::security::register_commands());
        commands.extend(modules::automod::register_commands());
        commands.extend(modules::moderation::register_commands());
        commands.extend(modules::voice::register_commands());
        commands.extend(modules::coude::register_commands());
        commands.extend(modules::tickets::register_commands());

        if let Err(e) = serenity::model::application::Command::set_global_commands(
            &ctx.http,
            commands,
        )
        .await
        {
            tracing::error!(error = %e, "Erreur enregistrement commandes");
        } else {
            info!("Slash commands enregistrees (sentinel-bot unifie)");
        }

        // Charger les roles temporaires actifs + spawn cleanup
        let guild_ids: Vec<_> = ready.guilds.iter().map(|g| g.id).collect();
        modules::community::load_temp_roles(&ctx, &guild_ids).await;
        modules::community::spawn_temp_role_cleanup(ctx.clone());

        // Background tasks blackjack (AFK cleanup consumer)
        modules::blackjack::spawn_background(ctx.clone());

        // Security: sync membres au demarrage + background tasks
        let ctx_sec = ctx.clone();
        let guilds_for_sec: Vec<_> = ready.guilds.clone();
        tokio::spawn(async move {
            modules::security::on_ready_sync(&ctx_sec, &guilds_for_sec).await;
        });
        modules::security::spawn_background(ctx.clone());

        // Sync periodique des roles Discord vers l'API (5 min)
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            modules::community::sync_all_guild_roles(&ctx_clone).await;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
                modules::community::sync_all_guild_roles(&ctx_clone).await;
            }
        });

        // Audit: bootstrap watched users + Redis consumer
        modules::audit::on_ready(&ctx).await;

        // Automod: background tasks (slowmode deactivation + cache cleanup)
        modules::automod::spawn_background_tasks(&ctx);

        // Moderation: Redis consumer pour events externes
        modules::moderation::spawn_background(ctx.clone());

        // Voice: reconcile + spawn AFK sweep
        modules::voice::on_ready(&ctx, &ready).await;

        // Coude: stocker les guild IDs + spawn background tasks
        let coude_guild_ids: Vec<_> = ready.guilds.iter().map(|g| g.id).collect();
        modules::coude::on_ready(&ctx, coude_guild_ids).await;
        modules::coude::spawn_background(ctx.clone());

        // Tickets: deploy panel + spawn background tasks (inactive close, SLA, Redis consumer)
        modules::tickets::on_ready(&ctx, &ready).await;
        modules::tickets::spawn_background(ctx.clone());

        // Progression: hydrate voice sessions + tick periodique credit XP
        modules::progression::on_ready(&ctx, &ready).await;
        modules::progression::spawn_voice_tick(ctx.clone());
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }
        modules::automod::on_message(&ctx, &msg).await;
        modules::audit::on_message(&ctx, &msg).await;
        modules::progression::on_message(&ctx, &msg).await;
        modules::voice::on_message(&ctx, &msg).await;
        modules::tickets::on_message(&ctx, &msg).await;
    }

    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        message_id: MessageId,
        guild_id: Option<GuildId>,
    ) {
        modules::audit::on_message_delete(&ctx, channel_id, message_id, guild_id).await;
    }

    async fn message_update(
        &self,
        ctx: Context,
        old: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        modules::audit::on_message_update(&ctx, old, new, event).await;
    }

    async fn message_delete_bulk(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        multiple_deleted: Vec<MessageId>,
        guild_id: Option<GuildId>,
    ) {
        modules::audit::on_message_delete_bulk(&ctx, channel_id, multiple_deleted, guild_id).await;
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        modules::audit::on_member_add(&ctx, &new_member).await;
        modules::welcome::on_member_add(&ctx, &new_member).await;
        modules::progression::assign_default_role(&ctx, &new_member).await;
        modules::community::on_member_add(&ctx, &new_member).await;
        modules::security::on_member_add(&ctx, &new_member).await;
    }

    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: User,
        _member: Option<Member>,
    ) {
        modules::audit::on_member_remove(&ctx, guild_id, &user).await;
        modules::welcome::on_member_remove(&ctx, guild_id, &user).await;
        modules::security::on_member_remove(&ctx, guild_id, &user).await;
    }

    async fn guild_member_update(
        &self,
        ctx: Context,
        old: Option<Member>,
        new_member: Option<Member>,
        event: serenity::model::event::GuildMemberUpdateEvent,
    ) {
        modules::audit::on_member_update(&ctx, old.clone(), new_member.clone(), event).await;
        if let Some(member) = new_member {
            modules::security::on_member_update(&ctx, &member).await;
        }
    }

    async fn channel_create(&self, ctx: Context, channel: GuildChannel) {
        modules::audit::on_channel_create(&ctx, &channel).await;
    }

    async fn channel_delete(
        &self,
        ctx: Context,
        channel: GuildChannel,
        messages: Option<Vec<Message>>,
    ) {
        modules::audit::on_channel_delete(&ctx, &channel, messages).await;
    }

    async fn guild_ban_addition(
        &self,
        ctx: Context,
        guild_id: GuildId,
        banned_user: User,
    ) {
        modules::audit::on_ban_add(&ctx, guild_id, &banned_user).await;
        modules::security::on_ban_add(&ctx, guild_id, &banned_user).await;
    }

    async fn guild_ban_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        unbanned_user: User,
    ) {
        modules::audit::on_ban_remove(&ctx, guild_id, &unbanned_user).await;
        modules::security::on_ban_remove(&ctx, guild_id, &unbanned_user).await;
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        modules::audit::on_voice_state_update(&ctx, old.clone(), &new).await;
        modules::voice::on_voice_state_update(&ctx, &old, &new).await;
        modules::progression::on_voice_state_update(&ctx, old, &new).await;
    }

    async fn guild_role_create(&self, ctx: Context, new: Role) {
        modules::audit::on_role_create(&ctx, &new).await;
    }

    async fn guild_role_delete(
        &self,
        ctx: Context,
        guild_id: GuildId,
        removed_role_id: RoleId,
        removed_role: Option<Role>,
    ) {
        modules::audit::on_role_delete(&ctx, guild_id, removed_role_id, removed_role).await;
    }

    async fn guild_role_update(&self, ctx: Context, old: Option<Role>, new: Role) {
        modules::audit::on_role_update(&ctx, old, &new).await;
    }

    async fn guild_update(
        &self,
        ctx: Context,
        old: Option<Guild>,
        new_incomplete: serenity::model::guild::PartialGuild,
    ) {
        modules::audit::on_guild_update(&ctx, old, &new_incomplete).await;
    }

    async fn thread_create(&self, ctx: Context, thread: GuildChannel) {
        modules::audit::on_thread_create(&ctx, &thread).await;
    }

    async fn thread_delete(
        &self,
        ctx: Context,
        thread: serenity::model::channel::PartialGuildChannel,
        full_thread: Option<GuildChannel>,
    ) {
        modules::audit::on_thread_delete(&ctx, &thread, full_thread).await;
    }

    async fn invite_create(&self, ctx: Context, data: serenity::model::event::InviteCreateEvent) {
        modules::audit::on_invite_create(&ctx, &data).await;
    }

    async fn invite_delete(&self, ctx: Context, data: serenity::model::event::InviteDeleteEvent) {
        modules::audit::on_invite_delete(&ctx, &data).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                let name = command.data.name.as_str();

                // Prison check (coude) : bloque les commandes gameplay si
                // le joueur est en prison, puis return.
                if modules::coude::handles_command(name)
                    && modules::coude::check_and_reply_if_in_prison(&ctx, &command).await
                {
                    return;
                }

                match name {
                    "purge" | "cleanup" => modules::cleanup::handle_command(&ctx, &command).await,
                    "game" | "game-admin" => modules::games::handle_command(&ctx, &command).await,
                    "roles-panel" | "parrain" => modules::community::handle_command(&ctx, &command).await,
                    "audit" => modules::audit::handle_command(&ctx, &command).await,
                    "level" | "stats" => modules::progression::handle_command(&ctx, &command).await,
                    "blackjack-setup" => modules::blackjack::handle_command(&ctx, &command).await,
                    "security" => modules::security::handle_command(&ctx, &command).await,
                    "automod" => modules::automod::handle_command(&ctx, &command).await,
                    "warn" | "unwarn" | "mute" | "unmute" | "ban" | "unban" | "history"
                    | "note" | "call" | "context" | "appeal" | "expirations" | "compare"
                    | "modstats" | "evidence" | "review" | "template" | "transcript"
                    | "export" | "massmute" | "massban" => {
                        modules::moderation::handle_command(&ctx, &command).await
                    }
                    "ticket" => modules::tickets::handle_command(&ctx, &command).await,
                    _ if modules::coude::handles_command(name) => {
                        modules::coude::handle_command(&ctx, &command).await
                    }
                    _ => {}
                }
            }
            Interaction::Component(component) => {
                let cid = component.data.custom_id.as_str();
                if modules::welcome::handles_component(cid) {
                    modules::welcome::on_component(&ctx, &component).await;
                } else if modules::games::handles_component(cid) {
                    modules::games::on_component(&ctx, &component).await;
                } else if modules::community::handles_component(cid) {
                    modules::community::on_component(&ctx, &component).await;
                } else if modules::blackjack::handles_component(cid) {
                    modules::blackjack::on_component(&ctx, &component).await;
                } else if modules::security::handles_component(cid) {
                    modules::security::on_component(&ctx, &component).await;
                } else if modules::automod::handles_component(cid) {
                    modules::automod::on_component(&ctx, &component).await;
                } else if modules::moderation::handles_component(cid) {
                    modules::moderation::on_component(&ctx, &component).await;
                } else if modules::voice::handles_component(cid) {
                    modules::voice::on_component(&ctx, &component).await;
                } else if modules::coude::handles_component(cid) {
                    // Prison check : bloque les boutons offensifs en prison.
                    if modules::coude::check_component_in_prison(&ctx, &component).await {
                        return;
                    }
                    modules::coude::on_component(&ctx, &component).await;
                } else if modules::tickets::handles_component(cid) {
                    modules::tickets::on_component(&ctx, &component).await;
                }
            }
            Interaction::Modal(modal) => {
                let mcid = modal.data.custom_id.as_str();
                if modules::voice::handles_modal(mcid) {
                    modules::voice::on_modal(&ctx, &modal).await;
                } else if modules::tickets::handles_modal(mcid) {
                    modules::tickets::on_modal(&ctx, &modal).await;
                }
            }
            Interaction::Autocomplete(autocomplete) => {
                let cmd_name = autocomplete.data.name.as_str();
                if modules::moderation::handles_autocomplete(cmd_name) {
                    modules::moderation::handle_autocomplete(&ctx, &autocomplete).await;
                }
            }
            _ => {}
        }
    }
}
