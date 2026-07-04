//! EventHandler unifie — dispatche vers les modules.

use std::panic::AssertUnwindSafe;

use futures_util::FutureExt;
use serenity::async_trait;
use serenity::model::application::{
    CommandData, CommandDataOption, CommandDataOptionValue, Interaction,
};
use serenity::model::channel::{GuildChannel, Message};
use serenity::model::event::MessageUpdateEvent;
use serenity::model::gateway::Ready;
use serenity::model::guild::{Guild, Member, Role};
use serenity::model::id::{ChannelId, GuildId, MessageId, RoleId};
use serenity::model::user::User;
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use tracing::info;

use crate::shared::heartbeat::{register_guilds, ApiClientKey};

use crate::modules;

/// Retourne le module fonctionnel associe a une commande slash. Sert a
/// alimenter le champ `details.module` du log "command.invoked".
fn command_module(name: &str) -> &'static str {
    match name {
        "purge" | "cleanup" => "cleanup",
        "game" | "game-admin" => "games",
        "roles-panel" | "parrain" => "community",
        "audit" => "audit",
        "level" | "stats" | "progression-resync" | "classement" => "progression",
        "blackjack-setup" => "blackjack",
        "slot-setup" => "slot",
        "wheel-setup" => "wheel",
        "tama-setup" => "tamagotchi",
        "rotation" => "rotation",
        "security" => "security",
        "automod" => "automod",
        "warn" | "unwarn" | "mute" | "unmute" | "ban" | "unban" | "history" | "note" | "call"
        | "signalement" | "context" | "appeal" | "expirations" | "compare" | "modstats"
        | "evidence" | "review" | "template" | "transcript" | "export" | "massmute" | "massban"
        | "copilote" => "moderation",
        "ticket" | "ticket-admin" => "tickets",
        "confess" | "confess-admin" => "confessions",
        _ if modules::coude::handles_command(name) => "coude",
        _ if modules::influence::handles_command(name) => "influence",
        _ => "unknown",
    }
}

/// `true` si la commande est une commande admin/moderateur (loggue dans le
/// salon dedie `command_log_channel_id`). Couvre automod + toutes les commandes
/// de moderation/securite/nettoyage/audit/rotation, les `*-setup`, les `*-admin`
/// et les panneaux de config communautaires.
fn is_admin_command(name: &str) -> bool {
    matches!(
        command_module(name),
        "moderation" | "automod" | "security" | "cleanup" | "audit" | "rotation"
    ) || name.ends_with("-setup")
        || name.ends_with("-admin")
        || matches!(name, "roles-panel" | "parrain")
}

/// Reconstruit le nom complet de la commande slash y compris
/// subcommand_group / subcommand (ex: "ticket close all", "audit channel set").
fn format_full_command(data: &CommandData) -> String {
    let mut parts = vec![data.name.to_string()];
    fn descend(opts: &[CommandDataOption], parts: &mut Vec<String>) {
        for opt in opts {
            match &opt.value {
                CommandDataOptionValue::SubCommandGroup(sub_opts)
                | CommandDataOptionValue::SubCommand(sub_opts) => {
                    parts.push(opt.name.to_string());
                    descend(sub_opts, parts);
                }
                _ => {}
            }
        }
    }
    descend(&data.options, &mut parts);
    format!("/{}", parts.join(" "))
}

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

        // Enregistrement per-guild des slash commands : filtre les modules
        // desactives via command_registry. Remplace l'ancien set_global_commands
        // qui enregistrait tout pour tout le monde -> impossible de cacher
        // une commande d'un module desactive.
        // On vide aussi les commandes globales heritees (set vide) car elles
        // sont visibles partout meme apres bascule per-guild.
        let _ =
            serenity::model::application::Command::set_global_commands(&ctx.http, Vec::new()).await;
        let guild_ids: Vec<_> = ready.guilds.iter().map(|g| g.id).collect();
        crate::command_registry::refresh_all_guilds(&ctx, &guild_ids).await;

        // Listener Redis pour les events bot_enabled_changed -> re-register
        // les commandes guild a la volee quand un admin toggle on/off.
        crate::command_registry::spawn_consumer(ctx.clone());
        modules::community::load_temp_roles(&ctx, &guild_ids).await;
        modules::community::spawn_temp_role_cleanup(ctx.clone());

        // Background tasks blackjack (AFK cleanup consumer)
        modules::blackjack::spawn_background(ctx.clone());
        modules::bump::spawn_background(ctx.clone());

        // Security: sync membres au demarrage + background tasks
        let ctx_sec = ctx.clone();
        let guilds_for_sec: Vec<_> = ready.guilds.clone();
        tokio::spawn(async move {
            modules::security::on_ready_sync(&ctx_sec, &guilds_for_sec).await;
        });
        modules::security::spawn_background(ctx.clone());
        // Phase 5F — consumer Redis pour quarantine_expired (worker).
        modules::security::quarantine_expired_consumer::spawn(ctx.clone());
        // Phase 5G — consumer Redis pour lockdown_expired (worker).
        modules::security::lockdown_expired_consumer::spawn(ctx.clone());
        // Phase 5H — consumer Redis pour slowmode_expired (worker).
        modules::security::slowmode_expired_consumer::spawn(ctx.clone());

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
        modules::rotation::spawn_background_tasks(&ctx);

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

        // Announcements : consumer Redis stream pour les annonces planifiees
        // publiees par announcement-worker.
        modules::announcements::spawn(ctx.clone());

        // Confessions : consumer Redis stream pour synchroniser les
        // suppressions web -> Discord (delete confession ou reply).
        modules::confessions::spawn_consumer(ctx.clone());

        // Tamagotchi : refresh horaire des cartes + consumer maladie/mort (DM).
        modules::tamagotchi::spawn_background(ctx.clone());

        // Welcome : consumer Redis pour publier le panneau de reglement
        // (bouton "Publier le reglement" du dashboard).
        modules::welcome::spawn(ctx.clone());

        // Slot : fermeture auto des salons de machine a sous inactifs
        // (timeout par guild, defaut 2 min). Suivi en memoire -> cleanup
        // dans le bot, pas dans le worker.
        modules::slot::spawn_background(ctx.clone());

        // Games : consumer Redis pour deployer/rafraichir le panneau de jeux
        // (bouton "Deployer" du dashboard).
        modules::games::spawn(ctx.clone());
        modules::game_portal::spawn(ctx.clone());
        modules::influence::spawn(ctx.clone());

        // AI dataset : task de collecte (client-streaming gRPC longue duree).
        modules::ai_dataset::spawn_collector(ctx.clone()).await;
    }

    async fn message(&self, ctx: Context, msg: Message) {
        // On met en cache TOUS les messages (bots inclus) pour l'audit : ca
        // permet d'identifier une suppression de message de bot et de l'exclure
        // des logs. Le reste du pipeline ignore les bots.
        modules::audit::cache_message(&ctx, &msg).await;

        // Bump : la confirmation de /bump est postee par Disboard (un BOT). On
        // doit donc traiter ce module AVANT le filtre bot ci-dessous, sinon la
        // detection ne se declenche jamais. (Le module filtre lui-meme sur l'id
        // Disboard.)
        modules::bump::on_message(&ctx, &msg).await;

        if msg.author.bot {
            return;
        }
        // Salons "commandes uniquement" : supprime le message classique en
        // premier (avant l'XP / automod, qui n'ont pas a traiter un message
        // qui va disparaitre).
        modules::command_channel::on_message(&ctx, &msg).await;
        modules::automod::on_message(&ctx, &msg).await;
        modules::audit::on_message(&ctx, &msg).await;
        modules::progression::on_message(&ctx, &msg).await;
        modules::voice::on_message(&ctx, &msg).await;
        modules::tickets::on_message(&ctx, &msg).await;
        modules::ai_dataset::on_message(&ctx, &msg).await;
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
        // Bump : DiscordL edite un message vide pour y mettre l'embed de
        // resultat -> on re-detecte a l'edition (avant le move de `event`).
        modules::bump::on_message_update(&ctx, &event).await;
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
        // Prefixe emoji staff : applique l'emoji des le (re)join si le membre
        // porte deja un role staff (guarde par staff_prefix_enabled, best-effort).
        modules::progression::nickname::on_member_add(&ctx, &new_member).await;
        // Lifecycle : clear left_at + reset joined_at cote API. Le user
        // peut rejouer (wallet repart de zero, gere cote serveur).
        let guild_id = new_member.guild_id.to_string();
        let user_id = new_member.user.id.to_string();
        let api = ctx.data.read().await.get::<ApiClientKey>().cloned();
        if let Some(api) = api {
            let path = format!("/api/members/{guild_id}/{user_id}/rejoin");
            api.post_fire_and_forget(&path, &serde_json::json!({}))
                .await;
        }
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
        // Lifecycle : set left_at + reset wallet a 0. Le user n'apparaitra
        // plus dans les listes de jeu (filtrage cote query) mais ses donnees
        // non-jeu (infractions, audit, stats) sont conservees.
        let g = guild_id.to_string();
        let u = user.id.to_string();
        let api = ctx.data.read().await.get::<ApiClientKey>().cloned();
        if let Some(api) = api {
            let path = format!("/api/members/{g}/{u}/leave");
            api.post_fire_and_forget(&path, &serde_json::json!({}))
                .await;
        }
    }

    async fn guild_member_update(
        &self,
        ctx: Context,
        old: Option<Member>,
        new_member: Option<Member>,
        event: serenity::model::event::GuildMemberUpdateEvent,
    ) {
        // Fin du filtrage d'adhesion Discord (rules screening) : `pending`
        // passe de true a false -> on attribue le(s) role(s) du reglement.
        let screening_done = old.as_ref().map(|m| m.pending).unwrap_or(false) && !event.pending;
        let (sg_guild, sg_user) = (event.guild_id, event.user.id);

        modules::audit::on_member_update(&ctx, old.clone(), new_member.clone(), event).await;

        if screening_done {
            modules::welcome::on_screening_complete(&ctx, sg_guild, sg_user).await;
        }
        if let Some(member) = new_member {
            modules::security::on_member_update(&ctx, &member).await;
            // Prefixe emoji staff : recompute le pseudo au changement de role.
            modules::progression::nickname::on_member_update(&ctx, &member).await;
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

    async fn guild_ban_addition(&self, ctx: Context, guild_id: GuildId, banned_user: User) {
        modules::audit::on_ban_add(&ctx, guild_id, &banned_user).await;
        modules::security::on_ban_add(&ctx, guild_id, &banned_user).await;
    }

    async fn guild_ban_removal(&self, ctx: Context, guild_id: GuildId, unbanned_user: User) {
        modules::audit::on_ban_remove(&ctx, guild_id, &unbanned_user).await;
        modules::security::on_ban_remove(&ctx, guild_id, &unbanned_user).await;
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        modules::audit::on_voice_state_update(&ctx, old.clone(), &new).await;
        modules::voice::on_voice_state_update(&ctx, &old, &new).await;
        modules::welcome::on_voice_state_update(&ctx, &old, &new).await;
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

    /// Declenche quand le bot rejoint une nouvelle guild OU au re-sync au
    /// demarrage (is_new=Some(false) dans ce cas). On enregistre les
    /// slash commands + register cote API uniquement pour les vraies
    /// nouvelles guilds, sinon on duplique le travail deja fait dans `ready`.
    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: Option<bool>) {
        if is_new != Some(true) {
            return;
        }
        info!(guild_id = %guild.id, name = %guild.name, "Bot ajoute a une nouvelle guild");

        // 1. Register cote API (heartbeat / dashboard)
        {
            let data = ctx.data.read().await;
            if let Some(api) = data.get::<ApiClientKey>() {
                let member_count = guild.member_count as i32;
                let owner_id = guild.owner_id.to_string();
                if let Err(e) = api
                    .register_guild(
                        &guild.id.to_string(),
                        &guild.name,
                        member_count,
                        Some(&owner_id),
                    )
                    .await
                {
                    tracing::warn!(error = %e, guild = %guild.name, "Erreur enregistrement guild");
                }
            }
        }

        // 2. Refresh slash commands pour cette guild
        crate::command_registry::refresh_guild_commands(&ctx, guild.id).await;
    }

    /// Declenche quand le bot est retire d'une guild (kick/ban/serveur
    /// supprime) OU lors d'une indisponibilite temporaire Discord (outage).
    /// On distingue les deux via `incomplete.unavailable` : si true, c'est un
    /// outage -> on ne supprime PAS (le serveur reviendra). Si false, le bot a
    /// reellement quitte -> on purge cote API pour que le selecteur web cesse
    /// d'afficher un serveur fantome.
    async fn guild_delete(
        &self,
        ctx: Context,
        incomplete: serenity::model::guild::UnavailableGuild,
        _full: Option<Guild>,
    ) {
        if incomplete.unavailable {
            // Outage Discord : indisponibilite temporaire, pas un retrait.
            return;
        }
        info!(guild_id = %incomplete.id, "Bot retire d'une guild");
        let api = ctx.data.read().await.get::<ApiClientKey>().cloned();
        if let Some(api) = api {
            if let Err(e) = api.delete_guild(&incomplete.id.to_string()).await {
                tracing::warn!(error = %e, guild_id = %incomplete.id, "Erreur suppression guild");
            }
        }
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

                // ── Telemetrie commande : invoked + success/error ──
                let api = {
                    let data = ctx.data.read().await;
                    data.get::<ApiClientKey>().cloned()
                };
                let full_cmd = format_full_command(&command.data);
                let module = command_module(name);
                let user_id = command.user.id.to_string();
                let user_name = command.user.name.clone();
                let guild_id = command.guild_id.map(|g| g.to_string()).unwrap_or_default();

                // ANONYMAT : /confess ne doit JAMAIS lier l'auteur au module
                // confessions dans les logs (cf. revue securite). On coupe toute
                // telemetrie pour cette commande. (confess-admin reste loggue
                // pour la tracabilite des actions de moderation.)
                let log_telemetry = name != "confess";

                if let Some(ref api) = api {
                    if log_telemetry {
                        api.send_bot_log_with_details(
                            "info",
                            &format!("Commande invoquée : {full_cmd} (par @{user_name})"),
                            serde_json::json!({
                                "event_type": "command.invoked",
                                "command": full_cmd,
                                "module": module,
                                "user_id": user_id,
                                "user_name": user_name,
                                "guild_id": guild_id,
                            }),
                        );
                    }
                }

                // Log une-ligne des commandes admin/moderateur dans le salon
                // dedie et parametrable (opt-in via la config audit-bot).
                if !guild_id.is_empty() && is_admin_command(name) {
                    modules::audit::log_admin_command(
                        &ctx, &guild_id, &user_id, &user_name, &full_cmd,
                    )
                    .await;
                }

                let start = std::time::Instant::now();

                let dispatch = AssertUnwindSafe(async {
                    match name {
                        "purge" | "cleanup" => {
                            modules::cleanup::handle_command(&ctx, &command).await
                        }
                        "game" | "game-admin" => {
                            modules::games::handle_command(&ctx, &command).await
                        }
                        "roles-panel" | "parrain" => {
                            modules::community::handle_command(&ctx, &command).await
                        }
                        "audit" => modules::audit::handle_command(&ctx, &command).await,
                        "level" | "stats" | "progression-resync" | "classement" => {
                            modules::progression::handle_command(&ctx, &command).await
                        }
                        "blackjack-setup" => {
                            modules::blackjack::handle_command(&ctx, &command).await
                        }
                        "slot-setup" => modules::slot::handle_command(&ctx, &command).await,
                        "wheel-setup" => modules::wheel::handle_command(&ctx, &command).await,
                        "tama-setup" => modules::tamagotchi::handle_command(&ctx, &command).await,
                        "rotation" => modules::rotation::handle_command(&ctx, &command).await,
                        "security" => modules::security::handle_command(&ctx, &command).await,
                        "automod" => modules::automod::handle_command(&ctx, &command).await,
                        "warn" | "unwarn" | "mute" | "unmute" | "ban" | "unban" | "history"
                        | "note" | "call" | "signalement" | "context" | "appeal"
                        | "expirations" | "compare" | "modstats" | "evidence" | "review"
                        | "template" | "transcript" | "export" | "massmute" | "massban"
                        | "copilote" => {
                            modules::moderation::handle_command(&ctx, &command).await
                        }
                        "ticket" | "ticket-admin" => {
                            modules::tickets::handle_command(&ctx, &command).await
                        }
                        "confess" | "confess-admin" => {
                            modules::confessions::handle_command(&ctx, &command).await
                        }
                        _ if modules::coude::handles_command(name) => {
                            modules::coude::handle_command(&ctx, &command).await
                        }
                        _ if modules::influence::handles_command(name) => {
                            modules::influence::handle_command(&ctx, &command).await
                        }
                        _ => {}
                    }
                })
                .catch_unwind()
                .await;

                let elapsed_ms = start.elapsed().as_millis() as u64;

                if let Some(ref api) = api {
                    if log_telemetry {
                        match dispatch {
                            Ok(()) => api.send_bot_log_with_details(
                                "info",
                                &format!("Commande OK : {full_cmd} ({elapsed_ms} ms)"),
                                serde_json::json!({
                                    "event_type": "command.success",
                                    "command": full_cmd,
                                    "module": module,
                                    "user_id": user_id,
                                    "user_name": user_name,
                                    "guild_id": guild_id,
                                    "elapsed_ms": elapsed_ms,
                                }),
                            ),
                            Err(_) => api.send_bot_log_with_details(
                                "error",
                                &format!("Commande PANIC : {full_cmd}"),
                                serde_json::json!({
                                    "event_type": "command.error",
                                    "command": full_cmd,
                                    "module": module,
                                    "user_id": user_id,
                                    "user_name": user_name,
                                    "guild_id": guild_id,
                                    "elapsed_ms": elapsed_ms,
                                    "kind": "panic",
                                }),
                            ),
                        }
                    }
                }
            }
            Interaction::Component(component) => {
                let cid = component.data.custom_id.as_str();
                if modules::announcements::handles_component(cid) {
                    modules::announcements::on_component(&ctx, &component).await;
                } else if modules::confessions::handles_component(cid) {
                    modules::confessions::on_component(&ctx, &component).await;
                } else if modules::welcome::handles_component(cid) {
                    modules::welcome::on_component(&ctx, &component).await;
                } else if modules::games::handles_component(cid) {
                    modules::games::on_component(&ctx, &component).await;
                } else if modules::game_portal::handles_component(cid) {
                    modules::game_portal::on_component(&ctx, &component).await;
                } else if modules::influence::handles_component(cid) {
                    modules::influence::on_component(&ctx, &component).await;
                } else if modules::community::handles_component(cid) {
                    modules::community::on_component(&ctx, &component).await;
                } else if modules::blackjack::handles_component(cid) {
                    modules::blackjack::on_component(&ctx, &component).await;
                } else if modules::slot::handles_component(cid) {
                    modules::slot::on_component(&ctx, &component).await;
                } else if modules::tamagotchi::handles_component(cid) {
                    modules::tamagotchi::on_component(&ctx, &component).await;
                } else if modules::wheel::handles_component(cid) {
                    modules::wheel::on_component(&ctx, &component).await;
                } else if modules::security::handles_component(cid) {
                    modules::security::on_component(&ctx, &component).await;
                } else if modules::automod::handles_component(cid) {
                    modules::automod::on_component(&ctx, &component).await;
                } else if modules::moderation::handles_component(cid) {
                    modules::moderation::on_component(&ctx, &component).await;
                } else if modules::rotation::handles_component(cid) {
                    // Boutons cliques en MP (validation admin tournant).
                    modules::rotation::on_component(&ctx, &component).await;
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
                } else if modules::confessions::handles_modal(mcid) {
                    modules::confessions::on_modal(&ctx, &modal).await;
                } else if modules::welcome::handles_modal(mcid) {
                    modules::welcome::on_modal(&ctx, &modal).await;
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
