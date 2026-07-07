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
pub mod restore;

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
            ),
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
        "restore" => cmd_restore_prompt(ctx, command, opt_string(opts, "id")).await,
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

async fn cmd_restore_prompt(ctx: &Context, command: &CommandInteraction, id: Option<String>) {
    let Some(id) = id else {
        return reply(ctx, command, "ID manquant.").await;
    };
    let confirm = CreateButton::new(format!("{CONFIRM_PREFIX}{id}"))
        .label("Confirmer la restauration")
        .style(ButtonStyle::Danger);
    let cancel = CreateButton::new(CANCEL_ID)
        .label("Annuler")
        .style(ButtonStyle::Secondary);
    let _ = command
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!(
                        "⚠️ **Restauration de `{id}`**\n\nCette action va **recreer** roles, \
                         salons et reglages sur ce serveur (elle n'efface PAS l'existant : \
                         nettoie manuellement avant si besoin). Confirme pour continuer."
                    ))
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
    let Some(snapshot_id) = cid.strip_prefix(CONFIRM_PREFIX) else {
        return;
    };
    let snapshot_id = snapshot_id.to_string();

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

    let progress = restore::Progress::new(ctx, component);
    let report = restore::restore(ctx, guild_id, &snapshot, &progress).await;

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
