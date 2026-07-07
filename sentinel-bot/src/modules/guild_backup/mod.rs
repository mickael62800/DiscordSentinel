//! Module Sauvegarde / Restauration de serveur (`guild-backup-bot`).
//!
//! Capture la structure complete d'un serveur Discord (roles, categories,
//! salons + overwrites, reglages, bans, emojis, roles par membre) vers l'API,
//! et la restaure via serenity avec REMAPPING d'IDs.
//!
//! Action TRES puissante (destructive/massive a la restauration) : reservee au
//! **proprietaire du serveur** (owner) ou a un membre Administrateur. La
//! restauration exige une CONFIRMATION par bouton.

pub mod api_client;
pub mod capture;
pub mod events;
pub mod guild_config;
pub mod progress;
pub mod restore;
pub mod wipe;

pub use events::spawn;

use std::sync::Arc;

use serenity::all::{
    ButtonStyle, CommandDataOption, CommandDataOptionValue, CommandInteraction, CommandOptionType,
    ComponentInteraction, Context, CreateActionRow, CreateButton, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage, GuildId,
    Permissions,
};
use tracing::warn;

use crate::shared::api_client::BaseApiClient;
use crate::shared::heartbeat::ApiClientKey;

const CONFIRM_PREFIX: &str = "gbackup:confirm:";
const CANCEL_ID: &str = "gbackup:cancel";
/// Suffixes du `custom_id` de confirmation encodant le mode wipe. Le custom_id
/// prend la forme `gbackup:confirm:<id>:wipe` ou `...:nowipe` afin que le
/// handler de bouton sache s'il doit d'abord vider le serveur.
const WIPE_SUFFIX: &str = ":wipe";
const NOWIPE_SUFFIX: &str = ":nowipe";

// ── Interface module ──

pub fn register_commands() -> Vec<CreateCommand> {
    vec![CreateCommand::new("backup")
        .description("Sauvegarde / restauration de la structure du serveur (owner)")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "create",
                "Capture la structure actuelle du serveur",
            )
            .add_sub_option(CreateCommandOption::new(
                CommandOptionType::String,
                "label",
                "Libelle de la sauvegarde (optionnel)",
            )),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "list",
            "Liste les sauvegardes du serveur",
        ))
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "restore",
                "Restaure une sauvegarde (recree roles/salons/reglages)",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "id",
                    "ID de la sauvegarde a restaurer",
                )
                .required(true),
            )
            .add_sub_option(CreateCommandOption::new(
                CommandOptionType::Boolean,
                "wipe",
                "⚠️ DESTRUCTIF : vide le serveur (salons/roles/emojis) AVANT de restaurer",
            )),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "delete",
                "Supprime une sauvegarde",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "id",
                    "ID de la sauvegarde a supprimer",
                )
                .required(true),
            ),
        )]
}

pub fn handles_component(cid: &str) -> bool {
    cid.starts_with(CONFIRM_PREFIX) || cid == CANCEL_ID
}

/// Hook (re)join : si le membre a des roles en attente (persistes lors d'un
/// restore), les lui re-attribue puis purge l'entree (atomique cote API).
///
/// Best-effort : n'impacte pas les autres handlers de join. Aucun effet (aucun
/// appel Discord) si le membre n'a pas d'entree en attente.
pub async fn on_member_add(ctx: &Context, member: &serenity::all::Member) {
    let Some(api) = api(ctx).await else {
        return;
    };
    let guild_id = member.guild_id.to_string();
    let user_id = member.user.id.to_string();

    let role_ids = match api_client::consume_pending_roles(&api, &guild_id, &user_id).await {
        Ok(ids) => ids,
        Err(e) => {
            warn!(error = %e, user = %user_id, "guild_backup: consume pending-roles impossible");
            return;
        }
    };
    if role_ids.is_empty() {
        return; // Cas nominal : rien en attente.
    }

    // Traduit les role_id (chaines) en RoleId serenity.
    let roles: Vec<serenity::all::RoleId> = role_ids
        .iter()
        .filter_map(|r| r.parse::<u64>().ok())
        .map(serenity::all::RoleId::new)
        .collect();
    if roles.is_empty() {
        return;
    }

    // Attribution best-effort : roles disparus / permission manquante -> log.
    match member.add_roles(&ctx.http, &roles).await {
        Ok(()) => tracing::info!(
            guild = %guild_id,
            user = %user_id,
            count = roles.len(),
            "guild_backup: roles re-attribues au retour"
        ),
        Err(e) => {
            warn!(error = %e, user = %user_id, "guild_backup: echec re-attribution roles au retour")
        }
    }
}

// ── Helpers ──

async fn api(ctx: &Context) -> Option<Arc<BaseApiClient>> {
    ctx.data.read().await.get::<ApiClientKey>().cloned()
}

/// RBAC : autorise l'owner du serveur OU un membre Administrateur.
async fn is_owner_or_admin(ctx: &Context, command: &CommandInteraction, guild_id: GuildId) -> bool {
    // Administrateur (perm effective de l'interaction) : suffisant.
    let is_admin = command
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| p.contains(Permissions::ADMINISTRATOR))
        .unwrap_or(false);
    if is_admin {
        return true;
    }
    // Sinon, verifie l'owner_id du serveur.
    match guild_id.to_partial_guild(&ctx.http).await {
        Ok(pg) => pg.owner_id == command.user.id,
        Err(e) => {
            warn!(error = %e, "guild_backup: lecture owner_id impossible");
            false
        }
    }
}

async fn reply(ctx: &Context, command: &CommandInteraction, text: &str) {
    let _ = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(text)
                    .ephemeral(true),
            ),
        )
        .await;
}

fn sub_options(command: &CommandInteraction) -> (&str, &[CommandDataOption]) {
    match command.data.options.first() {
        Some(opt) => match &opt.value {
            CommandDataOptionValue::SubCommand(opts) => (opt.name.as_str(), opts.as_slice()),
            _ => (opt.name.as_str(), &[]),
        },
        None => ("", &[]),
    }
}

fn opt_string(opts: &[CommandDataOption], name: &str) -> Option<String> {
    opts.iter()
        .find(|o| o.name == name)
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.clone()),
            _ => None,
        })
}

fn opt_bool(opts: &[CommandDataOption], name: &str) -> bool {
    opts.iter()
        .find(|o| o.name == name)
        .and_then(|o| match &o.value {
            CommandDataOptionValue::Boolean(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false)
}

// ── Dispatch commande ──

pub async fn handle_command(ctx: &Context, command: &CommandInteraction) {
    if command.data.name != "backup" {
        return;
    }
    let Some(guild_id) = command.guild_id else {
        return reply(
            ctx,
            command,
            "Commande disponible uniquement dans un serveur.",
        )
        .await;
    };
    if !is_owner_or_admin(ctx, command, guild_id).await {
        return reply(
            ctx,
            command,
            "❌ Reserve au proprietaire du serveur (ou a un Administrateur).",
        )
        .await;
    }

    let (sub, opts) = sub_options(command);
    match sub {
        "create" => cmd_create(ctx, command, guild_id, opt_string(opts, "label")).await,
        "list" => cmd_list(ctx, command, guild_id).await,
        "restore" => {
            cmd_restore_prompt(ctx, command, opt_string(opts, "id"), opt_bool(opts, "wipe")).await
        }
        "delete" => cmd_delete(ctx, command, opt_string(opts, "id")).await,
        _ => reply(ctx, command, "Sous-commande inconnue.").await,
    }
}

async fn cmd_create(
    ctx: &Context,
    command: &CommandInteraction,
    guild_id: GuildId,
    label: Option<String>,
) {
    let Some(api) = api(ctx).await else {
        return reply(ctx, command, "Service indisponible.").await;
    };
    // Defer : la capture peut depasser les 3s (HTTP roles/salons/membres).
    if command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
        .is_err()
    {
        return;
    }

    let label = label.unwrap_or_else(|| {
        format!(
            "Sauvegarde du {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M")
        )
    });
    let snapshot = match capture::capture(ctx, guild_id, &label, &command.user.id.to_string()).await
    {
        Ok(s) => s,
        Err(e) => {
            edit(ctx, command, &format!("❌ Capture impossible : {e}")).await;
            return;
        }
    };
    let (roles, cats, chans) = (
        snapshot.roles.len(),
        snapshot.categories.len(),
        snapshot.channels.len(),
    );
    match api_client::store_snapshot(&api, &guild_id.to_string(), &snapshot).await {
        Ok(id) => {
            edit(
                ctx,
                command,
                &format!(
                    "✅ Sauvegarde creee.\n**ID** : `{id}`\n**Label** : {label}\n\
                     {roles} role(s), {cats} categorie(s), {chans} salon(s), \
                     {} ban(s), {} emoji(s).",
                    snapshot.bans.len(),
                    snapshot.emojis.len()
                ),
            )
            .await;
        }
        Err(e) => edit(ctx, command, &format!("❌ Stockage impossible : {e}")).await,
    }
}

async fn cmd_list(ctx: &Context, command: &CommandInteraction, guild_id: GuildId) {
    let Some(api) = api(ctx).await else {
        return reply(ctx, command, "Service indisponible.").await;
    };
    match api_client::list_snapshots(&api, &guild_id.to_string()).await {
        Ok(list) if list.is_empty() => {
            reply(ctx, command, "Aucune sauvegarde pour ce serveur.").await
        }
        Ok(list) => {
            let mut txt = String::from("📦 **Sauvegardes du serveur**\n");
            for s in list.iter().take(20) {
                txt.push_str(&format!(
                    "\n• `{}` — **{}** ({}) — {} roles, {} salons",
                    s.id, s.label, s.created_at, s.role_count, s.channel_count
                ));
            }
            reply(ctx, command, &txt).await;
        }
        Err(e) => reply(ctx, command, &format!("❌ {e}")).await,
    }
}

async fn cmd_restore_prompt(
    ctx: &Context,
    command: &CommandInteraction,
    id: Option<String>,
    wipe: bool,
) {
    let Some(id) = id else {
        return reply(ctx, command, "ID manquant.").await;
    };
    // Le flag wipe est encode dans le custom_id du bouton (survit a l'aller-retour).
    let suffix = if wipe { WIPE_SUFFIX } else { NOWIPE_SUFFIX };
    let confirm = CreateButton::new(format!("{CONFIRM_PREFIX}{id}{suffix}"))
        .label(if wipe {
            "⚠️ VIDER puis restaurer"
        } else {
            "Confirmer la restauration"
        })
        .style(ButtonStyle::Danger);
    let cancel = CreateButton::new(CANCEL_ID)
        .label("Annuler")
        .style(ButtonStyle::Secondary);
    // Confirmation RENFORCEE en mode wipe : le message est explicite et alarmant.
    let content = if wipe {
        format!(
            "⚠️ **DESTRUCTIF — Restauration de `{id}` avec WIPE**\n\nCeci va **SUPPRIMER \
             tous les salons, roles et emojis actuels** du serveur AVANT de restaurer le \
             snapshot, pour repartir d'un serveur vierge.\n\n**Action irreversible.** Les \
             bans existants ne sont pas touches. Confirme uniquement si tu es certain."
        )
    } else {
        format!(
            "⚠️ **Restauration de `{id}`**\n\nCette action va **recreer** roles, \
             salons et reglages sur ce serveur (elle n'efface PAS l'existant : \
             nettoie manuellement avant si besoin). Confirme pour continuer."
        )
    };
    let _ = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true)
                    .components(vec![CreateActionRow::Buttons(vec![confirm, cancel])]),
            ),
        )
        .await;
}

async fn cmd_delete(ctx: &Context, command: &CommandInteraction, id: Option<String>) {
    let Some(id) = id else {
        return reply(ctx, command, "ID manquant.").await;
    };
    let Some(api) = api(ctx).await else {
        return reply(ctx, command, "Service indisponible.").await;
    };
    match api_client::delete_snapshot(&api, &id).await {
        Ok(()) => reply(ctx, command, &format!("🗑️ Sauvegarde `{id}` supprimee.")).await,
        Err(e) => reply(ctx, command, &format!("❌ {e}")).await,
    }
}

/// Edite la reponse deferree (ephemere) d'une commande.
async fn edit(ctx: &Context, command: &CommandInteraction, text: &str) {
    let _ = command
        .edit_response(
            &ctx.http,
            serenity::all::EditInteractionResponse::new().content(text),
        )
        .await;
}

// ── Dispatch composants (boutons de confirmation) ──

pub async fn on_component(ctx: &Context, component: &ComponentInteraction) {
    let cid = component.data.custom_id.clone();
    if cid == CANCEL_ID {
        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content("Restauration annulee.")
                        .components(vec![]),
                ),
            )
            .await;
        return;
    }
    let Some(rest) = cid.strip_prefix(CONFIRM_PREFIX) else {
        return;
    };
    // Decode le flag wipe encode en suffixe du custom_id.
    let (snapshot_id, wipe) = if let Some(id) = rest.strip_suffix(WIPE_SUFFIX) {
        (id.to_string(), true)
    } else if let Some(id) = rest.strip_suffix(NOWIPE_SUFFIX) {
        (id.to_string(), false)
    } else {
        // Retrocompat : ancien custom_id sans suffixe -> pas de wipe.
        (rest.to_string(), false)
    };

    let Some(guild_id) = component.guild_id else {
        return;
    };

    // Re-verifie l'autorisation (owner / admin) au moment du clic.
    let is_admin = component
        .member
        .as_ref()
        .map(|m| {
            m.permissions
                .map(|p| p.contains(Permissions::ADMINISTRATOR))
                .unwrap_or(false)
        })
        .unwrap_or(false);
    let allowed = if is_admin {
        true
    } else {
        match guild_id.to_partial_guild(&ctx.http).await {
            Ok(pg) => pg.owner_id == component.user.id,
            Err(_) => false,
        }
    };
    if !allowed {
        let _ = component
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("❌ Reserve au proprietaire du serveur.")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    let Some(api) = api(ctx).await else {
        return;
    };

    // Passe le message de confirmation en mode "en cours" (retire les boutons).
    if component
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .content("♻️ Restauration en cours…")
                    .components(vec![]),
            ),
        )
        .await
        .is_err()
    {
        return;
    }

    let snapshot = match api_client::get_snapshot(&api, &snapshot_id).await {
        Ok(s) => s,
        Err(e) => {
            let _ = component
                .edit_response(
                    &ctx.http,
                    serenity::all::EditInteractionResponse::new()
                        .content(format!("❌ Sauvegarde introuvable : {e}")),
                )
                .await;
            return;
        }
    };

    // Repart propre : purge d'eventuelles re-attributions en attente d'un
    // restore precedent (best-effort, ne bloque pas la restauration).
    let gid = guild_id.to_string();
    if let Err(e) = api_client::clear_pending_roles(&api, &gid).await {
        warn!(error = %e, "guild_backup: purge des pending-roles impossible");
    }

    let progress = progress::ProgressSink::interaction(ctx, component);

    // Phase de WIPE optionnelle : vide le serveur avant recreation.
    let wipe_report = if wipe {
        Some(wipe::wipe(ctx, guild_id, &progress).await)
    } else {
        None
    };

    let report = restore::restore(ctx, guild_id, &snapshot, &progress).await;

    // Persiste les re-attributions pour TOUS les membres (les absents seront
    // re-rolises a leur retour via le hook de join).
    if !report.pending_grants.is_empty() {
        match api_client::save_pending_roles(&api, &gid, &report.pending_grants).await {
            Ok(n) => {
                tracing::info!(guild = %gid, saved = n, "guild_backup: pending-roles enregistres")
            }
            Err(e) => {
                warn!(error = %e, "guild_backup: enregistrement des pending-roles impossible")
            }
        }
    }

    let mut txt = format!(
        "✅ **Restauration terminee**\n{} role(s) créé(s) ({} echec), {} categorie(s), \
         {} salon(s) ({} echec), {} ban(s), {} membre(s) re-rolises.",
        report.roles_created,
        report.roles_failed,
        report.categories_created,
        report.channels_created,
        report.channels_failed,
        report.bans_applied,
        report.members_updated,
    );
    if let Some(w) = wipe_report {
        txt.push_str(&format!(
            "\n🧨 Wipe : {} salon(s) / {} role(s) / {} emoji(s) supprimes.",
            w.channels_deleted, w.roles_deleted, w.emojis_deleted
        ));
    }
    if report.emojis_total > 0 {
        txt.push_str(&format!(
            "\nEmojis : {}/{}.",
            report.emojis_created, report.emojis_total
        ));
    }
    if let Some(ok) = report.icon_restored {
        txt.push_str(&format!(
            "\nIcone : {}.",
            if ok { "ok" } else { "echec" }
        ));
    }
    if !report.notes.is_empty() {
        txt.push_str("\n\n⚠️ Notes : ");
        txt.push_str(&report.notes.join(" ; "));
    }
    let _ = component
        .edit_response(
            &ctx.http,
            serenity::all::EditInteractionResponse::new().content(txt),
        )
        .await;
}
